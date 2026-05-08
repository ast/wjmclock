use crate::cli::Cli;
use crate::color::Color;
use crate::error::AppError;
use crate::geo::{LatLon, maidenhead};
use chrono_tz::Tz;
use directories::ProjectDirs;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub window: WindowConfig,
    /// The station QTH. Drawn on the map like any other marker, and reused
    /// by the propagation widget as the "from" end of path predictions.
    #[serde(default)]
    pub home: Option<MarkerConfig>,
    #[serde(default, rename = "marker")]
    pub markers: Vec<MarkerConfig>,
    #[serde(default, rename = "element")]
    pub elements: Vec<ElementConfig>,
}

/// Marker location: either a bare Maidenhead locator string or an inline
/// `{ lat, lon }` table. Serde dispatches by input shape.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LocationConfig {
    Locator(String),
    LatLon(LatLonConfig),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatLonConfig {
    pub lat: f32,
    pub lon: f32,
}

impl LocationConfig {
    fn resolve(&self) -> Result<LatLon, AppError> {
        match self {
            Self::Locator(s) => maidenhead::decode(s)
                .map_err(|e| AppError::InvalidLocation(format!("locator {s:?}: {e}"))),
            Self::LatLon(LatLonConfig { lat, lon }) => {
                if !(-90.0..=90.0).contains(lat) || !(-180.0..=180.0).contains(lon) {
                    return Err(AppError::InvalidLocation(format!(
                        "lat/lon out of range: {lat}, {lon}"
                    )));
                }
                Ok(LatLon {
                    lat: *lat,
                    lon: *lon,
                })
            }
        }
    }
}

/// One map marker. Resolved to a `Marker` via `MarkerConfig::resolve`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkerConfig {
    /// `"JO67AQ"` (locator string) or `{ lat = 57.7, lon = 12.0 }` (inline table).
    pub location: LocationConfig,
    /// Display text drawn next to the marker.
    pub text: String,
    /// Visual style. Currently only `"dot"` is supported.
    #[serde(default = "default_marker_kind")]
    pub kind: String,
    /// Optional IANA timezone (e.g. "Europe/Stockholm"). When set, the
    /// marker's local time is drawn beneath its text.
    pub timezone: Option<String>,
}

fn default_marker_kind() -> String {
    "dot".into()
}

#[derive(Debug, Clone, Copy)]
pub enum MarkerKind {
    Dot,
}

#[derive(Debug, Clone)]
pub struct Marker {
    pub coord: LatLon,
    pub text: String,
    pub kind: MarkerKind,
    pub tz: Option<Tz>,
}

impl MarkerConfig {
    pub fn resolve(&self) -> Result<Marker, AppError> {
        let coord = self.location.resolve()?;
        let kind = match self.kind.as_str() {
            "dot" => MarkerKind::Dot,
            other => {
                return Err(AppError::InvalidLocation(format!(
                    "unknown marker kind {other:?} (expected \"dot\")"
                )));
            }
        };
        let tz =
            match &self.timezone {
                Some(s) => Some(s.parse::<Tz>().map_err(|e| {
                    AppError::InvalidLocation(format!("unknown timezone {s:?}: {e}"))
                })?),
                None => None,
            };
        Ok(Marker {
            coord,
            text: self.text.clone(),
            kind,
            tz,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowConfig {
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default)]
    pub no_cursor: bool,
    #[serde(default = "default_background")]
    pub background: Color,
    /// Height of the top panel as a fraction of window height (0..1).
    #[serde(default = "default_top_panel_height")]
    pub top_panel_height: f32,
}

fn default_width() -> u32 {
    1920
}
fn default_height() -> u32 {
    1080
}
fn default_background() -> Color {
    Color::rgb(0x0a, 0x0a, 0x14)
}
fn default_top_panel_height() -> f32 {
    0.22
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            fullscreen: false,
            no_cursor: false,
            background: default_background(),
            top_panel_height: default_top_panel_height(),
        }
    }
}

