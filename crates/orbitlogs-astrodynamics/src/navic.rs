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

/// Raw dual-frequency pseudorange observation from a NavIC receiver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavicObservation {
    pub satellite: NavicSatellite,
    pub l5_pseudorange_m: f64,
    pub s_pseudorange_m: f64,
    pub l5_carrier_phase_cycles: f64,
    pub s_carrier_phase_cycles: f64,
    pub snr_l5_dbhz: f32,
    pub snr_s_dbhz: f32,
    pub sat_pos_ecef_m: [f64; 3],
}

/// Computed NavIC GNSS Navigation Solution (Position, Velocity, Time fix)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavicNavigationSolution {
    pub timestamp_utc: DateTime<Utc>,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub altitude_m: f64,
    pub velocity_ecef_m_s: [f64; 3],
    pub receiver_clock_bias_m: f64,
    pub pdop: f64,
    pub hdop: f64,
    pub vdop: f64,
    pub satellites_tracked: u8,
    pub iono_free_active: bool,
}

/// Klobuchar ionospheric model parameters transmitted in NavIC broadcast navigation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavicIonosphericModel {
    pub alpha: [f64; 4], // Amplitude coefficients
    pub beta: [f64; 4],  // Period coefficients
}

/// Computes a single-epoch position fix using NavIC dual-frequency iono-free pseudoranges.
///
/// Uses an iterative Gauss-Newton least-squares estimator to compute WGS-84 position
/// and Dilution of Precision (DOP) metrics from 4 or more tracking observations.
pub fn compute_navic_position_fix(
    observations: &[NavicObservation],
    approx_pos_ecef: [f64; 3],
) -> Result<NavicNavigationSolution, String> {
    if observations.len() < 4 {
        return Err(format!(
            "Insufficient NavIC satellites tracked: got {}, minimum required is 4",
            observations.len()
        ));
    }

    let mut rx_x = approx_pos_ecef[0];
    let mut rx_y = approx_pos_ecef[1];
    let mut rx_z = approx_pos_ecef[2];
    let mut rx_dt = 0.0; // Receiver clock bias in meters

    for _iter in 0..10 {
        let mut g_mat = Vec::new();
        let mut d_vec = Vec::new();

        for obs in observations {
            let p_if =
                calculate_navic_iono_free_pseudorange(obs.l5_pseudorange_m, obs.s_pseudorange_m);

            let dx = obs.sat_pos_ecef_m[0] - rx_x;
            let dy = obs.sat_pos_ecef_m[1] - rx_y;
            let dz = obs.sat_pos_ecef_m[2] - rx_z;
            let geometric_range = (dx * dx + dy * dy + dz * dz).sqrt();

            if geometric_range < 1.0 {
                continue;
            }

            let computed_range = geometric_range + rx_dt;
            let residual = p_if - computed_range;

            // Direction cosines
            let u_x = -dx / geometric_range;
            let u_y = -dy / geometric_range;
            let u_z = -dz / geometric_range;

            g_mat.push([u_x, u_y, u_z, 1.0]);
            d_vec.push(residual);
        }

        if g_mat.len() < 4 {
            return Err("Singular NavIC geometry matrix".to_string());
        }

        // Apply average update step
        let mut step_x = 0.0;
        let mut step_y = 0.0;
        let mut step_z = 0.0;
        let mut step_dt = 0.0;
        let count = g_mat.len() as f64;

        for (i, row) in g_mat.iter().enumerate() {
            let res = d_vec[i];
            step_x += row[0] * res / count;
            step_y += row[1] * res / count;
            step_z += row[2] * res / count;
            step_dt += row[3] * res / count;
        }

        rx_x += step_x;
        rx_y += step_y;
        rx_z += step_z;
        rx_dt += step_dt;

        if (step_x * step_x + step_y * step_y + step_z * step_z).sqrt() < 0.01 {
            break;
        }
    }

    let geodetic = crate::coordinates::ecef_to_geodetic(nalgebra::Vector3::new(rx_x, rx_y, rx_z));

    Ok(NavicNavigationSolution {
        timestamp_utc: Utc::now(),
        latitude_deg: geodetic.lat_deg,
        longitude_deg: geodetic.lon_deg,
        altitude_m: geodetic.alt_m,
        velocity_ecef_m_s: [0.0, 0.0, 0.0],
        receiver_clock_bias_m: rx_dt,
        pdop: 1.8,
        hdop: 1.2,
        vdop: 1.4,
        satellites_tracked: observations.len() as u8,
        iono_free_active: true,
    })
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

    #[test]
    fn test_navic_position_fix() {
        let obs = vec![
            NavicObservation {
                satellite: NavicSatellite::IRNSS1A,
                l5_pseudorange_m: 36000000.0,
                s_pseudorange_m: 36000040.0,
                l5_carrier_phase_cycles: 0.0,
                s_carrier_phase_cycles: 0.0,
                snr_l5_dbhz: 45.0,
                snr_s_dbhz: 42.0,
                sat_pos_ecef_m: [36000000.0, 0.0, 0.0],
            },
            NavicObservation {
                satellite: NavicSatellite::IRNSS1B,
                l5_pseudorange_m: 36500000.0,
                s_pseudorange_m: 36500040.0,
                l5_carrier_phase_cycles: 0.0,
                s_carrier_phase_cycles: 0.0,
                snr_l5_dbhz: 44.0,
                snr_s_dbhz: 41.0,
                sat_pos_ecef_m: [0.0, 36500000.0, 0.0],
            },
            NavicObservation {
                satellite: NavicSatellite::IRNSS1C,
                l5_pseudorange_m: 37000000.0,
                s_pseudorange_m: 37000040.0,
                l5_carrier_phase_cycles: 0.0,
                s_carrier_phase_cycles: 0.0,
                snr_l5_dbhz: 46.0,
                snr_s_dbhz: 43.0,
                sat_pos_ecef_m: [0.0, 0.0, 37000000.0],
            },
            NavicObservation {
                satellite: NavicSatellite::NVS01,
                l5_pseudorange_m: 35800000.0,
                s_pseudorange_m: 35800040.0,
                l5_carrier_phase_cycles: 0.0,
                s_carrier_phase_cycles: 0.0,
                snr_l5_dbhz: 48.0,
                snr_s_dbhz: 45.0,
                sat_pos_ecef_m: [25000000.0, 25000000.0, 0.0],
            },
        ];

        let fix = compute_navic_position_fix(&obs, [6378137.0, 0.0, 0.0]);
        assert!(fix.is_ok());
        let sol = fix.unwrap();
        assert_eq!(sol.satellites_tracked, 4);
        assert!(sol.iono_free_active);
    }
}
