use crate::color::Color;
use crate::config::{Marker, MarkerKind};
use crate::elements::{Element, Globals};
use crate::geo::{Coastline, Equirectangular, LatLon, Projection, Subsolar};
use crate::textures;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Timelike, Utc};
use egui::epaint::Vertex;
use egui::{Align2, Color32, FontId, Mesh, Pos2, Rect, Stroke, TextureHandle, pos2, vec2};
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
    coast_color: Color,
    #[serde(default = "default_grid")]
    grid: bool,
    #[serde(default = "default_grid_color")]
    grid_color: Color,
    #[serde(default = "default_subsolar_marker")]
    subsolar_marker: bool,
    #[serde(default = "default_marker_color")]
    marker_color: Color,
    /// When true, draw the bundled day basemap (Natural Earth III) and overlay
    /// the bundled Earth-at-Night raster on the night side. The terminator's
    /// deep-night dimming auto-fades so the city lights stay visible.
    /// Default false → flat `day_color` fill.
    #[serde(default)]
    texture: bool,
    /// Draw the vector coastline overlay on top of the base. Default true;
    /// turn off when the texture already shows coastlines.
    #[serde(default = "default_coastline")]
    coastline: bool,
    #[serde(default = "default_night_color")]
    night_color: Color,
    #[serde(default = "default_twilight_color")]
    twilight_color: Color,
    /// Solar elevation in degrees at which the night overlay reaches full opacity.
    /// 6 = civil twilight, 12 = nautical (Geochron-like), 18 = astronomical.
    #[serde(default = "default_twilight_extent")]
    twilight_extent: f32,
    /// Map fill on the day side (the unobscured "lit ocean" colour). Must be
    /// brighter than `night_color` for the gray-line to read in the right
    /// direction.
    #[serde(default = "default_day_color")]
    day_color: Color,
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
fn default_coast_color() -> Color {
    Color::rgb(0x39, 0xc0, 0x8c)
}
fn default_grid() -> bool {
    true
}
fn default_grid_color() -> Color {
    Color::rgb(0x1c, 0x3a, 0x3a)
}
fn default_subsolar_marker() -> bool {
    true
}
fn default_marker_color() -> Color {
    Color::rgb(0xff, 0x55, 0x77)
}
fn default_night_color() -> Color {
    Color::rgb(0x04, 0x09, 0x1e)
}
fn default_twilight_color() -> Color {
    Color::rgb(0xff, 0xb0, 0x60)
}
fn default_twilight_extent() -> f32 {
    12.0
}
fn default_day_color() -> Color {
    Color::rgb(0x15, 0x23, 0x3f)
}
fn default_coastline() -> bool {
    true
}

/// Day basemap + night overlay with lazy GPU upload. Decoded at `Map`
/// construction (errors surface early); uploaded on first paint when an
/// `egui::Context` is available.
#[derive(Default)]
struct MapTextures {
    day_pending: Option<egui::ColorImage>,
    day: Option<TextureHandle>,
    night_pending: Option<egui::ColorImage>,
    night: Option<TextureHandle>,
}

impl MapTextures {
    /// Decode both bundled rasters when `enabled`; otherwise leave empty.
    fn new(enabled: bool) -> Result<Self> {
        if enabled {
            Ok(Self {
                day_pending: Some(textures::decode_day()?),
                day: None,
                night_pending: Some(textures::decode_night()?),
                night: None,
            })
        } else {
            Ok(Self::default())
        }
    }

    /// Upload any pending images to the GPU. Idempotent after the first call.
    fn upload_pending(&mut self, ctx: &egui::Context) {
        if let Some(img) = self.day_pending.take() {
            self.day =
                Some(ctx.load_texture("wjmclock_map_day", img, egui::TextureOptions::LINEAR));
        }
        if let Some(img) = self.night_pending.take() {
            self.night =
                Some(ctx.load_texture("wjmclock_map_night", img, egui::TextureOptions::LINEAR));
        }
    }
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
    show_coastline: bool,
    markers: Vec<Marker>,
    marker_color: Color32,
    night_color: Color32,
    twilight_color: Color32,
    twilight_extent: f32,
    day_color: Color32,
    textures: MapTextures,
}

impl Map {
    pub fn from_toml(value: toml::Value, globals: &Globals) -> Result<Self> {
        let cfg: MapCfg = value.try_into().context("parse map config")?;
        match cfg.projection.as_str() {
            "equirectangular" | "platecarree" => {}
            other => return Err(anyhow!("unsupported projection: {other:?}")),
        }
        let coastline = Coastline::load().context("load coastline")?;
        let textures = MapTextures::new(cfg.texture)?;
        Ok(Self {
            coastline,
            projection: Equirectangular,
            show_terminator: cfg.terminator,
            night_dim: cfg.night_dim.clamp(0.0, 1.0),
            coast_color: cfg.coast_color.into(),
            grid: cfg.grid,
            grid_color: cfg.grid_color.into(),
            show_subsolar: cfg.subsolar_marker,
            show_coastline: cfg.coastline,
            markers: globals.markers.clone(),
            marker_color: cfg.marker_color.into(),
            night_color: cfg.night_color.into(),
            twilight_color: cfg.twilight_color.into(),
            twilight_extent: cfg.twilight_extent.clamp(1.0, 30.0),
            day_color: cfg.day_color.into(),
            textures,
        })
    }
}

