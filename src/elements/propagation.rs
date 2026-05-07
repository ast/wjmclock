//! HF propagation overlay element.
//!
//! Reads the latest snapshot from `PropagationService` (background-fetched
//! by a worker thread; never blocks the UI) and renders two optional rows:
//! a global HF band-condition table derived from NOAA solar indices, and
//! per-marker path predictions (band openings) derived from KC2G's
//! ray-traced MUF/LUF series.

use crate::config::Marker;
use crate::elements::{Element, Globals};
use crate::geo::Subsolar;
use crate::propagation::bands::{HF_BANDS, Rating};
use crate::propagation::{PropagationService, PropagationSnapshot, Target, bands, kc2g};
use anyhow::{Context, Result, anyhow};
use chrono::{Timelike, Utc};
use egui::{Align2, Color32, FontId, Stroke, vec2};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PropagationCfg {
    #[serde(default = "default_true")]
    band_conditions: bool,
    #[serde(default = "default_true")]
    path_predictions: bool,
}

fn default_true() -> bool {
    true
}

pub struct Propagation {
    home: Marker,
    targets: Vec<Marker>,
    show_band_conditions: bool,
    show_path_predictions: bool,
    /// Lazy-started on the first `update()` call (we need an `egui::Context`).
    service: Option<PropagationService>,
}

impl Propagation {
    pub fn from_toml(value: toml::Value, globals: &Globals) -> Result<Self> {
        let cfg: PropagationCfg = value.try_into().context("parse propagation config")?;
        let home = globals
            .home
            .as_ref()
            .ok_or_else(|| anyhow!("propagation element requires [home] in config"))?
            .clone();

        // Path predictions need targets that aren't the home itself.
        let targets: Vec<Marker> = globals
            .markers
            .iter()
            .filter(|m| m.text != home.text)
            .cloned()
            .collect();

        Ok(Self {
            home,
            targets,
            show_band_conditions: cfg.band_conditions,
            show_path_predictions: cfg.path_predictions,
            service: None,
        })
    }

    fn ensure_service(&mut self, ctx: &egui::Context) {
        if self.service.is_some() {
            return;
        }
        let targets = self
            .targets
            .iter()
            .map(|m| Target {
                name: m.text.clone(),
                coord: m.coord,
            })
            .collect();
        self.service = Some(PropagationService::start(
            self.home.coord,
            targets,
            ctx.clone(),
        ));
    }
}

