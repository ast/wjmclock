use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("config file not found: {0}")]
    ConfigNotFound(PathBuf),

    #[error("failed to read config {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse config {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("unknown element type: {0}")]
    UnknownElement(String),

    #[error("invalid element config for {kind}: {source}")]
    ElementConfig {
        kind: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("invalid location: {0}")]
    InvalidLocation(String),

    #[error("more than one element has slot=\"center\"; only one is allowed")]
    MultipleCenterElements,

    #[error("invalid slot config for {kind}: {msg}")]
    InvalidSlot { kind: String, msg: String },
}