impl Element for Map {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        // The terminator drifts ~0.25° per minute — refreshing once a minute is plenty.
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(60));

        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, egui::Sense::hover());
        let painter = ui.painter_at(rect);

        // Lazy GPU upload: we need a Context for `load_texture`, only
        // available once we're inside `ui`. Idempotent after first paint.
        self.textures.upload_pending(ui.ctx());

        // Solar geometry is needed by both the night-texture mask and the
        // terminator overlay; compute once so they stay in lockstep.
        let now = Utc::now();
        let sub = Subsolar::at(now);

        // Base layer: bundled day texture, or the flat day_color fill.
        // day_color must be brighter than night_color or the terminator
        // overlay reads inverted (lit area looks darker than the night side).
        if let Some(tex) = &self.textures.day {
            let mut mesh = Mesh::with_texture(tex.id());
            mesh.add_rect_with_uv(
                rect,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            painter.add(egui::Shape::mesh(mesh));
        } else {
            painter.rect_filled(rect, 0.0, self.day_color);
        }

        // Night-side texture (city lights), alpha-masked by solar elevation.
        // Drawn under the terminator so the warm twilight band still tints
        // the western edge of the night side.
        if let Some(tex) = &self.textures.night {
            draw_night_texture(&painter, rect, &sub, tex, self.twilight_extent);
        }

        if self.show_terminator {
            draw_terminator(
                &painter,
                rect,
                &sub,
                self.night_dim,
                self.twilight_color,
                self.night_color,
                self.twilight_extent,
                // When the night texture is in play, fade the deep-night fill
                // to transparent so city lights stay visible — only the warm
                // twilight band remains as a tint.
                self.textures.night.is_some(),
            );
        }
        if self.grid {
            draw_grid(&painter, rect, self.grid_color);
        }
        if self.show_coastline {
            draw_coastlines(
                &painter,
                rect,
                &self.coastline,
                &self.projection,
                self.coast_color,
            );
        }
        if self.show_subsolar {
            draw_subsolar(&painter, rect, &sub, &self.projection);
        }
        for marker in &self.markers {
            draw_marker(
                &painter,
                rect,
                &self.projection,
                marker,
                self.marker_color,
                now,
            );
        }
        // Border.
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::from_rgb(40, 60, 60)),
            egui::StrokeKind::Inside,
        );

        response
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

/// Solar-elevation mesh resolution. Both `draw_terminator` and
/// `draw_night_texture` sample on this grid; vertex colors interpolate
/// per-pixel.
const LIT_NX: usize = 192;
const LIT_NY: usize = 96;

/// Build a 192×96 mesh keyed off solar elevation. The closure receives
/// `(u, v, elev_deg)` for each grid vertex and returns its `(uv, color)`.
fn build_lit_mesh<F>(rect: Rect, sub: &Subsolar, mut mesh: Mesh, mut per_vertex: F) -> Mesh
where
    F: FnMut(f32, f32, f32) -> (Pos2, Color32),
{
    let dx = rect.width() / LIT_NX as f32;
    let dy = rect.height() / LIT_NY as f32;
    for j in 0..=LIT_NY {
        let v = j as f32 / LIT_NY as f32;
        let lat = 90.0 - v * 180.0;
        for i in 0..=LIT_NX {
            let u = i as f32 / LIT_NX as f32;
            let lon = -180.0 + u * 360.0;
            let elev = sub.elevation_at(lat, lon);
            let (uv, color) = per_vertex(u, v, elev);
            let pos = pos2(rect.min.x + i as f32 * dx, rect.min.y + j as f32 * dy);
            mesh.vertices.push(Vertex { pos, uv, color });
        }
    }
    let stride = (LIT_NX + 1) as u32;
    for j in 0..LIT_NY as u32 {
        for i in 0..LIT_NX as u32 {
            let i00 = j * stride + i;
            let i10 = i00 + 1;
            let i01 = i00 + stride;
            let i11 = i01 + 1;
            mesh.indices
                .extend_from_slice(&[i00, i10, i11, i00, i11, i01]);
        }
    }
    mesh
}

