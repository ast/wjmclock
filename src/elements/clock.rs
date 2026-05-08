use crate::color::Color;
use crate::elements::text_stack::{TextRow, paint_text_stack};
use crate::elements::{Element, claim_full_rect};
use anyhow::{Context, Result, anyhow};
use chrono::{Datelike, Timelike, Utc};
use chrono_tz::Tz;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClockCfg {
    #[serde(default = "default_tz")]
    timezone: String,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default = "default_label")]
    label: bool,
    #[serde(default = "default_color")]
    color: Color,
}

fn default_tz() -> String {
    "UTC".into()
}
fn default_format() -> String {
    "24h".into()
}
fn default_label() -> bool {
    true
}
fn default_color() -> Color {
    Color::rgb(0x7d, 0xf5, 0xd2)
}

/// Large digital clock. Supports any IANA timezone and 12h/24h format.
pub struct Clock {
    tz: Tz,
    label_text: String,
    twenty_four_hour: bool,
    show_label: bool,
    color: egui::Color32,
}

impl Clock {
    pub fn from_toml(value: toml::Value) -> Result<Self> {
        let cfg: ClockCfg = value.try_into().context("parse clock config")?;
        let tz: Tz = cfg
            .timezone
            .parse()
            .map_err(|e| anyhow!("unknown timezone {:?}: {}", cfg.timezone, e))?;
        let twenty_four_hour = match cfg.format.as_str() {
            "24h" | "24" => true,
            "12h" | "12" => false,
            other => return Err(anyhow!("format must be \"24h\" or \"12h\", got {other:?}")),
        };
        Ok(Self {
            tz,
            label_text: cfg.timezone,
            twenty_four_hour,
            show_label: cfg.label,
            color: cfg.color.into(),
        })
    }
}

impl Element for Clock {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        // Repaint at the next second boundary.
        let utc_now = Utc::now();
        let ms_to_next = 1000 - (utc_now.timestamp_subsec_millis() as i64);
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(ms_to_next.max(50) as u64));

        let now = utc_now.with_timezone(&self.tz);
        let time_str = if self.twenty_four_hour {
            format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second())
        } else {
            let (is_pm, h12) = now.hour12();
            format!(
                "{:>2}:{:02}:{:02} {}",
                h12,
                now.minute(),
                now.second(),
                if is_pm { "PM" } else { "AM" }
            )
        };
        let date_str = format!(
            "{} {:02} {} {}",
            weekday_short(now.weekday()),
            now.day(),
            month_short(now.month()),
            now.year(),
        );

        let (rect, response, painter) = claim_full_rect(ui);

        // Three stacked rows: time (dominant), date (medium), tz label (small).
        let time_chars = if self.twenty_four_hour { 8.5 } else { 11.5 };
        let date_chars = 16.0;
        let label_color = self.color.linear_multiply(0.75);

        let mut rows = vec![
            TextRow::monospace(time_str, 0.62, time_chars, self.color),
            TextRow::monospace(date_str, 0.18, date_chars, label_color),
        ];
        if self.show_label {
            let label_chars = (self.label_text.len() as f32 + 2.0).max(6.0);
            rows.push(TextRow::proportional(
                &self.label_text,
                0.10,
                label_chars,
                label_color,
            ));
        }

        paint_text_stack(&painter, rect, &rows);
        response
    }
}

fn weekday_short(w: chrono::Weekday) -> &'static str {
    match w {
        chrono::Weekday::Mon => "MON",
        chrono::Weekday::Tue => "TUE",
        chrono::Weekday::Wed => "WED",
        chrono::Weekday::Thu => "THU",
        chrono::Weekday::Fri => "FRI",
        chrono::Weekday::Sat => "SAT",
        chrono::Weekday::Sun => "SUN",
    }
}

fn month_short(m: u32) -> &'static str {
    [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ][((m - 1).min(11)) as usize]
}
