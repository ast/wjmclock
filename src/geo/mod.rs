pub mod coastline;
pub mod maidenhead;
pub mod projection;
pub mod terminator;

pub use coastline::Coastline;
pub use projection::{Equirectangular, Projection};
pub use terminator::Subsolar;

/// A geographic coordinate in degrees: latitude (-90..=90), longitude (-180..=180).
#[derive(Debug, Clone, Copy)]
pub struct LatLon {
    pub lat: f32,
    pub lon: f32,
}
