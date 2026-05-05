use crate::config::{ElementConfig, Location};
use crate::error::AppError;

pub mod callsign;
pub mod clock;
pub mod map;

pub use callsign::Callsign;
pub use clock::Clock;
pub use map::Map;

/// Globals available to every element at construction time. Currently just the
/// user's home location; designed to grow (units, language, etc.) without
/// breaking element APIs.
#[derive(Debug, Clone, Default)]
pub struct Globals {
    pub home: Option<Location>,
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
        other => Err(AppError::UnknownElement(other.into())),
    }
}
