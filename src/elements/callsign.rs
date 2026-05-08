use crate::color::Color;
use crate::elements::text_stack::{TextRow, paint_text_stack};
use crate::elements::{Element, claim_full_rect};
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
        let (rect, response, painter) = claim_full_rect(ui);

        let call_chars = (self.call.chars().count() as f32 + 0.5).max(4.0);
        let has_sub = self.subtitle.is_some();
        let mut rows = vec![TextRow::monospace(
            &self.call,
            if has_sub { 0.66 } else { 0.78 },
            call_chars,
            self.color,
        )];
        if let Some(sub) = &self.subtitle {
            let sub_chars = (sub.chars().count() as f32 + 1.0).max(8.0);
            rows.push(TextRow::proportional(
                sub,
                0.18,
                sub_chars,
                self.color.linear_multiply(0.75),
            ));
        }

        paint_text_stack(&painter, rect, &rows);
        response
    }
}