/// One TOML `[[element]]` entry. The element-specific keys are captured in
/// `extra` and parsed by the element constructor; the slot-positioning keys
/// (`slot`, `align`, `width`, `rect`, `open`, `title`) are consumed here.
#[derive(Debug, Deserialize)]
pub struct ElementConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub slot: SlotKind,
    /// Used by `slot = "top"`. Default left.
    #[serde(default)]
    pub align: TopAlign,
    /// Used by `slot = "top"`. Optional fractional width (0..1 of window width);
    /// when omitted, top elements split their side of the panel equally.
    #[serde(default)]
    pub width: Option<f32>,
    /// Used by `slot = "window"`. Initial pos+size as fractions of the window.
    #[serde(default)]
    pub rect: Option<FractionalRect>,
    /// Used by `slot = "window"`. Whether the window is shown on startup.
    #[serde(default = "default_true")]
    pub open: bool,
    /// Used by `slot = "window"`. Title-bar text.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(flatten)]
    pub extra: toml::Table,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotKind {
    Top,
    Center,
    Window,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TopAlign {
    #[default]
    Left,
    Right,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FractionalRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Config {
    /// Load the config: explicit `--config` path, then platform config dir,
    /// then `./wjmclock.toml`. Missing config returns the default.
    pub fn load(cli: &Cli) -> Result<Self, AppError> {
        let path = resolve_config_path(cli);
        let mut cfg = match path {
            Some(p) if p.exists() => parse_file(&p)?,
            Some(p) if cli.config.is_some() => return Err(AppError::ConfigNotFound(p)),
            _ => Self::default(),
        };
        cfg.apply_cli(cli);
        Ok(cfg)
    }

    fn apply_cli(&mut self, cli: &Cli) {
        if let Some(w) = cli.width {
            self.window.width = w;
        }
        if let Some(h) = cli.height {
            self.window.height = h;
        }
        if cli.fullscreen {
            self.window.fullscreen = true;
        }
        if cli.no_cursor {
            self.window.no_cursor = true;
        }
    }

    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            home: None,
            markers: Vec::new(),
            elements: default_elements(),
        }
    }
}

fn parse_file(path: &Path) -> Result<Config, AppError> {
    let text = std::fs::read_to_string(path).map_err(|source| AppError::ConfigRead {
        path: path.to_path_buf(),
        source,
    })?;
    let mut cfg: Config = toml::from_str(&text).map_err(|source| AppError::ConfigParse {
        path: path.to_path_buf(),
        source,
    })?;
    if cfg.elements.is_empty() {
        cfg.elements = default_elements();
    }
    Ok(cfg)
}

fn resolve_config_path(cli: &Cli) -> Option<PathBuf> {
    if let Some(p) = &cli.config {
        return Some(p.clone());
    }
    if let Some(dirs) = ProjectDirs::from("", "", "wjmclock") {
        let p = dirs.config_dir().join("wjmclock.toml");
        if p.exists() {
            return Some(p);
        }
    }
    let local = PathBuf::from("wjmclock.toml");
    if local.exists() { Some(local) } else { None }
}

