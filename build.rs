//! Embeds the current git commit (with a "-dirty" suffix when the working
//! tree has uncommitted changes) into the `GIT_COMMIT` env var, available
//! at compile-time via `env!("GIT_COMMIT")`. Used by `--version`.
//!
//! Falls back to "unknown" when git is unavailable or this isn't a checkout.

use std::process::Command;

fn main() {
    let commit = git_short_hash().unwrap_or_else(|| "unknown".to_string());
    let suffix = if commit != "unknown" && is_dirty() {
        "-dirty"
    } else {
        ""
    };
    println!("cargo:rustc-env=GIT_COMMIT={commit}{suffix}");

    // Re-run when HEAD or refs move (e.g. new commit on the current branch).
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rerun-if-changed=.git/refs");
}

fn git_short_hash() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn is_dirty() -> bool {
    let Ok(out) = Command::new("git").args(["status", "--porcelain"]).output() else {
        return false;
    };
    out.status.success() && !out.stdout.is_empty()
}
