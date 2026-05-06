//! Bundled equirectangular world-map textures.
//!
//! Each texture is `include_bytes!`'d at compile time (mirroring
//! `assets/coastline.geojson`) and decoded on demand into an
//! `egui::ColorImage` for upload to the GPU by the `Map` element.

use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone, Copy)]
pub enum TextureChoice {
    NaturalEarth,
    EarthAtNight,
}

impl TextureChoice {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "natural_earth" => Ok(Self::NaturalEarth),
            "earth_at_night" => Ok(Self::EarthAtNight),
            other => Err(anyhow!(
                "unknown texture {other:?} (expected \"natural_earth\" or \"earth_at_night\")"
            )),
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Self::NaturalEarth => include_bytes!("../assets/textures/natural_earth.png"),
            Self::EarthAtNight => include_bytes!("../assets/textures/earth_at_night.png"),
        }
    }
}

/// Decode a bundled texture into an `egui::ColorImage` (RGBA8). Run once at
/// `Map` construction; the resulting image is uploaded to the GPU on first paint.
pub fn decode(choice: TextureChoice) -> Result<egui::ColorImage> {
    let img = image::load_from_memory_with_format(choice.bytes(), image::ImageFormat::Png)
        .context("decode texture png")?
        .to_rgba8();
    let (w, h) = img.dimensions();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        img.as_raw(),
    ))
}
