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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use orbitlogs_models::GroundStationHealth as GroundStationState;
pub use orbitlogs_models::HealthState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubsystemHealth {
    pub state: HealthState,
    pub score: i32,
    pub explanation: String,
}

pub fn lerp(val: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    if in_max == in_min {
        return out_min;
    }
    let t = (val - in_min) / (in_max - in_min);
    let t = t.clamp(0.0, 1.0);
    out_min + t * (out_max - out_min)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteHealthAssessment {
    pub timestamp: DateTime<Utc>,
    pub satellite_id: orbitlogs_models::SatelliteId,
    pub mission_id: orbitlogs_models::MissionId,
    pub overall_state: HealthState,
    pub overall_score: i32,
    pub power: SubsystemHealth,
    pub thermal: SubsystemHealth,
    pub communications: SubsystemHealth,
    pub propulsion: SubsystemHealth,
    pub adcs: SubsystemHealth,
    pub payload: SubsystemHealth,
    pub flight_computer: SubsystemHealth,
    pub explanation: String,
}
