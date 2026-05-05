use crate::config::FractionalRect;
use egui::Rect;

/// Resolves a fractional rect (0..1 in both axes) against a pixel screen rect.
pub struct Layout;

impl Layout {
    pub fn resolve(screen: Rect, frac: FractionalRect) -> Rect {
        let min = egui::pos2(
            screen.min.x + frac.x * screen.width(),
            screen.min.y + frac.y * screen.height(),
        );
        let size = egui::vec2(frac.w * screen.width(), frac.h * screen.height());
        Rect::from_min_size(min, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_fractional_rect_to_pixels() {
        let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1920.0, 1080.0));
        let frac = FractionalRect {
            x: 0.0,
            y: 0.25,
            w: 1.0,
            h: 0.75,
        };
        let r = Layout::resolve(screen, frac);
        assert_eq!(r.min.x, 0.0);
        assert_eq!(r.min.y, 270.0);
        assert_eq!(r.width(), 1920.0);
        assert_eq!(r.height(), 810.0);
    }
}