impl Element for Propagation {
    fn update(&mut self, ctx: &egui::Context) {
        self.ensure_service(ctx);
        // Day/night flips are visible in the band table; tick once a minute
        // so we redraw when the home QTH crosses the terminator.
        ctx.request_repaint_after(std::time::Duration::from_secs(60));
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let snap = self
            .service
            .as_ref()
            .map(|s| s.snapshot())
            .unwrap_or_default();

        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter_at(rect);
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::from_rgb(40, 60, 60)),
            egui::StrokeKind::Inside,
        );

        let pad = (rect.height() * 0.04).clamp(4.0, 12.0);
        let row_h = (rect.height() * 0.07).clamp(14.0, 22.0);
        let header_size = (row_h * 0.95).max(13.0);
        let body_size = (row_h * 0.78).max(11.0);

        let mut y = rect.min.y + pad;
        let x_left = rect.min.x + pad;
        let x_right = rect.max.x - pad;

        // ---- header ----
        let now = Utc::now();
        let title = "PROPAGATION";
        painter.text(
            egui::pos2(x_left, y),
            Align2::LEFT_TOP,
            title,
            FontId::proportional(header_size),
            Color32::from_rgb(220, 220, 220),
        );

        let status = match (&snap.fetched_at, &snap.last_error) {
            (Some(t), None) => format!("as of {:02}:{:02}Z", t.hour(), t.minute()),
            (Some(t), Some(_)) => format!("STALE — {:02}:{:02}Z", t.hour(), t.minute()),
            (None, Some(_)) => "ERR".into(),
            (None, None) => "loading…".into(),
        };
        let status_color = if snap.last_error.is_some() {
            Color32::from_rgb(255, 140, 90)
        } else {
            Color32::from_rgb(140, 180, 200)
        };
        painter.text(
            egui::pos2(x_right, y),
            Align2::RIGHT_TOP,
            &status,
            FontId::proportional(body_size),
            status_color,
        );
        y += header_size + pad;

        // ---- band conditions ----
        if self.show_band_conditions {
            y = draw_band_conditions(
                &painter,
                &snap,
                self.home.coord,
                now,
                rect,
                x_left,
                x_right,
                y,
                row_h,
                body_size,
            );
            y += pad;
        }

        // ---- path predictions ----
        if self.show_path_predictions {
            draw_path_predictions(
                &painter, &snap, now, rect, x_left, x_right, y, row_h, body_size,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_band_conditions(
    painter: &egui::Painter,
    snap: &PropagationSnapshot,
    home: crate::geo::LatLon,
    now: chrono::DateTime<Utc>,
    rect: egui::Rect,
    x_left: f32,
    x_right: f32,
    mut y: f32,
    row_h: f32,
    body_size: f32,
) -> f32 {
    let header_color = Color32::from_rgb(180, 200, 200);
    painter.text(
        egui::pos2(x_left, y),
        Align2::LEFT_TOP,
        "BAND CONDITIONS",
        FontId::proportional(body_size),
        header_color,
    );
    if let Some(s) = snap.solar {
        painter.text(
            egui::pos2(x_right, y),
            Align2::RIGHT_TOP,
            format!("SFI {:.0}  K {:.0}", s.sfi, s.k_index),
            FontId::monospace(body_size),
            header_color,
        );
    }
    y += body_size + 2.0;

    let Some(solar) = snap.solar else {
        painter.text(
            egui::pos2(x_left, y),
            Align2::LEFT_TOP,
            "(awaiting solar indices)",
            FontId::proportional(body_size),
            Color32::from_rgba_unmultiplied(200, 200, 200, 140),
        );
        return y + row_h;
    };

    let table = bands::derive(solar.sfi, solar.k_index);

    // Highlight the column matching the local day/night state at home.
    let is_day_at_home = Subsolar::at(now).elevation_at(home.lat, home.lon) >= 0.0;

    // Layout: label | day cell | night cell, three columns evenly across
    // the available width.
    let total_w = x_right - x_left;
    let label_w = total_w * 0.25;
    let cell_w = (total_w - label_w) * 0.5;

    // Sub-header
    painter.text(
        egui::pos2(x_left + label_w + cell_w * 0.5, y),
        Align2::CENTER_TOP,
        if is_day_at_home { "▶ DAY" } else { "DAY" },
        FontId::proportional(body_size * 0.95),
        if is_day_at_home {
            Color32::from_rgb(255, 220, 120)
        } else {
            header_color
        },
    );
    painter.text(
        egui::pos2(x_left + label_w + cell_w * 1.5, y),
        Align2::CENTER_TOP,
        if is_day_at_home { "NIGHT" } else { "▶ NIGHT" },
        FontId::proportional(body_size * 0.95),
        if !is_day_at_home {
            Color32::from_rgb(255, 220, 120)
        } else {
            header_color
        },
    );
    y += body_size + 2.0;

    for row in &table {
        if y + row_h > rect.max.y {
            break;
        }
        painter.text(
            egui::pos2(x_left, y),
            Align2::LEFT_TOP,
            row.label,
            FontId::monospace(body_size),
            Color32::from_rgb(220, 220, 220),
        );
        draw_rating_chip(
            painter,
            egui::Rect::from_min_size(
                egui::pos2(x_left + label_w, y),
                vec2(cell_w, body_size + 4.0),
            ),
            row.day,
            body_size,
        );
        draw_rating_chip(
            painter,
            egui::Rect::from_min_size(
                egui::pos2(x_left + label_w + cell_w, y),
                vec2(cell_w, body_size + 4.0),
            ),
            row.night,
            body_size,
        );
        y += row_h;
    }
    y
}

fn draw_rating_chip(painter: &egui::Painter, rect: egui::Rect, rating: Rating, font_size: f32) {
    let (bg, fg) = match rating {
        Rating::Good => (
            Color32::from_rgba_unmultiplied(60, 180, 90, 90),
            Color32::from_rgb(180, 240, 200),
        ),
        Rating::Fair => (
            Color32::from_rgba_unmultiplied(220, 180, 60, 80),
            Color32::from_rgb(255, 230, 160),
        ),
        Rating::Poor => (
            Color32::from_rgba_unmultiplied(220, 80, 80, 90),
            Color32::from_rgb(255, 180, 180),
        ),
    };
    let inset = rect.shrink2(vec2(2.0, 1.0));
    painter.rect_filled(inset, 2.0, bg);
    painter.text(
        inset.center(),
        Align2::CENTER_CENTER,
        format!("{rating}"),
        FontId::proportional(font_size),
        fg,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_path_predictions(
    painter: &egui::Painter,
    snap: &PropagationSnapshot,
    now: chrono::DateTime<Utc>,
    rect: egui::Rect,
    x_left: f32,
    x_right: f32,
    mut y: f32,
    row_h: f32,
    body_size: f32,
) {
    let header_color = Color32::from_rgb(180, 200, 200);
    painter.text(
        egui::pos2(x_left, y),
        Align2::LEFT_TOP,
        "PATHS FROM HOME",
        FontId::proportional(body_size),
        header_color,
    );
    y += body_size + 2.0;

    if snap.paths.is_empty() {
        painter.text(
            egui::pos2(x_left, y),
            Align2::LEFT_TOP,
            "(no markers configured)",
            FontId::proportional(body_size),
            Color32::from_rgba_unmultiplied(200, 200, 200, 140),
        );
        return;
    }

    for path in &snap.paths {
        if y + row_h > rect.max.y {
            break;
        }

        // Truncate name to fit column.
        let name_max = (x_right - x_left).min(160.0).min((x_right - x_left) * 0.4);
        let label_w = name_max;

        painter.text(
            egui::pos2(x_left, y),
            Align2::LEFT_TOP,
            &path.name,
            FontId::proportional(body_size),
            Color32::from_rgb(220, 220, 220),
        );

        let bands_str = match kc2g::nearest(&path.series, now) {
            None => "(no data)".to_string(),
            Some(p) => open_bands_string(p.luf_sp, p.muf_sp),
        };
        painter.text(
            egui::pos2(x_left + label_w, y),
            Align2::LEFT_TOP,
            &bands_str,
            FontId::monospace(body_size),
            Color32::from_rgb(180, 230, 180),
        );
        y += row_h;
    }
}

fn open_bands_string(luf_mhz: f32, muf_mhz: f32) -> String {
    let open: Vec<&'static str> = HF_BANDS
        .iter()
        .filter(|b| bands::path_open(luf_mhz, muf_mhz, b.freq_mhz))
        .map(|b| b.label)
        .collect();
    if open.is_empty() {
        "—".into()
    } else {
        open.join(" ")
    }
}
