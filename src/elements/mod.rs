use crate::config::{ElementConfig, Marker};
use crate::error::AppError;

pub mod callsign;
pub mod clock;
pub mod map;
pub mod propagation;

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
pub trait Element {
    /// Per-frame state update (request_repaint, advance animations, etc.).
    fn update(&mut self, ctx: &egui::Context);
    /// Draw inside the rect implied by the parent UI.
    fn ui(&mut self, ui: &mut egui::Ui);
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
                source: e,
            }),
        other => Err(AppError::UnknownElement(other.into())),
    }
}
