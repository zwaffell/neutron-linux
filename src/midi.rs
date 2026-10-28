use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::sync::mpsc::{self, Receiver};
pub struct Ports {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}
pub fn ports() -> Result<Ports, String> {
    let input = MidiInput::new("Neutron discovery").map_err(|e| e.to_string())?;
    let output = MidiOutput::new("Neutron discovery").map_err(|e| e.to_string())?;
    Ok(Ports {
        inputs: input
            .ports()
            .iter()
            .map(|p| input.port_name(p).unwrap_or_default())
            .collect(),
        outputs: output
            .ports()
            .iter()
            .map(|p| output.port_name(p).unwrap_or_default())
            .collect(),
    })
}
pub struct Connection {
    pub _input: MidiInputConnection<()>,
    pub output: MidiOutputConnection,
    pub receive: Receiver<Vec<u8>>,
}
pub fn connect(input_name: &str, output_name: &str) -> Result<Connection, String> {
    let mut input = MidiInput::new("Neutron Linux input").map_err(|e| e.to_string())?;
    input.ignore(Ignore::None);
    let output = MidiOutput::new("Neutron Linux output").map_err(|e| e.to_string())?;
    let ip = input
        .ports()
        .into_iter()
        .find(|p| input.port_name(p).ok().as_deref() == Some(input_name))
        .ok_or("Input disappeared; refresh ports")?;
    let op = output
        .ports()
        .into_iter()
        .find(|p| output.port_name(p).ok().as_deref() == Some(output_name))
        .ok_or("Output disappeared; refresh ports")?;
    let (tx, rx) = mpsc::sync_channel(256);
    let ic = input
        .connect(
            &ip,
            "Neutron receive",
            move |_, data, _| {
                if data.len() <= 4096 {
                    let _ = tx.try_send(data.to_vec());
                }
            },
            (),
        )
        .map_err(|e| e.to_string())?;
    let oc = output
        .connect(&op, "Neutron send")
        .map_err(|e| e.to_string())?;
    Ok(Connection {
        _input: ic,
        output: oc,
        receive: rx,
    })
}
