//! Bundled equirectangular world-map textures.
//!
//! Each texture is `include_bytes!`'d at compile time (mirroring
//! `assets/coastline.geojson`) and decoded on demand into an
//! `egui::ColorImage` for upload to the GPU by the `Map` element.

use anyhow::{Context, Result};

const DAY_PNG: &[u8] = include_bytes!("../assets/textures/natural_earth.png");
const NIGHT_PNG: &[u8] = include_bytes!("../assets/textures/earth_at_night.png");

/// Decode the bundled day basemap (Natural Earth III shaded relief) into an
/// `egui::ColorImage`. Run once at `Map` construction; uploaded to the GPU on
/// first paint.
pub fn decode_day() -> Result<egui::ColorImage> {
    decode_png(DAY_PNG, "day")
}

/// Decode the bundled night overlay (Earth at Night).
pub fn decode_night() -> Result<egui::ColorImage> {
    decode_png(NIGHT_PNG, "night")
}

fn decode_png(bytes: &[u8], label: &str) -> Result<egui::ColorImage> {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .with_context(|| format!("decode {label} texture png"))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        img.as_raw(),
    ))
}
