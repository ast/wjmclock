//! HF amateur band table and condition heuristics.
//!
//! The condition derivation is a HamQSL-style heuristic: SFI threshold per
//! band, K-index penalty on the lower bands (auroral absorption), and a
//! day/night flip — high bands favour daylight, low bands favour darkness.
//! These are visual ratings; not a substitute for real propagation prediction.

use std::fmt;

/// One amateur HF band. `freq_mhz` is the canonical "centre of activity"
/// used as a stand-in when comparing against MUF/LUF for a path.
#[derive(Debug, Clone, Copy)]
pub struct Band {
    pub label: &'static str,
    pub freq_mhz: f32,
}

/// All HF amateur bands, ordered low-to-high in frequency.
pub const HF_BANDS: &[Band] = &[
    Band {
        label: "80m",
        freq_mhz: 3.7,
    },
    Band {
        label: "40m",
        freq_mhz: 7.1,
    },
    Band {
        label: "30m",
        freq_mhz: 10.125,
    },
    Band {
        label: "20m",
        freq_mhz: 14.2,
    },
    Band {
        label: "17m",
        freq_mhz: 18.1,
    },
    Band {
        label: "15m",
        freq_mhz: 21.2,
    },
    Band {
        label: "12m",
        freq_mhz: 24.94,
    },
    Band {
        label: "10m",
        freq_mhz: 28.5,
    },
];

/// Visual rating of a band's expected condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rating {
    Good,
    Fair,
    Poor,
}

impl fmt::Display for Rating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rating::Good => f.write_str("Good"),
            Rating::Fair => f.write_str("Fair"),
            Rating::Poor => f.write_str("Poor"),
        }
    }
}

/// Day/night-flavoured rating for a single band.
#[derive(Debug, Clone, Copy)]
pub struct BandRating {
    pub label: &'static str,
    pub day: Rating,
    pub night: Rating,
}

/// Derive a HamQSL-style band-condition table from current solar indices.
///
/// `sfi` — F10.7 cm flux (typical range 60..300).
/// `k_index` — planetary K-index, 0..9. Higher = more disturbed ionosphere.
pub fn derive(sfi: f32, k_index: f32) -> Vec<BandRating> {
    HF_BANDS.iter().map(|b| rate(*b, sfi, k_index)).collect()
}

fn rate(band: Band, sfi: f32, k_index: f32) -> BandRating {
    BandRating {
        label: band.label,
        day: rate_at(band, sfi, k_index, true),
        night: rate_at(band, sfi, k_index, false),
    }
}

/// Heuristic per band/day/night cell.
///
/// Low bands (160/80/40m): geomagnetic storms drive D-layer absorption →
/// K-index dominates. Better at night when D-layer dissolves.
///
/// Mid bands (30/20m): the workhorses; need modest SFI, sensitive to K.
///
/// High bands (17/15/12/10m): F2 layer-driven; need strong SFI to ionise
/// enough for the band to "open"; primarily a daytime phenomenon.
fn rate_at(band: Band, sfi: f32, k_index: f32, is_day: bool) -> Rating {
    let freq = band.freq_mhz;

    // Low bands: K-index drives absorption; nighttime helps a lot.
    if freq < 8.0 {
        let base = if is_day { Rating::Fair } else { Rating::Good };
        return downgrade(base, k_index_penalty(k_index));
    }

    // Mid bands.
    if freq < 16.0 {
        let base = match (sfi, is_day) {
            (s, _) if s >= 90.0 => Rating::Good,
            (s, true) if s >= 70.0 => Rating::Good,
            (_, true) => Rating::Fair,
            (s, false) if s >= 80.0 => Rating::Fair,
            _ => Rating::Poor,
        };
        return downgrade(base, k_index_penalty(k_index));
    }

    // High bands: very SFI-driven, very daytime-favoured.
    let sfi_threshold_good = match band.label {
        "17m" => 85.0,
        "15m" => 100.0,
        "12m" => 120.0,
        "10m" => 140.0,
        _ => 100.0,
    };
    let sfi_threshold_fair = sfi_threshold_good - 25.0;

    let base = match (sfi, is_day) {
        (s, true) if s >= sfi_threshold_good => Rating::Good,
        (s, true) if s >= sfi_threshold_fair => Rating::Fair,
        (s, false) if s >= sfi_threshold_good + 30.0 => Rating::Fair,
        _ => Rating::Poor,
    };
    downgrade(base, k_index_penalty(k_index))
}

/// 0 = no penalty, 1 = down one rung, 2 = down two rungs.
fn k_index_penalty(k: f32) -> u8 {
    if k >= 5.0 {
        2
    } else if k >= 4.0 {
        1
    } else {
        0
    }
}

fn downgrade(r: Rating, steps: u8) -> Rating {
    let mut r = r;
    for _ in 0..steps {
        r = match r {
            Rating::Good => Rating::Fair,
            Rating::Fair => Rating::Poor,
            Rating::Poor => Rating::Poor,
        };
    }
    r
}

/// A single path's MUF/LUF window (e.g., from KC2G ray-tracing). A band is
/// "open" on this path when its frequency sits inside `[luf, muf]`.
pub fn path_open(luf_mhz: f32, muf_mhz: f32, band_freq_mhz: f32) -> bool {
    band_freq_mhz >= luf_mhz && band_freq_mhz <= muf_mhz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_table_is_low_to_high() {
        let mut prev = 0.0;
        for b in HF_BANDS {
            assert!(b.freq_mhz > prev, "{} not increasing", b.label);
            prev = b.freq_mhz;
        }
    }

    #[test]
    fn high_bands_open_when_sfi_is_high() {
        let table = derive(180.0, 1.0);
        let ten = table.iter().find(|b| b.label == "10m").unwrap();
        assert_eq!(ten.day, Rating::Good);
    }

    #[test]
    fn high_bands_closed_when_sfi_is_low() {
        let table = derive(70.0, 1.0);
        let ten = table.iter().find(|b| b.label == "10m").unwrap();
        assert_eq!(ten.day, Rating::Poor);
    }

    #[test]
    fn low_bands_better_at_night() {
        let table = derive(120.0, 1.0);
        let eighty = table.iter().find(|b| b.label == "80m").unwrap();
        assert_eq!(eighty.night, Rating::Good);
        assert_eq!(eighty.day, Rating::Fair);
    }

    #[test]
    fn high_k_degrades_low_bands_to_poor() {
        let table = derive(120.0, 6.0);
        let eighty = table.iter().find(|b| b.label == "80m").unwrap();
        // Night was Good; with K=6 (penalty 2) it should drop to Poor.
        assert_eq!(eighty.night, Rating::Poor);
    }

    #[test]
    fn path_open_window() {
        // 14 MHz inside [8, 18] = open
        assert!(path_open(8.0, 18.0, 14.0));
        // 21 MHz above MUF = closed
        assert!(!path_open(8.0, 18.0, 21.0));
        // 5 MHz below LUF = closed
        assert!(!path_open(8.0, 18.0, 5.0));
        // Boundaries inclusive
        assert!(path_open(8.0, 18.0, 8.0));
        assert!(path_open(8.0, 18.0, 18.0));
    }
}
