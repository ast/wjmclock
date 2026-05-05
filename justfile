# wjmclock — task runner. `just` (no args) lists recipes.

target := "aarch64-unknown-linux-gnu"

# List all recipes.
default:
    @just --list --unsorted

# ─── Local development ──────────────────────────────────────

# Debug build.
build:
    cargo build

# Release build.
release:
    cargo build --release

# Run the app, optionally with extra CLI args. Uses the repo's example config.
run *args:
    cargo run -- --config wjmclock.example.toml {{args}}

# Unit tests.
test:
    cargo test

# Format all sources in place.
fmt:
    cargo fmt --all

# Verify formatting without rewriting (CI-style).
fmt-check:
    cargo fmt --all -- --check

# Clippy, treating warnings as errors.
lint:
    cargo clippy --all-targets -- -D warnings

# fmt-check + lint + tests, the full local gate.
check: fmt-check lint test

# ─── Cross-compile for Raspberry Pi 4/5 (aarch64) ──────────

# Cross-compile a release binary for the Pi.
cross:
    cross build --release --target {{target}}

# Cross-compile a debug binary (quicker turnaround during iteration).
cross-debug:
    cross build --target {{target}}

# Cross-build, then scp the binary + config to the Pi (default host `shack`; override with `just deploy other-host`).
deploy host="shack": cross
    scp target/{{target}}/release/wjmclock {{host}}:./wjmclock
    ssh {{host}} mkdir -p .config/wjmclock
    scp ~/.config/wjmclock/wjmclock.toml {{host}}:.config/wjmclock/wjmclock.toml
    @echo "→ deployed to {{host}}: ~/wjmclock + ~/.config/wjmclock/wjmclock.toml"

# ─── Misc ───────────────────────────────────────────────────

# Remove build artifacts (host + cross targets).
clean:
    cargo clean
