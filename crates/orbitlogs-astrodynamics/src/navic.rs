// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! IRNSS / NavIC (Indian Regional Navigation Satellite System) GNSS support.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// NavIC / IRNSS Frequency Constants (MHz)
pub const NAVIC_L5_FREQ_MHZ: f64 = 1176.45;
pub const NAVIC_S_FREQ_MHZ: f64 = 2492.028;
pub const NAVIC_L1_FREQ_MHZ: f64 = 1575.42;

/// Speed of Light in vacuum (m/s)
pub const SPEED_OF_LIGHT_M_S: f64 = 299792458.0;

/// IRNSS Time (IRNT) Epoch: August 22, 1999 00:00:00 UTC
pub const IRNT_EPOCH_UNIX_TIMESTAMP: i64 = 935270400;

/// Active NavIC Spacecraft Segment Identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavicSatellite {
    IRNSS1A, // GEO 55° E
    IRNSS1B, // GSO 55° E
    IRNSS1C, // GEO 83° E
    IRNSS1D, // GSO 111.75° E
    IRNSS1E, // GSO 111.75° E
    IRNSS1F, // GEO 32.5° E
    IRNSS1G, // GEO 129.5° E
    IRNSS1I, // GSO 55° E
    NVS01,   // Next Gen NavIC L1
}

impl NavicSatellite {
    pub fn name(&self) -> &'static str {
        match self {
            NavicSatellite::IRNSS1A => "NavIC-1A",
            NavicSatellite::IRNSS1B => "NavIC-1B",
            NavicSatellite::IRNSS1C => "NavIC-1C",
            NavicSatellite::IRNSS1D => "NavIC-1D",
            NavicSatellite::IRNSS1E => "NavIC-1E",
            NavicSatellite::IRNSS1F => "NavIC-1F",
            NavicSatellite::IRNSS1G => "NavIC-1G",
            NavicSatellite::IRNSS1I => "NavIC-1I",
            NavicSatellite::NVS01 => "NVS-01",
        }
    }

    pub fn slot_longitude_deg(&self) -> f64 {
        match self {
            NavicSatellite::IRNSS1A | NavicSatellite::IRNSS1B | NavicSatellite::IRNSS1I => 55.0,
            NavicSatellite::IRNSS1C => 83.0,
            NavicSatellite::IRNSS1D | NavicSatellite::IRNSS1E => 111.75,
            NavicSatellite::IRNSS1F => 32.5,
            NavicSatellite::IRNSS1G => 129.5,
            NavicSatellite::NVS01 => 55.0,
        }
    }

    pub fn is_geo(&self) -> bool {
        matches!(
            self,
            NavicSatellite::IRNSS1A
                | NavicSatellite::IRNSS1C
                | NavicSatellite::IRNSS1F
                | NavicSatellite::IRNSS1G
        )
    }
}

/// Converts IRNSS Time (IRNT) Week Number (WN) and Time-of-Week (TOW in seconds)
/// to standard Utc DateTime.
///
/// # Arguments
/// * `wn` - IRNT Week Number
/// * `tow` - Time of Week in seconds (0.0 .. 604800.0)
/// * `leap_seconds` - Current UTC leap seconds offset (default 18s)
pub fn irnt_to_utc(wn: u16, tow: f64, leap_seconds: i64) -> DateTime<Utc> {
    let total_seconds = (wn as i64 * 604800) + tow as i64 - leap_seconds;
    let unix_time = IRNT_EPOCH_UNIX_TIMESTAMP + total_seconds;
    Utc.timestamp_opt(unix_time, 0).unwrap()
}

/// Computes dual-frequency ionosphere-free pseudorange combination
/// for NavIC L5 (1176.45 MHz) and S-band (2492.028 MHz).
///
/// Eliminates first-order ionospheric delay for high-precision spacecraft positioning.
pub fn calculate_navic_iono_free_pseudorange(p_l5_meters: f64, p_s_meters: f64) -> f64 {
    let f_l5_sq = NAVIC_L5_FREQ_MHZ.powi(2);
    let f_s_sq = NAVIC_S_FREQ_MHZ.powi(2);

    let gamma = f_l5_sq / f_s_sq;
    (p_s_meters - gamma * p_l5_meters) / (1.0 - gamma)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_irnt_to_utc() {
        // WN 1000, TOW 0s, 18 leap seconds
        let dt = irnt_to_utc(1000, 0.0, 18);
        assert_eq!(dt.year(), 2018);
    }

    #[test]
    fn test_iono_free_pseudorange() {
        let l5 = 36000000.0;
        let s = 36000050.0;
        let p_if = calculate_navic_iono_free_pseudorange(l5, s);
        assert!(p_if > 35000000.0 && p_if < 37000000.0);
    }
}
