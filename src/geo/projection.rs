use crate::geo::LatLon;
use egui::{Pos2, Rect, pos2};

/// Maps geographic coordinates onto a pixel rectangle.
pub trait Projection {
    fn project(&self, rect: Rect, p: LatLon) -> Pos2;
}

/// Plate carrée: longitude -> x, latitude -> y, linearly. Cheap and clear.
pub struct Equirectangular;

impl Projection for Equirectangular {
    fn project(&self, rect: Rect, p: LatLon) -> Pos2 {
        let x = rect.min.x + ((p.lon + 180.0) / 360.0) * rect.width();
        let y = rect.min.y + ((90.0 - p.lat) / 180.0) * rect.height();
        pos2(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equirectangular_corners() {
        let r = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(360.0, 180.0));
        let p = Equirectangular;
        assert_eq!(
            p.project(
                r,
                LatLon {
                    lat: 90.0,
                    lon: -180.0
                }
            ),
            pos2(0.0, 0.0)
        );
        assert_eq!(
            p.project(
                r,
                LatLon {
                    lat: -90.0,
                    lon: 180.0
                }
            ),
            pos2(360.0, 180.0)
        );
        assert_eq!(
            p.project(r, LatLon { lat: 0.0, lon: 0.0 }),
            pos2(180.0, 90.0)
        );
    }
}
