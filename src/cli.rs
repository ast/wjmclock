use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "wjmclock",
    about = "Modern HamClock-style war-room display",
    version
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
}
