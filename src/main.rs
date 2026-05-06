mod app;
mod cli;
mod color;
mod config;
mod elements;
mod error;
mod geo;
mod layout;
mod textures;

use anyhow::Context;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
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
