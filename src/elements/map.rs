use crate::config::{Location, parse_color};
use crate::elements::{Element, Globals};
use crate::geo::{Coastline, Equirectangular, LatLon, Projection, Subsolar};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use egui::epaint::Vertex;
use egui::{Align2, Color32, FontId, Mesh, Pos2, Rect, Stroke, pos2, vec2};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MapCfg {
    #[serde(default = "default_projection")]
    projection: String,
    #[serde(default = "default_terminator")]
    terminator: bool,
    #[serde(default = "default_night_dim")]
    night_dim: f32,
    #[serde(default = "default_coast_color")]
    coast_color: String,
    #[serde(default = "default_grid")]
    grid: bool,
    #[serde(default = "default_grid_color")]
    grid_color: String,
    #[serde(default = "default_subsolar_marker")]
    subsolar_marker: bool,
    #[serde(default = "default_home_marker")]
    home_marker: bool,
    #[serde(default = "default_home_color")]
    home_color: String,
    #[serde(default = "default_night_color")]
    night_color: String,
    #[serde(default = "default_twilight_color")]
    twilight_color: String,
    /// Solar elevation in degrees at which the night overlay reaches full opacity.
    /// 6 = civil twilight, 12 = nautical (Geochron-like), 18 = astronomical.
    #[serde(default = "default_twilight_extent")]
    twilight_extent: f32,
    /// Map fill on the day side (the unobscured "lit ocean" colour). Must be
    /// brighter than `night_color` for the gray-line to read in the right
    /// direction.
    #[serde(default = "default_day_color")]
    day_color: String,
}

fn default_projection() -> String {
    "equirectangular".into()
}
fn default_terminator() -> bool {
    true
}
fn default_night_dim() -> f32 {
    0.85
}
fn default_coast_color() -> String {
    "#39c08c".into()
}
fn default_grid() -> bool {
    true
}
fn default_grid_color() -> String {
    "#1c3a3a".into()
}
fn default_subsolar_marker() -> bool {
    true
}
fn default_home_marker() -> bool {
    true
}
fn default_home_color() -> String {
    "#ff5577".into()
}
fn default_night_color() -> String {
    "#04091e".into()
}
fn default_twilight_color() -> String {
    "#ffb060".into()
}
fn default_twilight_extent() -> f32 {
    12.0
}
fn default_day_color() -> String {
    "#15233f".into()
}

/// World map with coastlines and an optional day/night terminator overlay.
pub struct Map {
    coastline: Coastline,
    projection: Equirectangular,
    show_terminator: bool,
    night_dim: f32,
    coast_color: Color32,
    grid: bool,
    grid_color: Color32,
    show_subsolar: bool,
    home: Option<Location>,
    show_home: bool,
    home_color: Color32,
    night_color: Color32,
    twilight_color: Color32,
    twilight_extent: f32,
    day_color: Color32,
}

impl Map {
    pub fn from_toml(value: toml::Value, globals: &Globals) -> Result<Self> {
        let cfg: MapCfg = value.try_into().context("parse map config")?;
        match cfg.projection.as_str() {
            "equirectangular" | "platecarree" => {}
            other => return Err(anyhow!("unsupported projection: {other:?}")),
        }
        let coastline = Coastline::load().context("load coastline")?;
        Ok(Self {
            coastline,
            projection: Equirectangular,
            show_terminator: cfg.terminator,
            night_dim: cfg.night_dim.clamp(0.0, 1.0),
            coast_color: parse_color(&cfg.coast_color),
            grid: cfg.grid,
            grid_color: parse_color(&cfg.grid_color),
            show_subsolar: cfg.subsolar_marker,
            home: globals.home.clone(),
            show_home: cfg.home_marker,
            home_color: parse_color(&cfg.home_color),
            night_color: parse_color(&cfg.night_color),
            twilight_color: parse_color(&cfg.twilight_color),
            twilight_extent: cfg.twilight_extent.clamp(1.0, 30.0),
            day_color: parse_color(&cfg.day_color),
        })
    }
}