/// Default layout used when no config is found: clock pinned top-right,
/// map filling the central panel.
fn default_elements() -> Vec<ElementConfig> {
    let mut clock = toml::Table::new();
    clock.insert("timezone".into(), "UTC".into());
    clock.insert("format".into(), "24h".into());

    let mut map = toml::Table::new();
    map.insert("projection".into(), "equirectangular".into());
    map.insert("terminator".into(), true.into());
    map.insert("night_dim".into(), 0.55.into());

    vec![
        ElementConfig {
            kind: "clock".into(),
            slot: SlotKind::Top,
            align: TopAlign::Right,
            width: None,
            rect: None,
            open: true,
            title: None,
            extra: clock,
        },
        ElementConfig {
            kind: "map".into(),
            slot: SlotKind::Center,
            align: TopAlign::default(),
            width: None,
            rect: None,
            open: true,
            title: None,
            extra: map,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let toml = r#"
            [window]
            width = 800
            height = 600

            [[element]]
            type = "clock"
            slot = "top"
            timezone = "UTC"
            format = "24h"
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.window.width, 800);
        assert_eq!(cfg.elements.len(), 1);
        assert_eq!(cfg.elements[0].kind, "clock");
        assert!(matches!(cfg.elements[0].slot, SlotKind::Top));
    }

    #[test]
    fn parses_top_align_and_width() {
        let toml = r#"
            [[element]]
            type  = "callsign"
            slot  = "top"
            align = "right"
            width = 0.25
            call  = "SM6WJM"
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let el = &cfg.elements[0];
        assert!(matches!(el.slot, SlotKind::Top));
        assert!(matches!(el.align, TopAlign::Right));
        assert_eq!(el.width, Some(0.25));
    }

    #[test]
    fn parses_window_slot() {
        let toml = r#"
            [[element]]
            type  = "propagation"
            slot  = "window"
            title = "Propagation"
            rect  = { x = 0.66, y = 0.55, w = 0.33, h = 0.43 }
            open  = true
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let el = &cfg.elements[0];
        assert!(matches!(el.slot, SlotKind::Window));
        assert!(el.open);
        assert_eq!(el.title.as_deref(), Some("Propagation"));
        let r = el.rect.expect("window rect");
        assert!((r.x - 0.66).abs() < 1e-4);
        assert!((r.h - 0.43).abs() < 1e-4);
    }

    #[test]
    fn rejects_unknown_slot() {
        let toml = r#"
            [[element]]
            type = "clock"
            slot = "bottom"
        "#;
        assert!(toml::from_str::<Config>(toml).is_err());
    }

    #[test]
    fn parses_bundled_example_config() {
        let text = include_str!("../wjmclock.example.toml");
        let cfg: Config = toml::from_str(text).expect("example config must parse");
        assert!(
            cfg.elements
                .iter()
                .any(|e| matches!(e.slot, SlotKind::Center)),
            "example must include a center-slot element"
        );
    }

    fn parse_marker(snippet: &str) -> MarkerConfig {
        toml::from_str(snippet).unwrap()
    }

    #[test]
    fn location_string_is_locator() {
        let m = parse_marker(
            r#"
                location = "JO67AQ"
                text     = "GÖTEBORG"
            "#,
        );
        assert!(matches!(m.location, LocationConfig::Locator(ref s) if s == "JO67AQ"));
    }

    #[test]
    fn location_table_is_latlon() {
        let m = parse_marker(
            r#"
                location = { lat = -5.79, lon = -35.21 }
                text     = "NATAL"
            "#,
        );
        match m.location {
            LocationConfig::LatLon(LatLonConfig { lat, lon }) => {
                assert!((lat - -5.79).abs() < 1e-4);
                assert!((lon - -35.21).abs() < 1e-4);
            }
            other => panic!("expected LatLon, got {other:?}"),
        }
    }

    #[test]
    fn location_table_rejects_unknown_keys() {
        let snippet = r#"
            location = { lat = 0.0, lon = 0.0, foo = 1 }
            text     = "x"
        "#;
        assert!(toml::from_str::<MarkerConfig>(snippet).is_err());
    }

    #[test]
    fn location_rejects_other_shapes() {
        let snippet = r#"
            location = 5
            text     = "x"
        "#;
        assert!(toml::from_str::<MarkerConfig>(snippet).is_err());
    }

    #[test]
    fn resolve_rejects_out_of_range_latlon() {
        let m = parse_marker(
            r#"
                location = { lat = 91.0, lon = 0.0 }
                text     = "x"
            "#,
        );
        let err = m.resolve().unwrap_err().to_string();
        assert!(err.contains("out of range"), "got: {err}");
    }

    #[test]
    fn resolve_surfaces_locator_error() {
        let m = parse_marker(
            r#"
                location = "ZZ99"
                text     = "x"
            "#,
        );
        assert!(m.resolve().is_err());
    }
}
