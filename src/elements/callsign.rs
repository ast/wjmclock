use crate::color::Color;
use crate::elements::Element;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CallsignCfg {
    call: String,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default = "default_color")]
    color: Color,
}

fn default_color() -> Color {
    Color::rgb(0xff, 0xd6, 0x6b)
}

/// Static text element designed for amateur-radio callsigns. A primary line
/// (monospace, dominant) and an optional subtitle (proportional, dimmer).
pub struct Callsign {
    call: String,
    subtitle: Option<String>,
    color: egui::Color32,
}

impl Callsign {
    pub fn from_toml(value: toml::Value) -> Result<Self> {
        let cfg: CallsignCfg = value.try_into().context("parse callsign config")?;
        Ok(Self {
            call: cfg.call,
            subtitle: cfg.subtitle,
            color: cfg.color.into(),
        })
    }
}

impl Element for Callsign {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, egui::Sense::hover());
        let painter = ui.painter_at(rect);

        let call_chars = (self.call.chars().count() as f32 + 0.5).max(4.0);
        let sub_chars = self
            .subtitle
            .as_ref()
            .map(|s| s.chars().count() as f32 + 1.0)
            .unwrap_or(8.0)
            .max(8.0);

        let has_sub = self.subtitle.is_some();
        let main_size = (rect.height() * if has_sub { 0.66 } else { 0.78 })
            .min(rect.width() / (call_chars * 0.62));
        let sub_size = (rect.height() * 0.18).min(rect.width() / (sub_chars * 0.55));

        let rows = if has_sub { 2 } else { 1 };
        let used = main_size + if has_sub { sub_size } else { 0.0 };
        let gap = ((rect.height() - used) / (rows as f32 + 1.0)).max(0.0);

        let center_x = rect.center().x;
        let mut y = rect.min.y + gap;
        painter.text(
            egui::pos2(center_x, y),
            egui::Align2::CENTER_TOP,
            &self.call,
            egui::FontId::monospace(main_size),
            self.color,
        );
        y += main_size + gap;

        if let Some(sub) = &self.subtitle {
            painter.text(
                egui::pos2(center_x, y),
                egui::Align2::CENTER_TOP,
                sub,
                egui::FontId::proportional(sub_size),
                self.color.linear_multiply(0.75),
            );
        }

        response
    }
}