impl Element for Map {
    fn update(&mut self, ctx: &egui::Context) {
        // The terminator drifts ~0.25° per minute — refreshing once a minute is plenty.
        ctx.request_repaint_after(std::time::Duration::from_secs(60));
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter_at(rect);

        // Daytime backdrop. Must be brighter than `night_color` or the
        // overlay reads inverted (lit area looks darker than the night side).
        painter.rect_filled(rect, 0.0, self.day_color);

        if self.show_terminator {
            draw_terminator(
                &painter,
                rect,
                self.night_dim,
                self.twilight_color,
                self.night_color,
                self.twilight_extent,
            );
        }
        if self.grid {
            draw_grid(&painter, rect, self.grid_color);
        }
        draw_coastlines(
            &painter,
            rect,
            &self.coastline,
            &self.projection,
            self.coast_color,
        );
        if self.show_subsolar {
            draw_subsolar(&painter, rect, &self.projection);
        }
        if self.show_home
            && let Some(home) = &self.home
        {
            draw_home(&painter, rect, &self.projection, home, self.home_color);
        }
        // Border.
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::from_rgb(40, 60, 60)),
            egui::StrokeKind::Inside,
        );
    }
}

fn draw_grid(painter: &egui::Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.0, color);
    // Lat lines every 30°.
    for lat in (-60..=60).step_by(30) {
        let y = rect.min.y + ((90.0 - lat as f32) / 180.0) * rect.height();
        painter.line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], stroke);
    }
    // Lon lines every 30°.
    for lon in (-150..=150).step_by(30) {
        let x = rect.min.x + ((lon as f32 + 180.0) / 360.0) * rect.width();
        painter.line_segment([pos2(x, rect.min.y), pos2(x, rect.max.y)], stroke);
    }
    // Equator + prime meridian, slightly stronger.
    let strong = Stroke::new(1.0, color.gamma_multiply(1.6));
    let eq_y = rect.center().y;
    painter.line_segment([pos2(rect.min.x, eq_y), pos2(rect.max.x, eq_y)], strong);
    let pm_x = rect.center().x;
    painter.line_segment([pos2(pm_x, rect.min.y), pos2(pm_x, rect.max.y)], strong);
}

fn draw_coastlines(
    painter: &egui::Painter,
    rect: Rect,
    coast: &Coastline,
    proj: &Equirectangular,
    color: Color32,
) {
    let stroke = Stroke::new(1.2, color);
    for line in &coast.lines {
        if line.len() < 2 {
            continue;
        }
        // Antimeridian split: when consecutive longitudes jump >180°, break the line.
        let mut prev: Option<(LatLon, Pos2)> = None;
        for &p in line {
            let pixel = proj.project(rect, p);
            if let Some((pp, prev_pixel)) = prev {
                let dlon = (p.lon - pp.lon).abs();
                if dlon < 180.0 {
                    painter.line_segment([prev_pixel, pixel], stroke);
                }
            }
            prev = Some((p, pixel));
        }
    }
}

