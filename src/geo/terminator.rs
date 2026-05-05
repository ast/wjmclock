//! Subsolar point + day/twilight/night illumination.
//!
//! Implements the NOAA ESRL solar position formulas (good to ~0.5° — plenty
//! for a visual gray-line). All angles in radians inside the module; degrees
//! at the public boundary.

use chrono::{DateTime, Datelike, Timelike, Utc};
use std::f32::consts::PI;

/// The point on Earth where the sun is directly overhead at `time`.
#[derive(Debug, Clone, Copy)]
pub struct Subsolar {
    /// Latitude in degrees (-23.45..=23.45 over the year).
    pub lat: f32,
    /// Longitude in degrees (-180..=180).
    pub lon: f32,
    /// Cached solar declination in radians.
    pub decl: f32,
}

impl Subsolar {
    pub fn at(time: DateTime<Utc>) -> Self {
        // Fractional year, with hour-of-day correction.
        let doy = time.ordinal() as f32; // 1..=366
        let hour = time.hour() as f32 + (time.minute() as f32) / 60.0;
        let gamma = (2.0 * PI / 365.0) * (doy - 1.0 + (hour - 12.0) / 24.0);

        // Equation of time, minutes.
        let eot_min = 229.18
            * (0.000_075 + 0.001_868 * gamma.cos()
                - 0.032_077 * gamma.sin()
                - 0.014_615 * (2.0 * gamma).cos()
                - 0.040_849 * (2.0 * gamma).sin());

        // Declination, radians.
        let decl = 0.006_918 - 0.399_912 * gamma.cos() + 0.070_257 * gamma.sin()
            - 0.006_758 * (2.0 * gamma).cos()
            + 0.000_907 * (2.0 * gamma).sin()
            - 0.002_697 * (3.0 * gamma).cos()
            + 0.001_48 * (3.0 * gamma).sin();

        // Subsolar longitude: where true solar time = 12:00.
        let utc_hours = hour;
        let mut lon = -15.0 * (utc_hours + eot_min / 60.0 - 12.0);
        lon = wrap_lon(lon);

        Self {
            lat: decl.to_degrees(),
            lon,
            decl,
        }
    }

    /// Solar elevation angle (degrees) at a given location.
    pub fn elevation_at(&self, lat_deg: f32, lon_deg: f32) -> f32 {
        let lat = lat_deg.to_radians();
        // Hour angle: difference between observer's longitude and subsolar longitude.
        let h = wrap_lon(lon_deg - self.lon).to_radians();
        let sin_elev = lat.sin() * self.decl.sin() + lat.cos() * self.decl.cos() * h.cos();
        sin_elev.clamp(-1.0, 1.0).asin().to_degrees()
    }
}

fn wrap_lon(mut lon: f32) -> f32 {
    while lon > 180.0 {
        lon -= 360.0;
    }
    while lon < -180.0 {
        lon += 360.0;
    }
    lon
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// Around the March equinox at noon UTC, the subsolar point should sit near
    /// (0°, 0°) — within a few degrees.
    #[test]
    fn equinox_noon_near_origin() {
        let t = Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap();
        let s = Subsolar::at(t);
        assert!(s.lat.abs() < 1.5, "lat={}", s.lat);
        assert!(s.lon.abs() < 5.0, "lon={}", s.lon);
    }

    /// June solstice at noon UTC: subsolar latitude near +23.4°.
    #[test]
    fn june_solstice_tropic_of_cancer() {
        let t = Utc.with_ymd_and_hms(2025, 6, 21, 12, 0, 0).unwrap();
        let s = Subsolar::at(t);
        assert!((s.lat - 23.4).abs() < 0.5, "lat={}", s.lat);
    }

    /// At the subsolar point itself, solar elevation is ~90°.
    #[test]
    fn elevation_at_subsolar_is_overhead() {
        let t = Utc.with_ymd_and_hms(2025, 6, 21, 12, 0, 0).unwrap();
        let s = Subsolar::at(t);
        let e = s.elevation_at(s.lat, s.lon);
        assert!((e - 90.0).abs() < 0.5, "e={}", e);
    }
}
