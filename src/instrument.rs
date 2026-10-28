//! Native instrument panel, drawn with egui. Artwork and fonts are embedded.
use super::*;
use egui::{
    Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2,
};
const BG: Color32 = Color32::from_rgb(144, 40, 53);
const WHITE: Color32 = Color32::from_rgb(255, 253, 247);
const DARK: Color32 = Color32::from_rgb(59, 59, 57);

pub fn configure(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Cuprum".into(),
        egui::FontData::from_static(include_bytes!("../assets/Cuprum-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "Cuprum bold".into(),
        egui::FontData::from_static(include_bytes!("../assets/Cuprum-Bold.ttf")).into(),
    );
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "Cuprum".into());
    fonts.families.insert(
        FontFamily::Name("panel-bold".into()),
        vec!["Cuprum bold".into()],
    );
    ctx.set_fonts(fonts);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = BG;
    visuals.override_text_color = Some(WHITE);
    visuals.selection.bg_fill = Color32::from_rgb(183, 72, 86);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = BG;
        widget.weak_bg_fill = BG;
        widget.bg_stroke = Stroke::new(1.5, WHITE);
        widget.fg_stroke = Stroke::new(1.0, WHITE);
    }
    ctx.set_visuals(visuals);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = vec2(8.0, 5.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(15.0));
    });
}
fn texture(ctx: &egui::Context, name: &str, data: &[u8]) -> egui::TextureHandle {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().expect("embedded PNG header");
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut bytes).expect("embedded PNG pixels");
    let raw = &bytes[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => raw.to_vec(),
        png::ColorType::Rgb => raw
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        _ => panic!("Unexpected artwork color format"),
    };
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [info.width as usize, info.height as usize],
            &rgba,
        ),
        egui::TextureOptions::LINEAR,
    )
}
struct Canvas<'a> {
    ui: &'a mut egui::Ui,
    origin: Pos2,
    scale: f32,
    assets: std::collections::HashMap<&'static str, egui::TextureHandle>,
}
impl Canvas<'_> {
    fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(
            self.origin + vec2(x, y) * self.scale,
            vec2(w, h) * self.scale,
        )
    }
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.scale
    }
    fn stroke(&self) -> Stroke {
        Stroke::new(1.8 * self.scale, WHITE)
    }
    fn text(&self, x: f32, y: f32, text: impl Into<String>, size: f32, bold: bool) {
        self.ui.painter().text(
            self.pt(x, y),
            Align2::CENTER_CENTER,
            text.into(),
            FontId::new(
                size * self.scale,
                if bold {
                    FontFamily::Name("panel-bold".into())
                } else {
                    FontFamily::Proportional
                },
            ),
            WHITE,
        );
    }
    fn line(&self, x: f32, y: f32, x2: f32, y2: f32) {
        self.ui.painter().line_segment(
            [self.pt(x, y), self.pt(x2, y2)],
            Stroke::new(self.scale, WHITE),
        );
    }
    fn art(&self, name: &str, x: f32, y: f32, w: f32, h: f32) {
        if let Some(texture) = self.assets.get(name) {
            self.ui.painter().image(
                texture.id(),
                self.rect(x, y, w, h),
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                WHITE,
            );
        }
    }
    fn panel(&self, title: &str, art: &str, x: f32, y: f32, w: f32, h: f32) {
        self.art(art, x, y - 36.0, 166.0, 44.0);
        self.ui
            .painter()
            .rect_filled(self.rect(x, y, w, h), 14.0 * self.scale, BG);
        self.ui.painter().rect_stroke(
            self.rect(x, y, w, h),
            14.0 * self.scale,
            self.stroke(),
            StrokeKind::Inside,
        );
        self.text(x + 66.0, y - 15.0, title, 16.0, true);
    }
    fn title(&self, x: f32, y: f32, w: f32, title: &str) {
        self.text(x + w / 2.0, y + 15.0, title, 13.0, true);
    }
    fn response(
        &self,
        id: impl std::hash::Hash + std::fmt::Debug,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sense: Sense,
    ) -> egui::Response {
        self.ui
            .interact(self.rect(x, y, w, h), egui::Id::new(id), sense)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    }
    fn toggle(
        &mut self,
        id: &str,
        x: f32,
        y: f32,
        w: f32,
        title: &str,
        labels: (&str, &str),
        value: bool,
    ) -> bool {
        self.title(x, y, w, title);
        let r = self.response(
            (id, "toggle"),
            x + w / 2.0 - 36.0,
            y + 28.0,
            72.0,
            30.0,
            Sense::click(),
        );
        let center = self.pt(x + w / 2.0, y + 43.0);
        let track = Rect::from_center_size(center, vec2(44.0, 17.0) * self.scale);
        self.ui.painter().rect(
            track,
            10.0 * self.scale,
            if value { WHITE } else { BG },
            self.stroke(),
            StrokeKind::Inside,
        );
        self.ui.painter().circle(
            center + vec2(if value { 11.0 } else { -11.0 } * self.scale, 0.0),
            11.5 * self.scale,
            BG,
            self.stroke(),
        );
        self.text(x + w / 2.0 - 24.0, y + 67.0, labels.0, 10.0, false);
        self.text(x + w / 2.0 + 24.0, y + 67.0, labels.1, 10.0, false);
        r.clicked()
    }
    fn radios(
        &mut self,
        id: &str,
        x: f32,
        y: f32,
        w: f32,
        title: &str,
        labels: &[&str],
        value: u8,
    ) -> Option<u8> {
        self.title(x, y, w, title);
        let mut selected = None;
        let dx = w / labels.len() as f32;
        for (i, label) in labels.iter().enumerate() {
            let cx = x + dx * (i as f32 + 0.5);
            let r = self.response((id, i), cx - dx / 2.0, y + 28.0, dx, 47.0, Sense::click());
            self.ui
                .painter()
                .circle_stroke(self.pt(cx, y + 43.0), 7.3 * self.scale, self.stroke());
            if value as usize == i {
                self.ui
                    .painter()
                    .circle_filled(self.pt(cx, y + 43.0), 4.2 * self.scale, WHITE);
            }
            self.text(cx, y + 67.0, *label, 9.0, true);
            if r.clicked() {
                selected = Some(i as u8)
            }
        }
        selected
    }
    fn dropdown(
        &mut self,
        id: &str,
        x: f32,
        y: f32,
        w: f32,
        text: &str,
        items: &[(u8, String)],
        value: u8,
    ) -> Option<u8> {
        let rect = self.rect(x, y, w, 28.0);
        let response = self.response((id, "select"), x, y, w, 28.0, Sense::click());
        self.ui
            .painter()
            .rect_stroke(rect, 10.0 * self.scale, self.stroke(), StrokeKind::Inside);
        self.ui
            .painter()
            .with_clip_rect(rect.shrink(5.0 * self.scale))
            .text(
                self.pt(x + (w - 15.0) / 2.0, y + 14.0),
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(14.0 * self.scale),
                WHITE,
            );
        self.line(x + w - 17.0, y + 11.0, x + w - 12.0, y + 16.0);
        self.line(x + w - 12.0, y + 16.0, x + w - 7.0, y + 11.0);
        let mut selected = None;
        egui::Popup::menu(&response)
            .width(w * self.scale)
            .show(|ui| {
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for (n, label) in items {
                            if ui.selectable_label(*n == value, label).clicked() {
                                selected = Some(*n);
                                ui.close();
                            }
                        }
                    });
            });
        selected
    }
    fn button(&mut self, id: &str, x: f32, y: f32, w: f32, label: &str) -> bool {
        let response = self.response((id, "button"), x, y, w, 26.0, Sense::click());
        self.ui.painter().rect(
            self.rect(x, y, w, 26.0),
            9.0 * self.scale,
            if response.hovered() {
                Color32::from_rgb(169, 54, 68)
            } else {
                BG
            },
            self.stroke(),
            StrokeKind::Inside,
        );
        self.text(x + w / 2.0, y + 13.0, label, 12.0, true);
        response.clicked()
    }
    fn slider(&mut self, id: &str, x: f32, y: f32, w: f32, title: &str, value: u8) -> Option<u8> {
        self.title(x, y, w, title);
        let pill = self.rect(x + w / 2.0 - 22.0, y + 29.0, 44.0, 22.0);
        self.ui.painter().rect(
            pill,
            12.0 * self.scale,
            DARK,
            self.stroke(),
            StrokeKind::Inside,
        );
        self.text(
            x + w / 2.0,
            y + 40.0,
            format!("{}%", u32::from(value) * 100 / 63),
            14.0,
            true,
        );
        let left = x + 22.0;
        let width = w - 44.0;
        let response = self.response(
            (id, "slider"),
            left - 8.0,
            y + 53.0,
            width + 16.0,
            26.0,
            Sense::click_and_drag(),
        );
        let rail = self.rect(left, y + 62.0, width, 11.0);
        self.ui
            .painter()
            .rect_stroke(rail, 7.0 * self.scale, self.stroke(), StrokeKind::Inside);
        self.ui.painter().circle(
            self.pt(left + width * f32::from(value) / 63.0, y + 67.5),
            12.0 * self.scale,
            BG,
            self.stroke(),
        );
        if response.clicked() {
            response.request_focus()
        }
        if (response.dragged() || response.clicked())
            && let Some(pos) = response.interact_pointer_pos()
        {
            return Some(
                (((pos.x - self.pt(left, 0.0).x) / (width * self.scale)) * 63.0)
                    .round()
                    .clamp(0.0, 63.0) as u8,
            );
        }
        if response.has_focus() {
            let step = self.ui.input(|i| {
                i.key_pressed(egui::Key::ArrowRight) as i16
                    - i.key_pressed(egui::Key::ArrowLeft) as i16
            });
            if step != 0 {
                return Some((i16::from(value) + step).clamp(0, 63) as u8);
            }
        }
        None
    }
    fn tuner(&self, x: f32, y: f32, w: f32, title: &str, hz: Option<f64>) {
        self.text(x + w / 2.0, y + 9.0, title, 13.0, true);
        let gw = 134.0;
        self.art("tunerBG", x + (w - gw) / 2.0, y + 20.0, gw, 60.0);
        let center = self.pt(x + w / 2.0, y + 80.0);
        let cents = hz
            .map(|h| {
                let n = 69.0 + 12.0 * (h / 440.0).log2();
                (n - n.round()) * 100.0
            })
            .unwrap_or(0.0);
        let angle = (cents as f32 / 50.0 * 0.95).clamp(-0.95, 0.95);
        self.ui.painter().line_segment(
            [
                center,
                center + vec2(angle.sin() * 51.0, -angle.cos() * 51.0) * self.scale,
            ],
            self.stroke(),
        );
        let r = self.rect(x + w / 2.0 - 19.0, y + 55.0, 38.0, 25.0);
        self.ui.painter().rect(
            r,
            4.0 * self.scale,
            DARK,
            Stroke::new(self.scale, WHITE),
            StrokeKind::Inside,
        );
        self.text(
            x + w / 2.0,
            y + 68.0,
            hz.map(|h| note_name((69.0 + 12.0 * (h / 440.0).log2()).round() as i32))
                .unwrap_or("?".into()),
            17.0,
            true,
        );
    }
}
impl Neutron {
    fn bool_cell(
        &mut self,
        c: &mut Canvas<'_>,
        name: &str,
        x: f32,
        y: f32,
        w: f32,
        title: &str,
        labels: (&str, &str),
    ) {
        let v = self.values[name].as_bool().unwrap_or(false);
        if c.toggle(name, x, y, w, title, labels, v) {
            self.edit(name, Value::Bool(!v));
        }
    }
    fn radio_cell(
        &mut self,
        c: &mut Canvas<'_>,
        name: &str,
        x: f32,
        y: f32,
        w: f32,
        title: &str,
        labels: &[&str],
    ) {
        let v = self.values[name].as_u64().unwrap_or(0) as u8;
        if let Some(n) = c.radios(name, x, y, w, title, labels, v) {
            self.edit(name, Value::from(n));
        }
    }
    fn select_cell(
        &mut self,
        c: &mut Canvas<'_>,
        name: &str,
        x: f32,
        y: f32,
        w: f32,
        title: &str,
        items: Vec<(u8, String)>,
    ) {
        c.title(x, y, w, title);
        let value = self.values[name].as_u64().unwrap_or(0) as u8;
        let text = items
            .iter()
            .find(|(n, _)| *n == value)
            .map(|(_, s)| s.as_str())
            .unwrap_or("Unknown");
        if let Some(n) = c.dropdown(name, x + 12.0, y + 34.0, w - 24.0, text, &items, value) {
            self.edit(name, Value::from(n));
        }
    }
    fn oscillator_panel(&mut self, c: &mut Canvas<'_>, x: f32, y: f32) {
        c.panel("OSCILLATORS", "oscHeader", x, y, 480.0, 420.0);
        for row in 1..5 {
            c.line(x, y + 84.0 * row as f32, x + 480.0, y + 84.0 * row as f32);
        }
        for row in [0.0, 84.0, 252.0, 336.0] {
            for dx in [160.0, 320.0] {
                c.line(x + dx, y + row + 12.0, x + dx, y + row + 72.0);
            }
        }
        for i in 0..2 {
            let yy = y + i as f32 * 84.0;
            self.bool_cell(
                c,
                &format!("OSC{}_BLEND_MODE", i + 1),
                x,
                yy,
                160.0,
                &format!("OSC {} BLEND MODE", i + 1),
                ("BLEND", "SWITCH"),
            );
            self.bool_cell(
                c,
                &format!("OSC{}_TUNE_POT", i + 1),
                x + 160.0,
                yy,
                160.0,
                &format!("OSC {} TUNE POT", i + 1),
                ("ENABLE", "BYPASS"),
            );
            self.radio_cell(
                c,
                &format!("OSC{}_RANGE", i + 1),
                x + 320.0,
                yy,
                160.0,
                &format!("OSC {} RANGE", i + 1),
                &["32'", "16'", "8'", "±10 OCT"],
            );
        }
        c.tuner(x + 20.0, y + 172.0, 155.0, "OSC 1 TUNER", self.frequency[0]);
        c.tuner(
            x + 305.0,
            y + 172.0,
            155.0,
            "OSC 2 TUNER",
            self.frequency[1],
        );
        let delta = self.frequency[0]
            .zip(self.frequency[1])
            .map(|(a, b)| format!("{:+.1}", 12.0 * (b / a).log2()))
            .unwrap_or("?".into());
        c.ui.painter().rect(
            c.rect(x + 210.0, y + 197.0, 60.0, 23.0),
            4.0 * c.scale,
            DARK,
            Stroke::new(c.scale, WHITE),
            StrokeKind::Inside,
        );
        c.text(x + 240.0, y + 209.0, delta, 16.0, true);
        self.select_cell(
            c,
            "OSC1_AUTOGLIDE",
            x,
            y + 252.0,
            160.0,
            "OSC 1 AUTOGLIDE",
            (0..25)
                .map(|n| (n, format!("{:+}", i16::from(n) - 12)))
                .collect(),
        );
        self.bool_cell(
            c,
            "OSC_SYNC",
            x + 160.0,
            y + 252.0,
            160.0,
            "OSC SYNC",
            ("OFF", "ON"),
        );
        self.select_cell(
            c,
            "OSC2_AUTOGLIDE",
            x + 320.0,
            y + 252.0,
            160.0,
            "OSC 2 AUTOGLIDE",
            (0..25)
                .map(|n| (n, format!("{:+}", i16::from(n) - 12)))
                .collect(),
        );
        self.bool_cell(
            c,
            "PARAPHONIC_MODE",
            x,
            y + 336.0,
            160.0,
            "PARAPHONIC MODE",
            ("OFF", "ON"),
        );
        self.select_cell(
            c,
            "OSC_KEY_SPLIT",
            x + 160.0,
            y + 336.0,
            160.0,
            "OSC KEY SPLIT",
            std::iter::once((0, "Disabled".into()))
                .chain((24..87).map(|n| (n, note_name(i32::from(n)))))
                .collect(),
        );
        self.bool_cell(
            c,
            "OSC2_KEY_TRACK",
            x + 320.0,
            y + 336.0,
            160.0,
            "OSC 2 KEY TRACKING",
            ("TRACK", "HOLD"),
        );
    }
    fn lfo_panel(&mut self, c: &mut Canvas<'_>, x: f32, y: f32) {
        c.panel("LFO SETUP", "lfoHeader", x, y, 480.0, 454.0);
        for dy in [84.0, 168.0, 252.0, 352.0] {
            c.line(x, y + dy, x + 480.0, y + dy);
        }
        for dy in [0.0, 84.0] {
            for dx in [160.0, 320.0] {
                c.line(x + dx, y + dy + 12.0, x + dx, y + dy + 72.0);
            }
        }
        c.line(x + 160.0, y + 180.0, x + 160.0, y + 240.0);
        for (i, name, title) in [
            (0, "LFO_BLEND_MODE", "LFO BLEND MODE"),
            (1, "LFO_KEY_SYNC", "LFO KEY SYNC"),
            (2, "LFO_ONE_SHOT", "LFO ONE-SHOT"),
        ] {
            self.bool_cell(
                c,
                name,
                x + i as f32 * 160.0,
                y,
                160.0,
                title,
                if i == 0 {
                    ("BLEND", "SWITCH")
                } else {
                    ("OFF", "ON")
                },
            );
        }
        self.select_cell(
            c,
            "LFO_KEY_TRACK",
            x,
            y + 84.0,
            160.0,
            "LFO KEY TRACKING",
            std::iter::once((0, "Disabled".into()))
                .chain((12..109).map(|n| (n, note_name(i32::from(n)))))
                .collect(),
        );
        let v = self.values["LFO_DEPTH"].as_u64().unwrap_or(0) as u8;
        if let Some(n) = c.slider("LFO_DEPTH", x + 160.0, y + 84.0, 160.0, "LFO DEPTH", v) {
            self.edit("LFO_DEPTH", Value::from(n));
        }
        self.bool_cell(
            c,
            "LFO_RETRIGGER",
            x + 320.0,
            y + 84.0,
            160.0,
            "LFO RETRIGGER",
            ("OFF", "ON"),
        );
        self.bool_cell(
            c,
            "LFO_MIDI_SYNC",
            x,
            y + 168.0,
            160.0,
            "LFO MIDI SYNC",
            ("OFF", "ON"),
        );
        c.title(x + 160.0, y + 168.0, 320.0, "RESET LFO ORDER");
        if c.button("reset-lfo", x + 246.0, y + 202.0, 148.0, "RESET") {
            self.action("RESET_LFO_ORDER");
        }
        for (name, dy, title) in [
            ("LFO_SHAPE_ORDER", 252.0, "LFO SHAPE REORDER"),
            ("LFO_PHASE_OFFSET", 352.0, "LFO PHASE OFFSET"),
        ] {
            c.title(x, y + dy, 480.0, title);
            let mut value = self.values[name].clone();
            let array = value.as_array_mut().unwrap();
            let mut changed = false;
            for (i, item) in array.iter_mut().enumerate() {
                let val = item.as_u64().unwrap_or(0) as u8;
                let items: Vec<(u8, String)> = if name == "LFO_SHAPE_ORDER" {
                    ["Sine", "Triangle", "R. Saw", "Square", "Saw"]
                        .iter()
                        .enumerate()
                        .map(|(i, s)| (i as u8, s.to_string()))
                        .collect()
                } else {
                    (0..8)
                        .map(|n| (n, format!("{}°", u16::from(n) * 45)))
                        .collect()
                };
                let text = items
                    .iter()
                    .find(|(n, _)| *n == val)
                    .map(|(_, t)| t.as_str())
                    .unwrap_or("?");
                if let Some(n) = c.dropdown(
                    &format!("{name}-{i}"),
                    x + 9.0 + i as f32 * 95.0,
                    y + dy + 39.0,
                    82.0,
                    text,
                    &items,
                    val,
                ) {
                    *item = Value::from(n);
                    changed = true;
                }
                c.text(
                    x + 50.0 + i as f32 * 95.0,
                    y + dy + 82.0,
                    (i + 1).to_string(),
                    10.0,
                    true,
                );
            }
            if changed {
                self.edit(name, value);
            }
        }
    }
    fn filter_panel(&mut self, c: &mut Canvas<'_>, x: f32, y: f32) {
        c.panel("FILTERING", "filterHeader", x, y, 480.0, 244.0);
        c.line(x + 240.0, y + 12.0, x + 240.0, y + 232.0);
        c.line(x, y + 81.0, x + 240.0, y + 81.0);
        c.line(x, y + 162.0, x + 240.0, y + 162.0);
        self.bool_cell(
            c,
            "VCF_KEY_TRACK",
            x,
            y,
            240.0,
            "FILTER KEY TRACKING",
            ("OFF", "ON"),
        );
        let v = self.values["VCF_MOD_DEPTH"].as_u64().unwrap_or(0) as u8;
        if let Some(n) = c.slider("VCF_MOD_DEPTH", x, y + 81.0, 240.0, "FILTER MOD DEPTH", v) {
            self.edit("VCF_MOD_DEPTH", Value::from(n));
        }
        self.radio_cell(
            c,
            "VCF_MOD_SRC",
            x,
            y + 162.0,
            240.0,
            "FILTER MOD SOURCE",
            &["OFF", "AF. TOUCH", "MOD WHEEL", "VELO"],
        );
        c.title(x + 240.0, y, 240.0, "VCF MODE");
        let value = self.values["VCF_MODE"].as_u64().unwrap_or(0) as usize;
        for row in 0..2 {
            c.text(
                x + 258.0,
                y + 56.0 + row as f32 * 66.0,
                (row + 1).to_string(),
                12.0,
                false,
            );
            for col in 0..3 {
                let cx = x + 280.0 + col as f32 * 76.0;
                let cy = y + 81.0 + row as f32 * 66.0;
                let selected = (value + row) % 3 == col;
                let response = c.response(
                    ("VCF_MODE", row, col),
                    cx - 26.0,
                    cy - 26.0,
                    52.0,
                    52.0,
                    Sense::click(),
                );
                c.ui.painter().circle(
                    c.pt(cx, cy),
                    19.0 * c.scale,
                    if selected { WHITE } else { BG },
                    c.stroke(),
                );
                let ink = if selected { BG } else { WHITE };
                let points = match col {
                    0 => vec![(-10.0, 8.0), (-2.0, -8.0), (10.0, -8.0)],
                    1 => vec![(-10.0, 8.0), (0.0, -8.0), (10.0, 8.0)],
                    _ => vec![(-10.0, -8.0), (2.0, -8.0), (10.0, 8.0)],
                };
                c.ui.painter().add(egui::Shape::line(
                    points
                        .into_iter()
                        .map(|(a, b)| c.pt(cx + a, cy + b))
                        .collect(),
                    Stroke::new(2.5 * c.scale, ink),
                ));
                if response.clicked() {
                    self.edit("VCF_MODE", Value::from(((col + 3 - row) % 3) as u8));
                }
            }
        }
        c.line(x + 265.0, y + 198.0, x + 465.0, y + 198.0);
        for col in 0..3 {
            let cx = x + 280.0 + col as f32 * 76.0;
            c.line(cx, y + 175.0, cx, y + 199.0);
            c.ui.painter()
                .circle_filled(c.pt(cx, y + 198.0), 3.0 * c.scale, WHITE);
        }
        c.text(x + 400.0, y + 220.0, "HP        BP        LP", 11.0, false);
    }
    fn options_panel(&mut self, c: &mut Canvas<'_>, x: f32, y: f32) {
        c.panel("OPTIONS", "optionsHeader", x, y, 480.0, 405.0);
        for dy in [81.0, 162.0, 243.0, 324.0] {
            c.line(x, y + dy, x + 480.0, y + dy);
        }
        for row in [81.0, 162.0] {
            for dx in [160.0, 320.0] {
                c.line(x + dx, y + row + 12.0, x + dx, y + row + 69.0);
            }
        }
        for row in [0.0, 243.0] {
            c.line(x + 160.0, y + row + 12.0, x + 160.0, y + row + 69.0);
        }
        c.line(x + 240.0, y + 336.0, x + 240.0, y + 393.0);
        self.select_cell(
            c,
            "MIDI_CHANNEL",
            x,
            y,
            160.0,
            "MIDI CHANNEL",
            (0..16).map(|n| (n, (n + 1).to_string())).collect(),
        );
        c.title(x + 160.0, y, 320.0, "MIDI DEVICE");
        self.bool_cell(
            c,
            "DISABLE_MIDI_DIPS",
            x,
            y + 81.0,
            160.0,
            "DISABLE MIDI DIPS",
            ("OFF", "ON"),
        );
        self.bool_cell(
            c,
            "ENV_RETRIGGER",
            x + 160.0,
            y + 81.0,
            160.0,
            "ENV RETRIGGER",
            ("STACCATO", "LEGATO"),
        );
        let para = self.values["PARAPHONIC_MODE"].as_bool().unwrap_or(false);
        self.radio_cell(
            c,
            if para {
                "PARA_NOTE_PRIORITY"
            } else {
                "NOTE_PRIORITY"
            },
            x + 320.0,
            y + 81.0,
            160.0,
            "NOTE PRIORITY",
            if para {
                &["LOW", "HIGH", "LAST", "HI/LO"]
            } else {
                &["LOW", "HIGH", "LAST"]
            },
        );
        self.select_cell(
            c,
            "PITCH_BEND_RANGE",
            x,
            y + 162.0,
            160.0,
            "PITCH BEND RANGE",
            (0..25).map(|n| (n, n.to_string())).collect(),
        );
        self.radio_cell(
            c,
            "ASSIGN_OUT",
            x + 160.0,
            y + 162.0,
            160.0,
            "ASSIGNABLE OUT",
            &["OSC1", "OSC2", "VEL", "MOD", "AFT"],
        );
        self.bool_cell(
            c,
            "POLYCHAIN_MODE",
            x + 320.0,
            y + 162.0,
            160.0,
            "POLYCHAIN MODE",
            ("OFF", "ON"),
        );
        self.bool_cell(
            c,
            "KEY_RANGE_MUTE",
            x,
            y + 243.0,
            160.0,
            "KEY RANGE MUTE",
            ("OFF", "ON"),
        );
        c.title(x + 160.0, y + 243.0, 320.0, "KEY RANGE");
        for (i, name) in ["KEY_RANGE_MIN", "KEY_RANGE_MAX"].iter().enumerate() {
            let val = self.values[*name].as_u64().unwrap_or(24) as u8;
            let items = (24..97)
                .map(|n| (n, note_name(i32::from(n))))
                .collect::<Vec<_>>();
            if let Some(n) = c.dropdown(
                name,
                x + 170.0 + i as f32 * 154.0,
                y + 277.0,
                144.0,
                &note_name(i32::from(val)),
                &items,
                val,
            ) {
                self.edit(name, Value::from(n));
            }
            c.text(
                x + 242.0 + i as f32 * 154.0,
                y + 313.0,
                if i == 0 { "MIN" } else { "MAX" },
                10.0,
                false,
            );
        }
        c.title(x, y + 324.0, 240.0, "RESTORE DEFAULT SETTINGS");
        if c.button("restore", x + 48.0, y + 360.0, 144.0, "RESTORE") {
            self.restore_confirm = true;
        }
        c.title(x + 240.0, y + 324.0, 240.0, "RESET KEY RANGE");
        if c.button("reset-key", x + 288.0, y + 360.0, 144.0, "RESET") {
            self.action("RESET_KEY_RANGE");
        }
    }
    fn device_ports(&mut self, c: &mut Canvas<'_>, x: f32, y: f32) {
        for i in 0..2 {
            let ports = if i == 0 {
                &self.ports.inputs
            } else {
                &self.ports.outputs
            };
            let selected = if i == 0 { &self.input } else { &self.output };
            let text = if !self.live {
                "Offline"
            } else if selected.is_empty() {
                "Select device"
            } else {
                selected
            };
            let items: Vec<_> = ports
                .iter()
                .take(128)
                .enumerate()
                .map(|(n, s)| (n as u8, s.clone()))
                .collect();
            let index = ports.iter().position(|s| s == selected).unwrap_or(255) as u8;
            if let Some(n) = c.dropdown(
                if i == 0 { "input-port" } else { "output-port" },
                x + 170.0 + i as f32 * 154.0,
                y + 34.0,
                144.0,
                text,
                &items,
                index,
            ) {
                if self.connection.is_none() {
                    if i == 0 {
                        self.input = ports[n as usize].clone();
                    } else {
                        self.output = ports[n as usize].clone();
                    }
                }
            }
            c.text(
                x + 242.0 + i as f32 * 154.0,
                y + 70.0,
                if i == 0 { "IN" } else { "OUT" },
                10.0,
                false,
            );
        }
    }
    pub fn instrument_ui(&mut self, ui: &mut egui::Ui) {
        if self.artwork.is_empty() {
            for (name, bytes) in [
                (
                    "oscHeader",
                    include_bytes!("../assets/oscHeader.png").as_slice(),
                ),
                (
                    "lfoHeader",
                    include_bytes!("../assets/lfoHeader.png").as_slice(),
                ),
                (
                    "filterHeader",
                    include_bytes!("../assets/filterHeader.png").as_slice(),
                ),
                (
                    "optionsHeader",
                    include_bytes!("../assets/optionsHeader.png").as_slice(),
                ),
                (
                    "neutronLogo",
                    include_bytes!("../assets/neutronLogo.png").as_slice(),
                ),
                (
                    "behringerLogo",
                    include_bytes!("../assets/behringerLogo.png").as_slice(),
                ),
                (
                    "tunerBG",
                    include_bytes!("../assets/tunerBG.png").as_slice(),
                ),
            ] {
                self.artwork.insert(name, texture(ui.ctx(), name, bytes));
            }
        }
        ui.painter().rect_filled(ui.max_rect(), 0.0, BG);
        egui::Frame::new()
            .fill(Color32::from_rgb(34, 34, 34))
            .inner_margin(8)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    if ui.selectable_label(!self.live, "Offline preview").clicked() {
                        self.disconnect();
                        self.live = false;
                        self.values = protocol::defaults(&self.spec);
                        self.status = "Offline preview — no hardware access".into();
                    }
                    if ui.selectable_label(self.live, "Real synth").clicked() {
                        self.live = true;
                        self.status =
                            "Refresh MIDI ports, choose your Neutron, then connect".into();
                    }
                    if self.live {
                        if ui
                            .add_enabled(
                                self.connection.is_none(),
                                egui::Button::new("Refresh MIDI ports"),
                            )
                            .clicked()
                        {
                            self.refresh();
                        }
                        if self.connection.is_some() {
                            if ui.button("Disconnect").clicked() {
                                self.disconnect();
                            }
                        } else if ui
                            .add_enabled(
                                !self.input.is_empty() && !self.output.is_empty(),
                                egui::Button::new("Connect"),
                            )
                            .clicked()
                        {
                            self.connect();
                        }
                    }
                    ui.label(egui::RichText::new(&self.status).size(13.0));
                });
            });
        let assets = self.artwork.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            let wide = ui.available_width() >= 780.0;
            let design = if wide { 1040.0 } else { 520.0 };
            let scale = (ui.available_width() / design).min(1.35);
            let height = if wide { 982.0 } else { 1790.0 };
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height * scale), Sense::hover());
            let origin = rect.min + vec2((rect.width() - design * scale) / 2.0, 0.0);
            ui.add_enabled_ui(!self.live || (self.ready && self.got_settings), |ui| {
                let mut c = Canvas {
                    ui,
                    origin,
                    scale,
                    assets: assets.clone(),
                };
                c.art(
                    "neutronLogo",
                    if wide { 805.0 } else { 266.0 },
                    9.0,
                    137.0,
                    38.0,
                );
                c.art(
                    "behringerLogo",
                    if wide { 954.0 } else { 420.0 },
                    4.0,
                    68.0,
                    59.0,
                );
                self.oscillator_panel(&mut c, 16.0, 50.0);
                if wide {
                    self.lfo_panel(&mut c, 16.0, 516.0);
                    self.filter_panel(&mut c, 536.0, 94.0);
                    self.options_panel(&mut c, 536.0, 400.0);
                } else {
                    self.filter_panel(&mut c, 16.0, 525.0);
                    self.lfo_panel(&mut c, 16.0, 822.0);
                    self.options_panel(&mut c, 16.0, 1325.0);
                }
            });
            // Device selection must remain available before the firmware handshake.
            ui.add_enabled_ui(self.connection.is_none(), |ui| {
                let mut c = Canvas {
                    ui,
                    origin,
                    scale,
                    assets,
                };
                self.device_ports(
                    &mut c,
                    if wide { 536.0 } else { 16.0 },
                    if wide { 400.0 } else { 1325.0 },
                );
            });
            egui::CollapsingHeader::new("Advanced / MIDI messages").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Device ID");
                    let mut id = self.values["DEVICE_ID"].as_u64().unwrap_or(0) as u8;
                    if ui
                        .add_enabled(
                            !self.live || (self.ready && self.got_settings),
                            egui::DragValue::new(&mut id).range(0..=15),
                        )
                        .changed()
                    {
                        self.edit("DEVICE_ID", Value::from(id));
                    }
                });
                for line in self.log.iter().rev().take(30) {
                    ui.monospace(line);
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(app: &mut Neutron, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1100.0, 1300.0))),
            events,
            ..Default::default()
        };
        ctx.run_ui(input, |ui| app.draw(ui))
    }
    fn click(app: &mut Neutron, ctx: &egui::Context, pos: Pos2) {
        frame(
            app,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        frame(
            app,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
    }
    #[test]
    fn panel_pointer_events_generate_correct_midi() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = Neutron::default();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let toggle = ctx
            .read_response(egui::Id::new(("OSC_SYNC", "toggle")))
            .unwrap()
            .rect;
        click(&mut app, &ctx, toggle.center());
        assert_eq!(app.values["OSC_SYNC"], Value::Bool(true));
        assert!(app.log.iter().any(|s| s.ends_with("0A 0E 01 F7")));
        let radio = ctx
            .read_response(egui::Id::new(("OSC1_RANGE", 2usize)))
            .unwrap()
            .rect;
        click(&mut app, &ctx, radio.center());
        assert_eq!(app.values["OSC1_RANGE"], Value::from(2));
        assert!(app.log.iter().any(|s| s.ends_with("0A 26 02 F7")));
        let slider = ctx
            .read_response(egui::Id::new(("VCF_MOD_DEPTH", "slider")))
            .unwrap()
            .rect;
        click(&mut app, &ctx, slider.center());
        assert!((30..=33).contains(&app.values["VCF_MOD_DEPTH"].as_u64().unwrap()));
        assert!(app.connection.is_none());
        let combo = ctx
            .read_response(egui::Id::new(("OSC1_AUTOGLIDE", "select")))
            .unwrap()
            .rect;
        click(&mut app, &ctx, combo.center());
        assert!(egui::Popup::is_any_open(&ctx));
        let output = frame(&mut app, &ctx, vec![]);
        let choice = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(t) if t.galley.job.text == "-12" => Some(t.pos + vec2(5.0, 7.0)),
                _ => None,
            })
            .expect("visible dropdown option");
        click(&mut app, &ctx, choice);
        assert_eq!(app.values["OSC1_AUTOGLIDE"], Value::from(0));
        assert!(app.log.iter().any(|s| s.ends_with("0A 24 00 F7")));
    }
    #[test]
    fn disconnected_live_panel_does_not_send_on_click() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = Neutron {
            live: true,
            ..Default::default()
        };
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let r = ctx
            .read_response(egui::Id::new(("OSC_SYNC", "toggle")))
            .unwrap();
        assert!(!r.enabled());
        assert!(
            ctx.read_response(egui::Id::new(("input-port", "select")))
                .unwrap()
                .enabled()
        );
        click(&mut app, &ctx, r.rect.center());
        assert_eq!(app.values["OSC_SYNC"], Value::Bool(false));
        assert!(app.log.is_empty());
    }
    #[test]
    fn narrow_window_stacks_panels_and_keeps_controls_in_bounds() {
        let ctx = egui::Context::default();
        configure(&ctx);
        let mut app = Neutron::default();
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 2200.0))),
                ..Default::default()
            };
            let _ = ctx.run_ui(input, |ui| app.draw(ui));
        }
        for id in ["OSC1_AUTOGLIDE", "LFO_KEY_TRACK", "input-port"] {
            let response = ctx.read_response(egui::Id::new((id, "select"))).unwrap();
            assert!(response.rect.left() >= 0.0 && response.rect.right() <= 600.0);
        }
        let filter = ctx
            .read_response(egui::Id::new(("VCF_MOD_DEPTH", "slider")))
            .unwrap()
            .rect;
        let lfo = ctx
            .read_response(egui::Id::new(("LFO_DEPTH", "slider")))
            .unwrap()
            .rect;
        assert!(filter.bottom() < lfo.top());
        assert!(app.connection.is_none());
    }
}