/// Geochron-style grayline: a smooth gradient from transparent at the
/// day/night boundary, through a warm twilight tint, into a deep navy at full
/// night. Sampled on a 192×96 grid — vertex colors interpolate per-pixel.
fn draw_terminator(
    painter: &egui::Painter,
    rect: Rect,
    night_dim: f32,
    twilight: Color32,
    night: Color32,
    extent_deg: f32,
) {
    const NX: usize = 192;
    const NY: usize = 96;
    let sub = Subsolar::at(Utc::now());

    let mut mesh = Mesh::default();
    let dx = rect.width() / NX as f32;
    let dy = rect.height() / NY as f32;
    for j in 0..=NY {
        let v = j as f32 / NY as f32;
        let lat = 90.0 - v * 180.0;
        for i in 0..=NX {
            let u = i as f32 / NX as f32;
            let lon = -180.0 + u * 360.0;
            let elev = sub.elevation_at(lat, lon);
            let pos = pos2(rect.min.x + i as f32 * dx, rect.min.y + j as f32 * dy);
            mesh.vertices.push(Vertex {
                pos,
                uv: egui::epaint::WHITE_UV,
                color: terminator_color(elev, night_dim, twilight, night, extent_deg),
            });
        }
    }
    let stride = (NX + 1) as u32;
    for j in 0..NY as u32 {
        for i in 0..NX as u32 {
            let i00 = j * stride + i;
            let i10 = i00 + 1;
            let i01 = i00 + stride;
            let i11 = i01 + 1;
            mesh.indices
                .extend_from_slice(&[i00, i10, i11, i00, i11, i01]);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Color at a given solar elevation (degrees), as the *overlay* applied to the
/// background. Day -> transparent. Twilight -> warm tint, alpha rising. Night
/// -> deep navy at `night_dim` opacity. Smoothstep alpha keeps the band soft.
fn terminator_color(
    elev_deg: f32,
    night_dim: f32,
    twilight: Color32,
    night: Color32,
    extent_deg: f32,
) -> Color32 {
    if elev_deg >= 0.0 {
        return Color32::TRANSPARENT;
    }
    let t = ((-elev_deg) / extent_deg).clamp(0.0, 1.0);

    // Warm tint dominates near the terminator and fades sharply into night.
    let warm_w = (1.0 - t).powf(2.5);
    let mix = |a: u8, b: u8| ((a as f32 * warm_w) + (b as f32 * (1.0 - warm_w))) as u8;
    let r = mix(twilight.r(), night.r());
    let g = mix(twilight.g(), night.g());
    let b = mix(twilight.b(), night.b());

    // Smoothstep alpha: gentle ramp from 0 at sunrise to night_dim at full night.
    let a_smooth = t * t * (3.0 - 2.0 * t);
    let alpha = (a_smooth * night_dim * 255.0) as u8;

    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

fn draw_subsolar(painter: &egui::Painter, rect: Rect, proj: &Equirectangular) {
    let sub = Subsolar::at(Utc::now());
    let p = proj.project(
        rect,
        LatLon {
            lat: sub.lat,
            lon: sub.lon,
        },
    );
    let r = (rect.width().min(rect.height()) * 0.012).max(4.0);
    painter.circle_filled(p, r, Color32::from_rgb(255, 220, 80));
    painter.circle_stroke(
        p,
        r * 1.8,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 220, 80, 120)),
    );
    // Tiny ray cross.
    let ray = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 220, 80, 180));
    painter.line_segment([p - vec2(r * 2.4, 0.0), p + vec2(r * 2.4, 0.0)], ray);
    painter.line_segment([p - vec2(0.0, r * 2.4), p + vec2(0.0, r * 2.4)], ray);
}

fn draw_home(
    painter: &egui::Painter,
    rect: Rect,
    proj: &Equirectangular,
    home: &Location,
    color: Color32,
) {
    let p = proj.project(rect, home.coord);
    let r = (rect.width().min(rect.height()) * 0.010).max(4.0);

    // Filled disc + bright outline.
    painter.circle_filled(p, r, color);
    painter.circle_stroke(p, r, Stroke::new(1.5, Color32::WHITE));
    // Soft halo.
    painter.circle_stroke(p, r * 2.2, Stroke::new(1.0, color.gamma_multiply(0.6)));

    // Label, offset down-right so it doesn't overlap the disc.
    let font = FontId::proportional((r * 2.0).max(12.0));
    let label_pos = p + vec2(r * 1.6, r * 1.2);
    let label_color = Color32::from_rgb(230, 230, 230);
    // Subtle drop-shadow for legibility on bright map regions.
    painter.text(
        label_pos + vec2(1.0, 1.0),
        Align2::LEFT_TOP,
        &home.label,
        font.clone(),
        Color32::from_rgba_unmultiplied(0, 0, 0, 180),
    );
    painter.text(label_pos, Align2::LEFT_TOP, &home.label, font, label_color);
}