/// Smoothstep "nightness" of a solar elevation: 0 on the day side, rising
/// smoothly to 1 at `extent_deg` below the horizon.
fn night_smoothstep(elev_deg: f32, extent_deg: f32) -> f32 {
    if elev_deg >= 0.0 {
        return 0.0;
    }
    let t = ((-elev_deg) / extent_deg).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Geochron-style grayline: a smooth gradient from transparent at the
/// day/night boundary, through a warm twilight tint, into a deep navy at full
/// night.
///
/// `fade_at_night = true` collapses the deep-night dimming to transparent so
/// an underlying night texture (city lights) stays visible — only the warm
/// twilight band remains as a tint.
#[allow(clippy::too_many_arguments)]
fn draw_terminator(
    painter: &egui::Painter,
    rect: Rect,
    sub: &Subsolar,
    night_dim: f32,
    twilight: Color32,
    night: Color32,
    extent_deg: f32,
    fade_at_night: bool,
) {
    let mesh = build_lit_mesh(rect, sub, Mesh::default(), |_, _, elev| {
        let color = terminator_color(elev, night_dim, twilight, night, extent_deg, fade_at_night);
        (egui::epaint::WHITE_UV, color)
    });
    painter.add(egui::Shape::mesh(mesh));
}

/// Color at a given solar elevation (degrees), as the *overlay* applied to the
/// background. Day -> transparent. Twilight -> warm tint, alpha rising. Night
/// -> deep navy at `night_dim` opacity.
///
/// `fade_at_night = true` shapes the alpha into a bell that peaks in the
/// twilight band and falls back to 0 at full night, so an underlying night
/// texture remains visible.
fn terminator_color(
    elev_deg: f32,
    night_dim: f32,
    twilight: Color32,
    night: Color32,
    extent_deg: f32,
    fade_at_night: bool,
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

    let alpha_curve = if fade_at_night {
        // Bell: 0 at day, peaks in the twilight band, 0 at full night.
        4.0 * t * (1.0 - t)
    } else {
        night_smoothstep(elev_deg, extent_deg)
    };
    let alpha = (alpha_curve * night_dim * 255.0) as u8;

    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

/// Earth-at-night raster overlay: textured 192×96 mesh with per-vertex alpha
/// matching the night-side smoothstep.
fn draw_night_texture(
    painter: &egui::Painter,
    rect: Rect,
    sub: &Subsolar,
    tex: &TextureHandle,
    extent_deg: f32,
) {
    let mesh = build_lit_mesh(rect, sub, Mesh::with_texture(tex.id()), |u, v, elev| {
        let alpha = (night_smoothstep(elev, extent_deg) * 255.0) as u8;
        (
            pos2(u, v),
            Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
        )
    });
    painter.add(egui::Shape::mesh(mesh));
}

fn draw_subsolar(painter: &egui::Painter, rect: Rect, sub: &Subsolar, proj: &Equirectangular) {
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

fn draw_marker(
    painter: &egui::Painter,
    rect: Rect,
    proj: &Equirectangular,
    marker: &Marker,
    color: Color32,
    now: DateTime<Utc>,
) {
    let p = proj.project(rect, marker.coord);
    let r = (rect.width().min(rect.height()) * 0.005).max(3.0);

    match marker.kind {
        MarkerKind::Dot => {
            // Filled disc + bright outline.
            painter.circle_filled(p, r, color);
            painter.circle_stroke(p, r, Stroke::new(1.0, Color32::WHITE));
            // Soft halo.
            painter.circle_stroke(p, r * 2.2, Stroke::new(1.0, color.gamma_multiply(0.6)));
        }
    }

    // Label size is independent of the disc — shrinking the dot shouldn't
    // shrink the text.
    let label_size = (rect.width().min(rect.height()) * 0.020).clamp(12.0, 24.0);
    let font = FontId::proportional(label_size);
    // Sit just outside the halo, vertically centred on the disc.
    let label_pos = p + vec2(r * 2.2 + 4.0, -label_size * 0.4);
    let label_color = Color32::from_rgb(230, 230, 230);
    let shadow = Color32::from_rgba_unmultiplied(0, 0, 0, 180);
    // Subtle drop-shadow for legibility on bright map regions.
    painter.text(
        label_pos + vec2(1.0, 1.0),
        Align2::LEFT_TOP,
        &marker.text,
        font.clone(),
        shadow,
    );
    painter.text(label_pos, Align2::LEFT_TOP, &marker.text, font, label_color);

    // Optional local time, smaller, beneath the label. Map repaints once a
    // minute (see Map::update), so minute precision is the right granularity.
    if let Some(tz) = marker.tz {
        let local = now.with_timezone(&tz);
        let time_str = format!("{:02}:{:02}", local.hour(), local.minute());
        let time_size = (label_size * 0.85).max(11.0);
        let time_font = FontId::monospace(time_size);
        let time_pos = label_pos + vec2(0.0, label_size * 1.05);
        painter.text(
            time_pos + vec2(1.0, 1.0),
            Align2::LEFT_TOP,
            &time_str,
            time_font.clone(),
            shadow,
        );
        painter.text(
            time_pos,
            Align2::LEFT_TOP,
            &time_str,
            time_font,
            label_color,
        );
    }
}
