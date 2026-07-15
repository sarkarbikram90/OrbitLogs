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
use crate::coordinates::{calculate_look_angles, ecef_to_enu, eci_to_ecef, geodetic_to_ecef};
use chrono::{DateTime, Utc};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use sgp4::{Constants, Elements};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AstrodynamicsError {
    #[error("SGP4 propagation error: {0}")]
    Sgp4Error(String),
}

impl From<sgp4::Error> for AstrodynamicsError {
    fn from(err: sgp4::Error) -> Self {
        AstrodynamicsError::Sgp4Error(format!("{:?}", err))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TleRecord {
    pub satellite_id: String,
    pub epoch: DateTime<Utc>,
    pub line1: String,
    pub line2: String,
}

impl TleRecord {
    /// Validates the lines and parses Keplerian components using the SGP4 crate.
    pub fn to_elements(&self) -> Result<Elements, AstrodynamicsError> {
        let elements = Elements::from_tle(
            Some(self.satellite_id.clone()),
            self.line1.as_bytes(),
            self.line2.as_bytes(),
        )?;
        Ok(elements)
    }

    /// Propagates the orbit to a target time, returning position and velocity in ECI (TEME) frame in km and km/s.
    pub fn propagate_to(
        &self,
        target_time: DateTime<Utc>,
    ) -> Result<([f64; 3], [f64; 3]), AstrodynamicsError> {
        let elements = self.to_elements()?;
        let constants = Constants::from_elements(&elements)?;

        // Calculate minutes since epoch
        let delta_seconds = target_time.signed_duration_since(self.epoch).num_seconds() as f64;
        let delta_minutes = delta_seconds / 60.0;

        let prediction = constants.propagate(delta_minutes)?;

        Ok((prediction.position, prediction.velocity))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactWindow {
    pub satellite_id: String,
    pub ground_station_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub max_elevation: f32,
}

/// Predicts contact windows between a ground station and a satellite.
pub fn predict_contact_windows(
    tle: &TleRecord,
    station_id: &str,
    station_lat: f64,
    station_lon: f64,
    station_alt: f64,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Vec<ContactWindow> {
    let mut windows = Vec::new();
    let mut step = start;
    let mut active_window: Option<ContactWindow> = None;

    // Station coordinates in ECEF (meters)
    let station_ecef = geodetic_to_ecef(station_lat, station_lon, station_alt);

    while step < end {
        if let Ok((pos_eci, _)) = tle.propagate_to(step) {
            // Convert ECI (km) to meters for standard WGS84 coordinate transformations
            let pos_eci_m = Vector3::new(
                pos_eci[0] * 1000.0,
                pos_eci[1] * 1000.0,
                pos_eci[2] * 1000.0,
            );
            let pos_ecef = eci_to_ecef(pos_eci_m, step);
            let enu = ecef_to_enu(pos_ecef, station_lat, station_lon, station_ecef);
            let look = calculate_look_angles(&enu);

            if look.elevation_deg > 10.0 {
                // 10 degree elevation mask
                if let Some(ref mut win) = active_window {
                    win.end_time = step;
                    if look.elevation_deg > win.max_elevation as f64 {
                        win.max_elevation = look.elevation_deg as f32;
                    }
                } else {
                    active_window = Some(ContactWindow {
                        satellite_id: tle.satellite_id.clone(),
                        ground_station_id: station_id.to_string(),
                        start_time: step,
                        end_time: step,
                        max_elevation: look.elevation_deg as f32,
                    });
                }
            } else if let Some(win) = active_window.take() {
                windows.push(win);
            }
        }

        step = step + chrono::Duration::seconds(30); // 30-second steps
    }

    if let Some(win) = active_window {
        windows.push(win);
    }

    windows
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_iss_tle_propagation() {
        // ISS TLE from epoch 2020-351.52044816
        let record = TleRecord {
            satellite_id: "ISS".to_string(),
            epoch: Utc.with_ymd_and_hms(2020, 12, 16, 12, 29, 27).unwrap(),
            line1: "1 25544U 98067A   20351.52044816  .00001099  00000-0  28236-4 0  9997"
                .to_string(),
            line2: "2 25544  51.6453 293.7432 0001229  93.1819  69.7547 15.49168434259798"
                .to_string(),
        };

        let elements_res = record.to_elements();
        assert!(
            elements_res.is_ok(),
            "Failed to parse elements: {:?}",
            elements_res.err()
        );

        // Propagate 10 minutes forward
        let target = record.epoch + chrono::Duration::minutes(10);
        let prop_res = record.propagate_to(target);
        assert!(
            prop_res.is_ok(),
            "Failed to propagate: {:?}",
            prop_res.err()
        );

        let (pos, vel) = prop_res.unwrap();
        // Position coordinates in km should be non-zero
        assert!(pos[0] != 0.0);
        assert!(pos[1] != 0.0);
        assert!(pos[2] != 0.0);

        // Velocity coordinates in km/s should be non-zero
        assert!(vel[0] != 0.0);
        assert!(vel[1] != 0.0);
        assert!(vel[2] != 0.0);
    }
}
