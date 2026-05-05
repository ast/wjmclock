//! Maidenhead Locator System decoder.
//!
//! Decodes 2/4/6/8-character grid locators (e.g. "JN58td", "JO99cd11")
//! to a `LatLon` placed at the *centre* of the smallest specified cell.
//!
//! Encoding scheme:
//!   chars 1-2: field    A..R, 20° lon × 10° lat
//!   chars 3-4: square   0..9, 2°  lon × 1°  lat
//!   chars 5-6: subsq.   a..x, 5'  lon × 2.5' lat
//!   chars 7-8: ext.     0..9, 30" lon × 15"  lat

use crate::geo::LatLon;
use anyhow::{Result, bail};

pub fn decode(locator: &str) -> Result<LatLon> {
    let s = locator.trim();
    let len = s.len();
    if !(len == 2 || len == 4 || len == 6 || len == 8) {
        bail!("locator must be 2, 4, 6, or 8 chars: {locator:?}");
    }
    let chars: Vec<char> = s.chars().collect();

    // Field (always present).
    let f1 = chars[0].to_ascii_uppercase();
    let f2 = chars[1].to_ascii_uppercase();
    if !('A'..='R').contains(&f1) || !('A'..='R').contains(&f2) {
        bail!("field chars must be A..R, got {f1}{f2}");
    }
    let mut lon = (f1 as i32 - b'A' as i32) as f64 * 20.0 - 180.0;
    let mut lat = (f2 as i32 - b'A' as i32) as f64 * 10.0 - 90.0;
    let mut lon_step = 20.0;
    let mut lat_step = 10.0;

    if len >= 4 {
        let s1 = chars[2];
        let s2 = chars[3];
        if !s1.is_ascii_digit() || !s2.is_ascii_digit() {
            bail!("square chars must be 0..9, got {s1}{s2}");
        }
        lon += s1.to_digit(10).unwrap() as f64 * 2.0;
        lat += s2.to_digit(10).unwrap() as f64;
        lon_step = 2.0;
        lat_step = 1.0;
    }

    if len >= 6 {
        let ss1 = chars[4].to_ascii_lowercase();
        let ss2 = chars[5].to_ascii_lowercase();
        if !('a'..='x').contains(&ss1) || !('a'..='x').contains(&ss2) {
            bail!("subsquare chars must be a..x, got {ss1}{ss2}");
        }
        lon += (ss1 as i32 - b'a' as i32) as f64 * (5.0 / 60.0);
        lat += (ss2 as i32 - b'a' as i32) as f64 * (2.5 / 60.0);
        lon_step = 5.0 / 60.0;
        lat_step = 2.5 / 60.0;
    }

    if len >= 8 {
        let e1 = chars[6];
        let e2 = chars[7];
        if !e1.is_ascii_digit() || !e2.is_ascii_digit() {
            bail!("extended chars must be 0..9, got {e1}{e2}");
        }
        lon += e1.to_digit(10).unwrap() as f64 * (30.0 / 3600.0);
        lat += e2.to_digit(10).unwrap() as f64 * (15.0 / 3600.0);
        lon_step = 30.0 / 3600.0;
        lat_step = 15.0 / 3600.0;
    }

    // Centre within the smallest specified cell.
    lon += lon_step / 2.0;
    lat += lat_step / 2.0;

    Ok(LatLon {
        lat: lat as f32,
        lon: lon as f32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn jn58_centre_munich() {
        // JN58 covers 10–12°E, 48–49°N. Centre ≈ 11°E, 48.5°N.
        let p = decode("JN58").unwrap();
        assert!(approx(p.lon, 11.0, 1e-3), "lon={}", p.lon);
        assert!(approx(p.lat, 48.5, 1e-3), "lat={}", p.lat);
    }

    #[test]
    fn jo99cd_near_stockholm() {
        // JO99cd ≈ 18.21°E, 59.146°N (south-east of central Stockholm).
        let p = decode("JO99cd").unwrap();
        assert!(approx(p.lon, 18.208, 0.05), "lon={}", p.lon);
        assert!(approx(p.lat, 59.146, 0.05), "lat={}", p.lat);
    }

    #[test]
    fn case_insensitive() {
        let a = decode("jn58TD").unwrap();
        let b = decode("JN58td").unwrap();
        assert_eq!(a.lat, b.lat);
        assert_eq!(a.lon, b.lon);
    }

    #[test]
    fn aa00_southwest_corner() {
        // AA at field level is 180°W, 90°S; centre of AA is (-170°, -85°).
        let p = decode("AA").unwrap();
        assert!(approx(p.lon, -170.0, 1e-3));
        assert!(approx(p.lat, -85.0, 1e-3));
    }

    #[test]
    fn rejects_bad_lengths() {
        assert!(decode("J").is_err());
        assert!(decode("JN5").is_err());
        assert!(decode("JN58t").is_err());
    }

    #[test]
    fn rejects_bad_chars() {
        assert!(decode("ZZ").is_err());
        assert!(decode("JNAB").is_err());
        assert!(decode("JN58zz").is_err());
    }
}
