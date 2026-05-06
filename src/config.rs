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
    #[serde(default, rename = "marker")]
    pub markers: Vec<MarkerConfig>,
    #[serde(default, rename = "element")]
    pub elements: Vec<ElementConfig>,
}

/// One map marker, settable as either a Maidenhead grid locator or raw lat/lon.
/// Resolved to a `Marker` via `MarkerConfig::resolve` after parsing.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkerConfig {
    /// Maidenhead grid locator, e.g. "JN58td".
    pub locator: Option<String>,
    /// Raw latitude in degrees (-90..=90). Used if `locator` is absent.
    pub lat: Option<f32>,
    /// Raw longitude in degrees (-180..=180). Used if `locator` is absent.
    pub lon: Option<f32>,
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
        let coord = if let Some(loc) = &self.locator {
            maidenhead::decode(loc)
                .map_err(|e| AppError::InvalidLocation(format!("locator {loc:?}: {e}")))?
        } else if let (Some(lat), Some(lon)) = (self.lat, self.lon) {
            if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
                return Err(AppError::InvalidLocation(format!(
                    "lat/lon out of range: {lat}, {lon}"
                )));
            }
            LatLon { lat, lon }
        } else {
            return Err(AppError::InvalidLocation(
                "set either `locator` or both `lat` and `lon`".into(),
            ));
        };
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

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            fullscreen: false,
            no_cursor: false,
            background: default_background(),
        }
    }
}

/// One TOML `[[element]]` entry. The element-specific keys are captured in
/// `extra` and parsed by the element constructor.
#[derive(Debug, Deserialize)]
pub struct ElementConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub rect: FractionalRect,
    #[serde(flatten)]
    pub extra: toml::Table,
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

/// Default layout used when no config is found: clock top-left, map below.
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
            rect: FractionalRect {
                x: 0.0,
                y: 0.0,
                w: 0.45,
                h: 0.25,
            },
            extra: clock,
        },
        ElementConfig {
            kind: "map".into(),
            rect: FractionalRect {
                x: 0.0,
                y: 0.25,
                w: 1.0,
                h: 0.75,
            },
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
            rect = { x = 0.0, y = 0.0, w = 1.0, h = 0.2 }
            timezone = "UTC"
            format = "24h"
        "#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.window.width, 800);
        assert_eq!(cfg.elements.len(), 1);
        assert_eq!(cfg.elements[0].kind, "clock");
    }
}
