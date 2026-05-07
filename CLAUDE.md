# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project intent

`wjmclock` is a modern, Rust-based reimplementation of [HamClock](https://www.hamclock.com/) — a desktop "war-room" style display showing a clock alongside a world map with a day/night terminator (gray line), markers, and HF propagation overlays. Visual reference: `docs/screenshot.png`.

Target deployment is a Raspberry Pi 4 driving a 1920×1080 screen, but the layout scales to other resolutions via fractional rects.

## Stack

- **CLI:** `clap` v4 (`derive`) + `clap_complete`
- **Errors:** `thiserror` v2 + `anyhow` v1
- **UI:** `eframe` / `egui` v0.34 (features: `default_fonts`, `glow`, `wayland`, `x11`)
- **Config:** `serde` + `toml`
- **Time:** `chrono` + `chrono-tz`
- **Geo:** `geojson` for coastlines; custom Maidenhead decoder and solar-position math (NOAA ESRL)
- **Network:** `ureq` v2 (`json` + `tls`, no defaults) for hourly propagation fetches on a worker thread
- **Parsing:** `nom` v7 (currently used for hex color parsing)
- **Images:** `image` v0.25 (PNG only) — assets are `include_bytes!`'d
- **Paths:** `directories` v6 for XDG config

Prefer these before adding alternatives. Rust edition is `2024`.

## Architecture

### Module layout (`src/`)

- `main.rs`, `cli.rs`, `app.rs` — entry, CLI, eframe loop (Esc / Ctrl+Q quit, F11 fullscreen)
- `config.rs` — TOML loading, CLI overrides, location resolution (Maidenhead or lat/lon)
- `error.rs` — `AppError` enum
- `layout.rs` — fractional rect (0..1) → pixel rect resolver
- `color.rs` — hex color parser (`#rgb` / `#rrggbb` / `#rrggbbaa`) with serde integration
- `textures.rs` — bundled PNG decode
- `geo/` — `LatLon`, `Projection` trait (Equirectangular), `Subsolar` / terminator math, Maidenhead decoder, coastline loader
- `elements/` — `Element` trait + `make_element()` factory, plus `clock`, `callsign`, `map`, `propagation`
- `propagation/` — background `PropagationService` thread, NOAA SWPC + KC2G MUF/LUF fetchers, band tables

### Element trait (extension point)

```rust
pub trait Element {
    fn update(&mut self, ctx: &egui::Context);
    fn ui(&mut self, ui: &mut egui::Ui);
}
```

Adding an element: implement `Element` + a `from_toml(toml::Value)` constructor, add a file under `elements/`, and add a match arm in `make_element()` (`elements/mod.rs`). Elements are positioned via `FractionalRect` and configured per-element in TOML.

### Projection trait

`geo::Projection::project(rect, LatLon) -> Pos2`. Currently only `Equirectangular`; add new projections by implementing the trait and extending the map config.

### Threading model

- UI runs on eframe's main thread.
- Long-running / network work runs on a named worker thread (see `propagation/mod.rs`): spawn detached, share state via `Arc<Mutex<Snapshot>>`, and call `egui::Context::request_repaint()` after updates.
- Per-frame repaint cadence is throttled with `ctx.request_repaint_after(...)` (clock: 1s, map: 60s, propagation: 60s).
- No channels yet. New stream-style background producers (events rather than periodic snapshots) should prefer `crossbeam-channel` or `std::sync::mpsc`.

### Config

TOML resolution order: `--config <path>` → `$XDG_CONFIG_HOME/wjmclock/wjmclock.toml` → `./wjmclock.toml` → built-in defaults. See `wjmclock.example.toml` for the full schema (window, home, markers, elements).

## Design constraints

- **Modularity:** generally one struct per file. Keep modules small so new elements / projections / data sources can be added without touching unrelated code.
- **Performance target:** must run smoothly on a Pi 4. Avoid per-frame allocations and heavy redraws; prefer cached / precomputed data (the map terminator uses a fixed 192×96 mesh; coastlines and textures are embedded and decoded once).
- **Data-driven:** every visual aspect (colors, timezones, projections, element visibility, layout) is TOML-configurable. Don't hard-code layouts or styling.

## Common commands

```bash
cargo build                  # debug build
cargo build --release        # release build (use this when measuring Pi performance)
cargo run -- <args>          # run with CLI args
cargo test                   # run all tests
cargo test <name>            # run a single test by substring match
cargo fmt
cargo clippy -- -D warnings  # lint, treat warnings as errors
```

Cross-compilation for aarch64 (Pi 4 / 5) is wired via `Cross.toml` and the `justfile`.
