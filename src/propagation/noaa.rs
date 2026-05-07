//! NOAA Space Weather Prediction Center (SWPC) JSON fetcher.
//!
//! Pulls the two indices the propagation widget needs:
//!   * F10.7 cm radio flux (a.k.a. SFI), 24-hour data
//!   * Planetary K-index, 1-minute estimates
//!
//! Both endpoints return arrays of dicts. Conventions in the wild:
//! `f107_cm_flux.json` is newest-first; `planetary_k_index_1m.json` is
//! oldest-first. We don't trust the order — we sort by `time_tag` and take
//! the latest valid sample.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::time::Duration;

const SFI_URL: &str = "https://services.swpc.noaa.gov/json/f107_cm_flux.json";
const KP_URL: &str = "https://services.swpc.noaa.gov/json/planetary_k_index_1m.json";
const TIMEOUT: Duration = Duration::from_secs(20);

/// What the propagation widget needs from NOAA.
#[derive(Debug, Clone, Copy)]
pub struct SolarIndices {
    pub sfi: f32,
    pub k_index: f32,
}

#[derive(Debug, Deserialize)]
struct SfiSample {
    time_tag: String,
    flux: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct KpSample {
    time_tag: String,
    kp_index: Option<f32>,
}

pub fn fetch() -> Result<SolarIndices> {
    let agent = ureq::AgentBuilder::new()
        .timeout(TIMEOUT)
        .user_agent("wjmclock/0.1 (+https://github.com/)")
        .build();

    let sfi = fetch_sfi(&agent).context("fetch SFI")?;
    let k_index = fetch_kp(&agent).context("fetch K-index")?;
    Ok(SolarIndices { sfi, k_index })
}

fn fetch_sfi(agent: &ureq::Agent) -> Result<f32> {
    let body: Vec<SfiSample> = agent.get(SFI_URL).call()?.into_json()?;
    body.into_iter()
        .filter(|s| s.flux.is_some())
        .max_by(|a, b| a.time_tag.cmp(&b.time_tag))
        .and_then(|s| s.flux)
        .ok_or_else(|| anyhow!("no SFI samples in response"))
}

fn fetch_kp(agent: &ureq::Agent) -> Result<f32> {
    let body: Vec<KpSample> = agent.get(KP_URL).call()?.into_json()?;
    body.into_iter()
        .filter(|s| s.kp_index.is_some())
        .max_by(|a, b| a.time_tag.cmp(&b.time_tag))
        .and_then(|s| s.kp_index)
        .ok_or_else(|| anyhow!("no Kp samples in response"))
}
