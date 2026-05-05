# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project intent

`wjmclock` is a modern, Rust-based reimplementation of [HamClock](https://www.hamclock.com/) — a desktop "war-room" style display showing a clock alongside a world map with a day/night terminator (gray line). Visual reference: `docs/ham-clock.png`.

Target deployment is a Raspberry Pi 4 driving a 1920×1080 screen, but the layout must scale to other resolutions.

The project is in its earliest stage: `src/main.rs` is currently the cargo `Hello, world!` template and `Cargo.toml` lists no dependencies yet. Treat the items below as design constraints rather than as descriptions of existing code.

## Design constraints

- **Stack:** `clap` (CLI), `thiserror` + `anyhow` (errors), `egui` (UI). Prefer these before adding alternatives.
- **Configuration:** runtime configurable via CLI args *and* a TOML config file. The TOML config must be able to add, remove, and configure individual UI elements (clock, map, etc.) — i.e. elements are data-driven, not hard-coded.
- **Modularity:** generally one struct per file. Keep modules small and composable so new UI elements can be added without touching unrelated code.
- **Performance target:** must run smoothly on a Pi 4. Avoid per-frame allocations and heavy redraws; prefer cached/precomputed data for the map and terminator.

## Common commands

```bash
cargo build              # debug build
cargo build --release    # release build (use this when measuring Pi performance)
cargo run -- <args>      # run with CLI args
cargo test               # run all tests
cargo test <name>        # run a single test by substring match
cargo fmt                # format
cargo clippy -- -D warnings   # lint, treat warnings as errors
```

The Rust edition is `2024` (see `Cargo.toml`), which requires a sufficiently recent toolchain.
