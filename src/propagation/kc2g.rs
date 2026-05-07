//! KC2G prop.kc2g.com point-to-point predictions.
//!
//! `/api/ptp.json?from_grid=lat,lon&to_grid=lat,lon` returns a forecast
//! series — one entry per ~hour, each with short-path and long-path MUF/LUF
//! computed by the upstream ray-tracer. The endpoint accepts a Maidenhead
//! grid OR a `"lat,lon"` string for both ends; we always send `"lat,lon"`.

use crate::geo::LatLon;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct PathPrediction {
    pub forecast_time: DateTime<Utc>,
    pub muf_sp: f32,
    pub luf_sp: f32,
    /// Long-path MUF/LUF — kept for completeness; the UI currently shows
    /// short-path only.
    #[allow(dead_code)]
    pub muf_lp: f32,
    #[allow(dead_code)]
    pub luf_lp: f32,
}

#[derive(Debug, Deserialize)]
struct RawSample {
    ts: i64,
    metrics: Metrics,
}

#[derive(Debug, Deserialize)]
struct Metrics {
    muf_sp: f32,
    luf_sp: f32,
    muf_lp: f32,
    luf_lp: f32,
}

pub fn fetch_path(from: LatLon, to: LatLon) -> Result<Vec<PathPrediction>> {
    let agent = ureq::AgentBuilder::new()
        .timeout(TIMEOUT)
        .user_agent("wjmclock/0.1")
        .build();

    let url = format!(
        "https://prop.kc2g.com/api/ptp.json?from_grid={:.4},{:.4}&to_grid={:.4},{:.4}",
        from.lat, from.lon, to.lat, to.lon
    );

    let body: Vec<RawSample> = agent
        .get(&url)
        .call()
        .with_context(|| format!("GET {url}"))?
        .into_json()
        .context("parse ptp.json")?;

    if body.is_empty() {
        return Err(anyhow!("ptp.json returned an empty series"));
    }

    Ok(body
        .into_iter()
        .map(|s| PathPrediction {
            forecast_time: Utc.timestamp_opt(s.ts, 0).single().unwrap_or_else(Utc::now),
            muf_sp: s.metrics.muf_sp,
            luf_sp: s.metrics.luf_sp,
            muf_lp: s.metrics.muf_lp,
            luf_lp: s.metrics.luf_lp,
        })
        .collect())
}

/// Pick the prediction whose forecast time is nearest to `now`.
pub fn nearest(series: &[PathPrediction], now: DateTime<Utc>) -> Option<&PathPrediction> {
    series.iter().min_by_key(|p| {
        (p.forecast_time.timestamp() - now.timestamp())
            .checked_abs()
            .unwrap_or(i64::MAX)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pred(ts_secs: i64, muf: f32) -> PathPrediction {
        PathPrediction {
            forecast_time: Utc.timestamp_opt(ts_secs, 0).single().unwrap(),
            muf_sp: muf,
            luf_sp: 5.0,
            muf_lp: muf,
            luf_lp: 5.0,
        }
    }

    #[test]
    fn nearest_picks_closest() {
        let series = [pred(1000, 10.0), pred(2000, 20.0), pred(3000, 30.0)];
        let now = Utc.timestamp_opt(2100, 0).single().unwrap();
        let p = nearest(&series, now).unwrap();
        assert_eq!(p.muf_sp, 20.0);
    }

    #[test]
    fn nearest_handles_empty() {
        assert!(nearest(&[], Utc::now()).is_none());
    }
}
