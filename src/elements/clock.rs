use crate::color::Color;
use crate::elements::Element;
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

        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, egui::Sense::hover());
        let painter = ui.painter_at(rect);

        // Three stacked rows: time (dominant), date (medium), tz label (small).
        // Cap each by both height-fraction and width-fraction so nothing overflows.
        // Width factors are tuned for monospace ~0.6 em advance + a little margin.
        let time_chars = if self.twenty_four_hour { 8.5 } else { 11.5 };
        let date_chars = 16.0;
        let label_chars = (self.label_text.len() as f32 + 2.0).max(6.0);

        let time_size = (rect.height() * 0.62).min(rect.width() / (time_chars * 0.62));
        let date_size = (rect.height() * 0.18).min(rect.width() / (date_chars * 0.62));
        let label_size = (rect.height() * 0.10).min(rect.width() / (label_chars * 0.62));

        // Distribute leftover vertical space as gaps above/between/below the rows.
        let rows = if self.show_label { 3 } else { 2 };
        let used = time_size + date_size + if self.show_label { label_size } else { 0.0 };
        let gap = ((rect.height() - used) / (rows as f32 + 1.0)).max(0.0);

        let center_x = rect.center().x;
        let label_color = self.color.linear_multiply(0.75);

        let mut y = rect.min.y + gap;
        painter.text(
            egui::pos2(center_x, y),
            egui::Align2::CENTER_TOP,
            time_str,
            egui::FontId::monospace(time_size),
            self.color,
        );
        y += time_size + gap;

        painter.text(
            egui::pos2(center_x, y),
            egui::Align2::CENTER_TOP,
            date_str,
            egui::FontId::monospace(date_size),
            label_color,
        );
        y += date_size + gap;

        if self.show_label {
            painter.text(
                egui::pos2(center_x, y),
                egui::Align2::CENTER_TOP,
                &self.label_text,
                egui::FontId::proportional(label_size),
                label_color,
            );
        }

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
