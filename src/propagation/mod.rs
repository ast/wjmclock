//! Background-fetched HF propagation snapshot.
//!
//! `PropagationService::start` spawns one detached worker thread that polls
//! NOAA SWPC + KC2G once per hour, updates an `Arc<Mutex<...>>` snapshot,
//! and pokes the egui context to repaint. The UI thread only reads the
//! snapshot (microsecond lock) — no network I/O ever runs on the UI thread.

use crate::geo::LatLon;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub mod bands;
pub mod kc2g;
pub mod noaa;

pub use kc2g::PathPrediction;
pub use noaa::SolarIndices;

const REFRESH_INTERVAL: Duration = Duration::from_secs(3600);

/// One named target for path prediction (typically a `[[marker]]`).
#[derive(Debug, Clone)]
pub struct Target {
    pub name: String,
    pub coord: LatLon,
}

/// Result of one path fetch — either a prediction series or a fetch error.
#[derive(Debug, Clone)]
pub struct PathSeries {
    pub name: String,
    pub series: Vec<PathPrediction>,
}

#[derive(Debug, Clone, Default)]
pub struct PropagationSnapshot {
    pub fetched_at: Option<DateTime<Utc>>,
    pub solar: Option<SolarIndices>,
    pub paths: Vec<PathSeries>,
    pub last_error: Option<String>,
}

pub struct PropagationService {
    snapshot: Arc<Mutex<PropagationSnapshot>>,
}

impl PropagationService {
    /// Spawn the worker thread and return a handle. The thread is detached;
    /// the OS reclaims it when the process exits.
    pub fn start(home: LatLon, targets: Vec<Target>, ctx: egui::Context) -> Self {
        let snapshot = Arc::new(Mutex::new(PropagationSnapshot::default()));
        let snap_for_worker = Arc::clone(&snapshot);

        std::thread::Builder::new()
            .name("wjmclock-propagation".into())
            .spawn(move || worker_loop(home, targets, snap_for_worker, ctx))
            .expect("spawn propagation worker");

        Self { snapshot }
    }

    /// Snapshot of the latest fetched data. Cheap (clone under a brief lock).
    pub fn snapshot(&self) -> PropagationSnapshot {
        self.snapshot.lock().unwrap().clone()
    }
}

fn worker_loop(
    home: LatLon,
    targets: Vec<Target>,
    snapshot: Arc<Mutex<PropagationSnapshot>>,
    ctx: egui::Context,
) {
    loop {
        match fetch_all(home, &targets) {
            Ok((solar, paths)) => {
                let mut s = snapshot.lock().unwrap();
                s.fetched_at = Some(Utc::now());
                s.solar = Some(solar);
                s.paths = paths;
                s.last_error = None;
            }
            Err(e) => {
                let mut s = snapshot.lock().unwrap();
                s.last_error = Some(format!("{e:#}"));
                // Keep previous solar/paths so the UI can still render
                // stale values; bump fetched_at only on success.
            }
        }
        ctx.request_repaint();
        std::thread::sleep(REFRESH_INTERVAL);
    }
}

fn fetch_all(home: LatLon, targets: &[Target]) -> anyhow::Result<(SolarIndices, Vec<PathSeries>)> {
    // NOAA: solar indices.
    let solar = noaa::fetch()?;

    // KC2G: per-target path series. Failure on one target shouldn't kill
    // the whole snapshot — record an empty series and continue.
    let paths = targets
        .iter()
        .map(|t| {
            let series = kc2g::fetch_path(home, t.coord).unwrap_or_default();
            PathSeries {
                name: t.name.clone(),
                series,
            }
        })
        .collect();

    Ok((solar, paths))
}
