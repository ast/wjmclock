use crate::config::{ElementConfig, Marker};
use crate::error::AppError;

pub mod callsign;
pub mod clock;
pub mod map;
pub mod propagation;
pub mod text_stack;

pub use callsign::Callsign;
pub use clock::Clock;
pub use map::Map;
pub use propagation::Propagation;

/// Globals available to every element at construction time.
/// `markers` includes the home marker (if configured); `home` is also kept
/// separately for non-map consumers (e.g., the propagation widget).
#[derive(Debug, Clone, Default)]
pub struct Globals {
    pub markers: Vec<Marker>,
    pub home: Option<Marker>,
}

/// A drawable, configurable widget placed in a fractional rect of the window.
///
/// Shaped like an `egui::Widget` except `&mut self` so elements can persist
/// state across frames (the worker handle in `Propagation`, etc.). Per-frame
/// state updates — `request_repaint_after`, lazy service init, … — happen
/// inside `ui()`. The blanket impl below makes `&mut dyn Element` usable with
/// `ui.add(...)` / `ui.put(...)` like any native widget.
///
/// Most elements paint absolutely with `ui.painter_at(rect)`; `claim_full_rect`
/// is the standard preamble for that style.
pub trait Element {
    fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response;
}

impl egui::Widget for &mut (dyn Element + '_) {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Element::ui(self, ui)
    }
}

/// Standard preamble for painter-based elements: take the full available rect,
/// claim it on the parent `Ui` (so auto-sizing containers like `egui::Window`
/// frame the content), and return a clipped `Painter` for it.
pub fn claim_full_rect(ui: &mut egui::Ui) -> (egui::Rect, egui::Response, egui::Painter) {
    let rect = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(rect, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    (rect, response, painter)
}

/// Construct an element from its TOML config. Adding a new element type =
/// add one file in `elements/` and one match arm here.
pub fn make_element(cfg: &ElementConfig, globals: &Globals) -> Result<Box<dyn Element>, AppError> {
    let extra = toml::Value::Table(cfg.extra.clone());
    match cfg.kind.as_str() {
        "clock" => Clock::from_toml(extra)
            .map(|e| Box::new(e) as Box<dyn Element>)
            .map_err(|e| AppError::ElementConfig {
                kind: cfg.kind.clone(),
                source: e.context("clock"),
            }),
        "map" => Map::from_toml(extra, globals)
            .map(|e| Box::new(e) as Box<dyn Element>)
            .map_err(|e| AppError::ElementConfig {
                kind: cfg.kind.clone(),
                source: e.context("map"),
            }),
        "callsign" => Callsign::from_toml(extra)
            .map(|e| Box::new(e) as Box<dyn Element>)
            .map_err(|e| AppError::ElementConfig {
                kind: cfg.kind.clone(),
                source: e.context("callsign"),
            }),
        "propagation" => Propagation::from_toml(extra, globals)
            .map(|e| Box::new(e) as Box<dyn Element>)
            .map_err(|e| AppError::ElementConfig {
                kind: cfg.kind.clone(),
                source: e.context("propagation"),
            }),
        other => Err(AppError::UnknownElement(other.into())),
    }
}
