mod app;
mod cli;
mod color;
mod config;
mod elements;
mod error;
mod geo;
mod layout;
mod propagation;
mod textures;

use anyhow::Context;
use clap::{CommandFactory, Parser};

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    // `--completion <SHELL>` is a side-effect-free "print and exit" mode.
    // Handle it before any config load or eframe init so it's cheap.
    if let Some(shell) = cli.completion {
        let mut cmd = cli::Cli::command();
        clap_complete::generate(shell, &mut cmd, "wjmclock", &mut std::io::stdout());
        return Ok(());
    }

    let cfg = config::Config::load(&cli).context("load config")?;

    let viewport = egui::ViewportBuilder::default()
        .with_title("wjmclock")
        .with_inner_size([cfg.window.width as f32, cfg.window.height as f32])
        .with_fullscreen(cfg.window.fullscreen);

    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "wjmclock",
        native_options,
        Box::new(|cc| {
            let app = app::App::new(cc, cfg)
                .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))?;
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))?;
    Ok(())
}
