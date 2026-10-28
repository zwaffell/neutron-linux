mod instrument;
mod midi;
mod protocol;
use eframe::egui;
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    time::{Duration, Instant},
};
struct Neutron {
    spec: protocol::Spec,
    values: BTreeMap<String, Value>,
    connection: Option<midi::Connection>,
    ports: midi::Ports,
    input: String,
    output: String,
    live: bool,
    ready: bool,
    got_settings: bool,
    status: String,
    last_poll: Instant,
    last_response: Instant,
    log: VecDeque<String>,
    frequency: [Option<f64>; 2],
    note: Option<u8>,
    restore_confirm: bool,
    screenshot: Option<std::path::PathBuf>,
    screenshot_requested: bool,
    artwork: std::collections::HashMap<&'static str, egui::TextureHandle>,
}
impl Default for Neutron {
    fn default() -> Self {
        let spec = protocol::spec();
        let values = protocol::defaults(&spec);
        Self {
            spec,
            values,
            connection: None,
            ports: midi::Ports {
                inputs: vec![],
                outputs: vec![],
            },
            input: String::new(),
            output: String::new(),
            live: false,
            ready: false,
            got_settings: false,
            status: "Offline preview — no MIDI devices opened".into(),
            last_poll: Instant::now(),
            last_response: Instant::now(),
            log: VecDeque::new(),
            frequency: [None, None],
            note: None,
            restore_confirm: false,
            screenshot: None,
            screenshot_requested: false,
            artwork: Default::default(),
        }
    }
}
impl Neutron {
    fn record(&mut self, label: &str, bytes: &[u8]) {
        if self.log.len() == 120 {
            self.log.pop_front();
        }
        self.log.push_back(format!(
            "{label} {}",
            bytes
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    fn send(&mut self, cmd: u8, data: &[u8]) -> Result<(), String> {
        let bytes = protocol::packet(cmd, data)?;
        self.record(if self.live { "OUT" } else { "PREVIEW" }, &bytes);
        if self.live {
            self.connection
                .as_mut()
                .ok_or("Not connected")?
                .output
                .send(&bytes)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn disconnect(&mut self) {
        self.connection = None;
        self.restore_confirm = false;
        self.ready = false;
        self.got_settings = false;
        self.frequency = [None, None];
        self.note = None;
        self.status = "Disconnected — choose Offline preview or reconnect".into();
    }
    fn connect(&mut self) {
        self.disconnect();
        self.live = true;
        match midi::connect(&self.input, &self.output) {
            Ok(c) => {
                self.connection = Some(c);
                self.status = "Checking firmware…".into();
                self.last_response = Instant::now();
                self.last_poll = Instant::now();
                if let Err(e) = self.send(115, &[]) {
                    self.disconnect();
                    self.status = e;
                }
            }
            Err(e) => self.status = e,
        }
    }
    fn refresh(&mut self) {
        match midi::ports() {
            Ok(p) => {
                if !p.inputs.contains(&self.input) {
                    self.input = p
                        .inputs
                        .iter()
                        .find(|s| s.to_lowercase().contains("neutron"))
                        .or(p.inputs.first())
                        .cloned()
                        .unwrap_or_default();
                }
                if !p.outputs.contains(&self.output) {
                    self.output = p
                        .outputs
                        .iter()
                        .find(|s| s.to_lowercase().contains("neutron"))
                        .or(p.outputs.first())
                        .cloned()
                        .unwrap_or_default();
                }
                self.ports = p;
                self.status =
                    "Choose the input and output for the same Neutron, then Connect".into();
            }
            Err(e) => self.status = format!("MIDI unavailable: {e}"),
        }
    }
    fn receive(&mut self, bytes: &[u8]) {
        let Ok((cmd, data)) = protocol::response(bytes) else {
            return;
        };
        self.record("IN", bytes);
        let result: Result<(), String> = (|| {
            match cmd {
                116 => {
                    let text = std::str::from_utf8(data).map_err(|_| "Invalid firmware text")?;
                    let (version, compatible) = protocol::firmware(text)?;
                    self.ready = compatible;
                    self.status = format!(
                        "Firmware {}.{}.{}{}",
                        version[0],
                        version[1],
                        version[2],
                        if compatible {
                            " — reading settings"
                        } else {
                            " — requires 2.0.2 or later"
                        }
                    );
                    if compatible {
                        self.send(5, &[])?;
                    }
                }
                6 if self.ready => {
                    self.values = protocol::settings(&self.spec, data)?;
                    self.got_settings = true;
                    self.status = "Connected — edits send MIDI to the selected synth".into();
                }
                90 if self.ready => {
                    self.send(5, &[])?;
                }
                114 if self.ready => {
                    let raw = protocol::decode(data)?;
                    if raw.len() < 14 {
                        return Err("Truncated tuning data".into());
                    }
                    for i in 0..2 {
                        let period = u32::from_le_bytes(raw[i * 7..i * 7 + 4].try_into().unwrap());
                        self.frequency[i] = (period != 0).then(|| 1_000_000.0 / f64::from(period));
                    }
                    self.note = Some(raw[4]);
                }
                _ => {}
            }
            Ok(())
        })();
        match result {
            Ok(()) => self.last_response = Instant::now(),
            Err(e) => self.status = e,
        }
    }
    fn tick(&mut self) {
        let messages: Vec<_> = self
            .connection
            .as_ref()
            .map(|c| c.receive.try_iter().take(64).collect())
            .unwrap_or_default();
        for m in messages {
            self.receive(&m);
        }
        if self.connection.is_some() && self.last_response.elapsed() > Duration::from_secs(6) {
            self.disconnect();
            self.status = "No valid response for 6 seconds — disconnected".into();
        }
        if self.connection.is_some() && self.last_poll.elapsed() > Duration::from_secs(1) {
            self.last_poll = Instant::now();
            let cmd = if self.ready { 5 } else { 115 };
            if let Err(e) = self.send(cmd, &[]) {
                self.disconnect();
                self.status = e;
            }
        }
    }
    fn edit(&mut self, name: &str, value: Value) {
        match protocol::writes(&self.spec, name, &value) {
            Ok(packets) => {
                for packet in packets {
                    self.record(if self.live { "OUT" } else { "PREVIEW" }, &packet);
                    if self.live {
                        if let Some(c) = self.connection.as_mut() {
                            if let Err(e) = c.output.send(&packet) {
                                self.status = e.to_string();
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                }
                self.values.insert(name.to_string(), value);
            }
            Err(e) => self.status = e,
        }
    }
    fn action(&mut self, name: &str) {
        let address = self.spec.parameters[name].address;
        if let Err(e) = self.send(10, &[address, 0]) {
            self.status = e;
            return;
        }
        if !self.live {
            match name {
                "RESET_LFO_ORDER" => {
                    self.values
                        .insert("LFO_SHAPE_ORDER".into(), serde_json::json!([0, 1, 2, 3, 4]));
                }
                "RESET_KEY_RANGE" => {
                    self.values.insert("KEY_RANGE_MIN".into(), Value::from(24));
                    self.values.insert("KEY_RANGE_MAX".into(), Value::from(96));
                }
                _ => {}
            }
        } else if let Err(e) = self.send(5, &[]) {
            self.status = e;
        }
    }
    fn draw(&mut self, ui: &mut egui::Ui) {
        self.tick();
        if let Some(path) = self.screenshot.clone() {
            let screenshot = ui.ctx().input(|i| {
                i.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = screenshot {
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let file = std::fs::File::create(path)?;
                    let mut encoder =
                        png::Encoder::new(file, image.size[0] as u32, image.size[1] as u32);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    encoder.write_header()?.write_image_data(&bytes)?;
                    Ok(())
                })();
                if let Err(error) = result {
                    eprintln!("Screenshot failed: {error}");
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            } else if !self.screenshot_requested {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.screenshot_requested = true;
            }
        }
        ui.ctx().request_repaint_after(Duration::from_millis(100));
        self.instrument_ui(ui);
        if self.restore_confirm {
            egui::Window::new("Restore default settings?")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(if self.live {
                        "This changes the connected synth's settings."
                    } else {
                        "This resets the offline preview."
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Restore").clicked() {
                            match self.send(11, &[]) {
                                Ok(()) => {
                                    if self.live {
                                        if let Err(e) = self.send(5, &[]) {
                                            self.status = e;
                                        }
                                    } else {
                                        self.values = protocol::defaults(&self.spec);
                                    }
                                }
                                Err(e) => self.status = e,
                            }
                            self.restore_confirm = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.restore_confirm = false;
                        }
                    });
                });
        }
    }
}
impl eframe::App for Neutron {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.draw(ui)
    }
}
fn note_name(n: i32) -> String {
    let names = [
        "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
    ];
    format!(
        "{}{}",
        names[n.rem_euclid(12) as usize],
        n.div_euclid(12) - 2
    )
}
fn main() -> eframe::Result {
    if std::env::args().any(|a| a == "--smoke-test") {
        let mut app = Neutron::default();
        assert_eq!(
            protocol::settings(&app.spec, &protocol::initial_settings(&app.spec)).unwrap(),
            app.values
        );
        let ctx = egui::Context::default();
        instrument::configure(&ctx);
        let output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw(ui));
        assert!(!output.shapes.is_empty());
        assert!(app.connection.is_none());
        println!(
            "Native Rust UI smoke test passed; {} editable parameters; no MIDI opened",
            app.values.len()
        );
        return Ok(());
    }
    if std::env::args().any(|a| a == "--help") {
        println!(
            "Neutron Linux: native Rust synth controller\nStarts in offline preview. Select Real synth, refresh ports, then Connect.\n--smoke-test  Render UI without a window or MIDI access"
        );
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 1040.0])
            .with_min_inner_size([520.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Neutron Linux",
        options,
        Box::new(|cc| {
            instrument::configure(&cc.egui_ctx);
            let mut app = Neutron::default();
            let args: Vec<String> = std::env::args().collect();
            if let Some(i) = args.iter().position(|a| a == "--screenshot") {
                app.screenshot = args.get(i + 1).map(std::path::PathBuf::from);
            }
            Ok(Box::new(app))
        }),
    )
}
#[cfg(test)]
mod app_tests {
    use super::*;
    #[test]
    fn offline_edits_never_open_midi() {
        let mut app = Neutron::default();
        app.edit("OSC_SYNC", Value::Bool(true));
        assert_eq!(app.values["OSC_SYNC"], true);
        assert!(app.connection.is_none());
        assert!(app.log[0].ends_with("0A 0E 01 F7"));
    }
    #[test]
    fn live_requires_handshake() {
        let mut app = Neutron {
            live: true,
            ..Default::default()
        };
        assert!(!app.ready);
        let mut b = protocol::HEADER.to_vec();
        b.extend([116, 1]);
        b.extend(b"2.0.1");
        b.push(247);
        app.receive(&b);
        assert!(!app.ready);
        assert!(!app.got_settings);
    }
    #[test]
    fn preview_reset() {
        let mut app = Neutron::default();
        app.values.insert("KEY_RANGE_MIN".into(), Value::from(40));
        app.action("RESET_KEY_RANGE");
        assert_eq!(app.values["KEY_RANGE_MIN"], 24);
        assert_eq!(app.values["KEY_RANGE_MAX"], 96);
    }
    #[test]
    fn settings_and_tuning_responses() {
        let mut app = Neutron {
            ready: true,
            ..Default::default()
        };
        let mut settings = protocol::HEADER.to_vec();
        settings.extend([6, 1]);
        settings.extend(protocol::initial_settings(&app.spec));
        settings.push(247);
        app.receive(&settings);
        assert!(app.got_settings);
        assert_eq!(app.values["KEY_RANGE_MIN"], 24);
        let mut raw = Vec::new();
        raw.extend(2000u32.to_le_bytes());
        raw.extend([69, 0, 0]);
        raw.extend(4000u32.to_le_bytes());
        raw.extend([69, 0, 0]);
        let mut tuning = protocol::HEADER.to_vec();
        tuning.extend([114, 1]);
        tuning.extend(protocol::encode(&raw));
        tuning.push(247);
        app.receive(&tuning);
        assert_eq!(app.frequency, [Some(500.0), Some(250.0)]);
        assert_eq!(app.note, Some(69));
    }
}
