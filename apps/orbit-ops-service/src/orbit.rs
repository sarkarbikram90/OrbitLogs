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

//! Orbital mechanics, Keplerian elements, and SGP4 propagation algorithms.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

// Re-export persistence/messaging operations for backward compatibility
pub use crate::orbit_service::{
    check_tle_expiration, get_latest_tle_or_default, import_tle_internal, scrape_celestrak_tles,
    ImportTleRequest,
};

pub const EARTH_RADIUS_KM: f64 = 6378.137;
pub const GM_KM3_S2: f64 = 398600.4418; // G * Earth Mass
pub const EARTH_ROTATION_RAD_S: f64 = 7.29211510e-5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbitState {
    pub satellite_id: String,
    pub timestamp: DateTime<Utc>,
    pub position_eci: [f64; 3], // km
    pub velocity_eci: [f64; 3], // km/s
    pub latitude: f64,          // degrees
    pub longitude: f64,         // degrees
    pub altitude: f64,          // km
    pub velocity: f64,          // km/s
    pub orbital_period: f64,    // minutes
    pub semi_major_axis: f64,   // km
    pub eccentricity: f64,
    pub inclination: f64,    // degrees
    pub raan: f64,           // degrees
    pub arg_of_perigee: f64, // degrees
    pub mean_anomaly: f64,   // degrees
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassPrediction {
    pub station: String,
    pub aos: DateTime<Utc>,
    pub los: DateTime<Utc>,
    pub duration: f64,      // minutes
    pub max_elevation: f64, // degrees
}

#[derive(Debug, Clone)]
pub struct GroundStationInfo {
    pub name: &'static str,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_km: f64,
}

pub fn get_ground_stations() -> Vec<GroundStationInfo> {
    vec![
        GroundStationInfo {
            name: "Svalbard",
            latitude: 78.2297,
            longitude: 15.6469,
            altitude_km: 0.5,
        },
        GroundStationInfo {
            name: "Troll",
            latitude: -72.0119,
            longitude: 2.5330,
            altitude_km: 1.3,
        },
        GroundStationInfo {
            name: "Kiruna",
            latitude: 67.8558,
            longitude: 20.9629,
            altitude_km: 0.4,
        },
        GroundStationInfo {
            name: "McMurdo",
            latitude: -77.8463,
            longitude: 166.6682,
            altitude_km: 0.1,
        },
        GroundStationInfo {
            name: "Bangalore",
            latitude: 13.0300,
            longitude: 77.5600,
            altitude_km: 0.9,
        },
        GroundStationInfo {
            name: "Hyderabad",
            latitude: 17.3850,
            longitude: 78.4867,
            altitude_km: 0.5,
        },
        GroundStationInfo {
            name: "Delhi",
            latitude: 28.6139,
            longitude: 77.2090,
            altitude_km: 0.20,
        },
        GroundStationInfo {
            name: "Siliguri",
            latitude: 26.7271,
            longitude: 88.3953,
            altitude_km: 0.11,
        },
    ]
}

// Generates dynamic deterministic TLE lines for SAT-0001 through SAT-0010
pub fn generate_default_tle(sat_id: &str) -> (String, String) {
    let idx = sat_id
        .strip_prefix("SAT-")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    let sat_num = 25500 + idx;
    let inc = 53.0 + (idx as f64 * 3.7) % 45.0;
    let raan = (idx as f64 * 36.0) % 360.0;
    let ecc_val = 0.0005 + (idx as f64 * 0.0001) % 0.01;
    let ecc_int = (ecc_val * 10000000.0) as i32;
    let arg_pe = (idx as f64 * 45.0) % 360.0;
    let mean_anom = (idx as f64 * 72.0) % 360.0;
    let mean_motion = 15.0 + (idx as f64 * 0.1) % 1.5;

    let line1 = format!(
        "1 {:05}U 26001A   26170.00000000  .00002182  00000-0  10000-3 0  9993",
        sat_num
    );
    let line2 = format!(
        "2 {:05} {:8.4} {:8.4} {:07} {:8.4} {:8.4} {:11.8}    1",
        sat_num, inc, raan, ecc_int, arg_pe, mean_anom, mean_motion
    );
    (line1, line2)
}

// Propagates TLE parameters using standard SGP4 model
pub fn propagate_tle_sgp4(
    sat_id: &str,
    line1: &str,
    line2: &str,
    time: DateTime<Utc>,
) -> Result<OrbitState, String> {
    let elements =
        sgp4::Elements::from_tle(Some(sat_id.to_string()), line1.as_bytes(), line2.as_bytes())
            .map_err(|e| format!("SGP4 parse error: {:?}", e))?;

    let constants = sgp4::Constants::from_elements(&elements)
        .map_err(|e| format!("SGP4 constants error: {:?}", e))?;

    let epoch_time = DateTime::<Utc>::from_naive_utc_and_offset(elements.datetime, Utc);
    let diff_ms = (time - epoch_time).num_milliseconds() as f64;
    let minutes = diff_ms / 60000.0;

    let prediction = constants
        .propagate(minutes)
        .map_err(|e| format!("SGP4 propagation error: {:?}", e))?;

    let position_eci = prediction.position;
    let velocity_eci = prediction.velocity;

    let dt_seconds = diff_ms / 1000.0;
    let theta = (EARTH_ROTATION_RAD_S * dt_seconds) % (2.0 * PI);
    let mut lon = position_eci[1].atan2(position_eci[0]) - theta;
    lon %= 2.0 * PI;
    if lon < -PI {
        lon += 2.0 * PI;
    } else if lon > PI {
        lon -= 2.0 * PI;
    }

    let r_xy = (position_eci[0].powi(2) + position_eci[1].powi(2)).sqrt();
    let lat = position_eci[2].atan2(r_xy);
    let alt = (position_eci[0].powi(2) + position_eci[1].powi(2) + position_eci[2].powi(2)).sqrt()
        - EARTH_RADIUS_KM;

    let inc = line2[8..16].trim().parse::<f64>().unwrap_or(53.0);
    let raan = line2[17..25].trim().parse::<f64>().unwrap_or(0.0);
    let ecc_str = line2[26..33].trim();
    let ecc = format!("0.{}", ecc_str).parse::<f64>().unwrap_or(0.0001);
    let arg_pe = line2[34..42].trim().parse::<f64>().unwrap_or(0.0);
    let mean_anom = line2[43..51].trim().parse::<f64>().unwrap_or(0.0);
    let mean_motion = line2[52..63].trim().parse::<f64>().unwrap_or(15.0);

    let mean_motion_rad_s = mean_motion * 2.0 * PI / 86400.0;
    let semi_major_axis = (GM_KM3_S2 / mean_motion_rad_s.powi(2)).cbrt();
    let orbital_period = 86400.0 / mean_motion / 60.0;

    let vel = (velocity_eci[0].powi(2) + velocity_eci[1].powi(2) + velocity_eci[2].powi(2)).sqrt();

    Ok(OrbitState {
        satellite_id: sat_id.to_string(),
        timestamp: time,
        position_eci,
        velocity_eci,
        latitude: lat * 180.0 / PI,
        longitude: lon * 180.0 / PI,
        altitude: alt,
        velocity: vel,
        orbital_period,
        semi_major_axis,
        eccentricity: ecc,
        inclination: inc,
        raan,
        arg_of_perigee: arg_pe,
        mean_anomaly: mean_anom,
    })
}

// Calculate slant range, elevation from Ground Station coordinates
pub fn calculate_elevation(
    sat_lat: f64,
    sat_lon: f64,
    sat_alt_km: f64,
    gs: &GroundStationInfo,
) -> f64 {
    let lat_s = sat_lat * PI / 180.0;
    let lon_s = sat_lon * PI / 180.0;
    let lat_g = gs.latitude * PI / 180.0;
    let lon_g = gs.longitude * PI / 180.0;

    let r_s = EARTH_RADIUS_KM + sat_alt_km;
    let r_g = EARTH_RADIUS_KM + gs.altitude_km;

    let cos_theta = lat_g.sin() * lat_s.sin() + lat_g.cos() * lat_s.cos() * (lon_s - lon_g).cos();
    let theta = cos_theta.acos();

    let d2 = r_g.powi(2) + r_s.powi(2) - 2.0 * r_g * r_s * cos_theta;
    let d = d2.sqrt();

    if d < 1e-3 {
        return 90.0;
    }

    let sin_el = (r_s * theta.sin()) / d;
    let cos_el = (r_s * cos_theta - r_g) / d;
    let el = sin_el.atan2(cos_el);
    el * 180.0 / PI
}

// Compute upcoming passes for the next 24 hours
pub fn predict_passes(
    sat_id: &str,
    line1: &str,
    line2: &str,
    start_time: DateTime<Utc>,
    duration_hours: i64,
) -> Vec<PassPrediction> {
    let stations = get_ground_stations();
    let mut passes = Vec::new();

    let total_steps = duration_hours * 120;

    for gs in &stations {
        let mut in_pass = false;
        let mut aos_time = start_time;
        let mut max_el = 0.0;

        for step in 0..total_steps {
            let t = start_time + chrono::Duration::seconds(step * 30);
            if let Ok(state) = propagate_tle_sgp4(sat_id, line1, line2, t) {
                let el = calculate_elevation(state.latitude, state.longitude, state.altitude, gs);

                if el > 5.0 {
                    if !in_pass {
                        in_pass = true;
                        aos_time = t;
                        max_el = el;
                    } else if el > max_el {
                        max_el = el;
                    }
                } else {
                    if in_pass {
                        in_pass = false;
                        let los_time = t;
                        let duration_min = (los_time - aos_time).num_seconds() as f64 / 60.0;
                        if duration_min > 1.0 {
                            passes.push(PassPrediction {
                                station: gs.name.to_string(),
                                aos: aos_time,
                                los: los_time,
                                duration: duration_min,
                                max_elevation: max_el,
                            });
                        }
                        max_el = 0.0;
                    }
                }
            }
        }
    }

    passes.sort_by_key(|p| p.aos);
    passes
}
