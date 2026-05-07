use clap::Parser;
use clap_complete::Shell;
use std::path::PathBuf;

/// "0.1.0 (a1b2c3d)" or "0.1.0 (a1b2c3d-dirty)". `GIT_COMMIT` is set by build.rs.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_COMMIT"), ")");

#[derive(Debug, Parser)]
#[command(
    name = "wjmclock",
    about = "Modern HamClock-style war-room display",
    version = VERSION
)]
pub struct Cli {
    /// Path to a TOML config file. Defaults to <config-dir>/wjmclock/wjmclock.toml then ./wjmclock.toml.
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Window width in logical pixels (overrides config).
    #[arg(long)]
    pub width: Option<u32>,

    /// Window height in logical pixels (overrides config).
    #[arg(long)]
    pub height: Option<u32>,

    /// Start in fullscreen (overrides config).
    #[arg(long)]
    pub fullscreen: bool,

    /// Hide the mouse cursor (kiosk mode).
    #[arg(long)]
    pub no_cursor: bool,

    /// Print a shell completion script and exit.
    /// Use as: source <(wjmclock --completion zsh)
    #[arg(long, value_name = "SHELL")]
    pub completion: Option<Shell>,
}
