use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub const HEADER: [u8; 6] = [0xf0, 0, 0x20, 0x32, 0x28, 0x7f];
#[derive(Clone, Deserialize)]
pub struct Field {
    pub block: usize,
    pub offset: u32,
    pub length: u32,
}
#[derive(Clone, Deserialize)]
pub struct Parameter {
    pub address: u8,
    pub settings: Option<Field>,
    pub default: Option<Value>,
}
#[derive(Deserialize)]
pub struct Spec {
    pub parameters: BTreeMap<String, Parameter>,
}
pub fn spec() -> Spec {
    serde_json::from_str(include_str!("spec.json")).expect("embedded protocol specification")
}
pub fn defaults(spec: &Spec) -> BTreeMap<String, Value> {
    spec.parameters
        .iter()
        .filter_map(|(k, p)| p.default.clone().map(|v| (k.clone(), v)))
        .collect()
}
pub fn packet(command: u8, data: &[u8]) -> Result<Vec<u8>, String> {
    if command > 127 || data.iter().any(|v| *v > 127) {
        return Err("SysEx data must be 7-bit".into());
    }
    let mut bytes = HEADER.to_vec();
    bytes.push(command);
    bytes.extend(data);
    bytes.push(247);
    Ok(bytes)
}
pub fn decode(data: &[u8]) -> Result<Vec<u8>, String> {
    if !data.len().is_multiple_of(8) || data.iter().any(|v| *v > 127) {
        return Err("Invalid SysEx binary packing".into());
    }
    Ok(data
        .chunks_exact(8)
        .flat_map(|g| (0..7).map(move |i| g[i] | (((g[7] >> i) & 1) << 7)))
        .collect())
}
pub fn encode(data: &[u8]) -> Vec<u8> {
    let mut raw = data.to_vec();
    raw.resize(data.len() + 7 - data.len() % 7, 0);
    raw.chunks_exact(7)
        .flat_map(|g| {
            let mut out = g.iter().map(|x| x & 127).collect::<Vec<_>>();
            out.push(
                g.iter()
                    .enumerate()
                    .fold(0, |v, (i, b)| v | ((b >> 7) << i)),
            );
            out
        })
        .collect()
}
pub fn response(bytes: &[u8]) -> Result<(u8, &[u8]), String> {
    if bytes.len() < 9
        || bytes[..5] != HEADER[..5]
        || bytes.last() != Some(&247)
        || bytes[1..bytes.len() - 1].iter().any(|v| *v > 127)
    {
        return Err("Not a valid Neutron response".into());
    }
    Ok((bytes[6], &bytes[8..bytes.len() - 1]))
}
pub fn firmware(text: &str) -> Result<([u32; 3], bool), String> {
    let clean: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let parts: Vec<_> = clean.split('.').collect();
    let strings = match parts.as_slice() {
        [a, b, c] => vec![a.to_string(), b.to_string(), c.to_string()],
        [a, b] if b.len() == 2 => vec![a.to_string(), b[..1].into(), b[1..].into()],
        _ => return Err("Unrecognized firmware version".into()),
    };
    let nums: Vec<u32> = strings
        .iter()
        .map(|v| {
            v.parse()
                .map_err(|_| "Invalid firmware version".to_string())
        })
        .collect::<Result<_, _>>()?;
    let v = [nums[0], nums[1], nums[2]];
    Ok((v, v[0] == 0 || v >= [2, 0, 2]))
}
pub fn settings(spec: &Spec, data: &[u8]) -> Result<BTreeMap<String, Value>, String> {
    let raw = decode(data)?;
    if raw.len() < 20 {
        return Err("Truncated settings response".into());
    }
    let words: Vec<u16> = raw
        .chunks_exact(2)
        .map(|x| u16::from_le_bytes([x[0], x[1]]))
        .collect();
    let mut state = defaults(spec);
    for (name, p) in &spec.parameters {
        let (Some(f), Some(initial)) = (&p.settings, &p.default) else {
            continue;
        };
        let raw = (u32::from(words[f.block]) >> f.offset) & ((1 << f.length) - 1);
        let value = if let Some(arr) = initial.as_array() {
            let bits = f.length / arr.len() as u32;
            Value::Array(
                (0..arr.len())
                    .map(|i| Value::from((raw >> (bits * i as u32)) & ((1 << bits) - 1)))
                    .collect(),
            )
        } else if initial.is_boolean() {
            Value::Bool(raw == 1)
        } else {
            Value::from(match name.as_str() {
                "OSC_KEY_SPLIT" if raw != 0 => raw + 23,
                "KEY_RANGE_MIN" => raw + 24,
                "KEY_RANGE_MAX" => 96 - raw,
                _ => raw,
            })
        };
        state.insert(name.clone(), value);
    }
    Ok(state)
}
pub fn initial_settings(spec: &Spec) -> Vec<u8> {
    let mut words = [0u16; 10];
    for (name, p) in &spec.parameters {
        let (Some(f), Some(v)) = (&p.settings, &p.default) else {
            continue;
        };
        let mut raw = if let Some(a) = v.as_array() {
            let bits = f.length / a.len() as u32;
            a.iter().enumerate().fold(0, |r, (i, n)| {
                r | ((n.as_u64().unwrap() as u32) << (bits * i as u32))
            })
        } else if let Some(b) = v.as_bool() {
            u32::from(b)
        } else {
            v.as_u64().unwrap() as u32
        };
        raw = match name.as_str() {
            "OSC_KEY_SPLIT" if raw != 0 => raw - 23,
            "KEY_RANGE_MIN" => raw - 24,
            "KEY_RANGE_MAX" => 96 - raw,
            _ => raw,
        };
        words[f.block] |= ((raw & ((1 << f.length) - 1)) << f.offset) as u16;
    }
    encode(
        &words
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<_>>(),
    )
}
pub fn bounds(name: &str) -> (u8, u8) {
    match name {
        "MIDI_CHANNEL" | "DEVICE_ID" => (0, 15),
        "NOTE_PRIORITY" => (0, 2),
        "PARA_NOTE_PRIORITY" => (0, 3),
        "PITCH_BEND_RANGE" => (0, 24),
        "ASSIGN_OUT" => (0, 4),
        "KEY_RANGE_MIN" | "KEY_RANGE_MAX" => (24, 96),
        "OSC1_RANGE" | "OSC2_RANGE" | "VCF_MOD_SRC" => (0, 3),
        "OSC1_AUTOGLIDE" | "OSC2_AUTOGLIDE" => (0, 24),
        "OSC_KEY_SPLIT" => (0, 86),
        "VCF_MODE" => (0, 2),
        "LFO_KEY_TRACK" => (0, 108),
        _ => (0, 63),
    }
}
pub fn writes(spec: &Spec, name: &str, value: &Value) -> Result<Vec<Vec<u8>>, String> {
    let p = spec.parameters.get(name).ok_or("Unknown parameter")?;
    let initial = p.default.as_ref().ok_or("Action is not a value")?;
    if initial.is_boolean() {
        return Ok(vec![packet(
            10,
            &[
                p.address,
                u8::from(value.as_bool().ok_or("Expected boolean")?),
            ],
        )?]);
    }
    if let Some(original) = initial.as_array() {
        let arr = value.as_array().ok_or("Expected array")?;
        if arr.len() != original.len() {
            return Err("Incorrect array length".into());
        }
        let max = if name == "LFO_SHAPE_ORDER" { 4 } else { 7 };
        return arr
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let n = x.as_u64().ok_or("Expected unsigned value")?;
                if n > max {
                    return Err("Array element out of range".into());
                }
                packet(10, &[p.address, i as u8, n as u8])
            })
            .collect();
    }
    let n = value.as_u64().ok_or("Expected unsigned value")?;
    let (min, max) = bounds(name);
    if n < u64::from(min)
        || n > u64::from(max)
        || ((name == "OSC_KEY_SPLIT" && n != 0 && n < 24)
            || (name == "LFO_KEY_TRACK" && n != 0 && n < 12))
    {
        return Err("Value out of range".into());
    }
    Ok(vec![packet(10, &[p.address, n as u8])?])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packets_match_recovered_ui() {
        let s = spec();
        assert_eq!(
            writes(&s, "OSC_SYNC", &Value::Bool(true)).unwrap()[0],
            vec![240, 0, 32, 50, 40, 127, 10, 14, 1, 247]
        );
        assert_eq!(
            writes(&s, "OSC1_RANGE", &Value::from(2)).unwrap()[0],
            vec![240, 0, 32, 50, 40, 127, 10, 38, 2, 247]
        );
    }
    #[test]
    fn high_bits_and_padding() {
        let raw = [0, 127, 128, 255, 1, 2, 3];
        assert_eq!(
            encode(&raw),
            vec![0, 127, 0, 127, 1, 2, 3, 12, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(&decode(&encode(&raw)).unwrap()[..7], &raw);
        for len in 0..100 {
            let v: Vec<_> = (0..len).map(|x| (x * 73) as u8).collect();
            assert_eq!(&decode(&encode(&v)).unwrap()[..len], v);
        }
    }
    #[test]
    fn initial_global_settings() {
        let s = spec();
        assert_eq!(settings(&s, &initial_settings(&s)).unwrap(), defaults(&s));
    }
    #[test]
    fn malformed_packets_rejected() {
        assert!(decode(&[128; 8]).is_err());
        assert!(decode(&[0; 7]).is_err());
        assert!(response(&[]).is_err());
        assert!(settings(&spec(), &[0; 8]).is_err());
        assert!(packet(10, &[128]).is_err());
    }
    #[test]
    fn firmware_gate() {
        assert!(firmware("V2.0.2").unwrap().1);
        assert!(firmware("2.02").unwrap().1);
        assert!(!firmware("2.0.1").unwrap().1);
        assert!(firmware("0.1.0").unwrap().1);
        assert!(firmware("").is_err());
    }
    #[test]
    fn reject_invalid_controls() {
        let s = spec();
        assert!(writes(&s, "MIDI_CHANNEL", &Value::from(16)).is_err());
        assert!(writes(&s, "OSC_SYNC", &Value::from(1)).is_err());
        assert!(writes(&s, "OSC_KEY_SPLIT", &Value::from(1)).is_err());
    }
    #[test]
    fn array_messages() {
        let s = spec();
        let p = writes(&s, "LFO_SHAPE_ORDER", &serde_json::json!([4, 3, 2, 1, 0])).unwrap();
        assert_eq!(p.len(), 5);
        assert_eq!(p[0], vec![240, 0, 32, 50, 40, 127, 10, 56, 0, 4, 247]);
    }
    #[test]
    fn transformed_settings_fixture() {
        let s = spec();
        let mut words = [0u16; 10];
        words[4] = 1 << 10;
        words[6] = 12 | (12 << 6);
        let payload = encode(
            &words
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let state = settings(&s, &payload).unwrap();
        assert_eq!(state["OSC_KEY_SPLIT"], 24);
        assert_eq!(state["KEY_RANGE_MIN"], 36);
        assert_eq!(state["KEY_RANGE_MAX"], 84);
    }
}
