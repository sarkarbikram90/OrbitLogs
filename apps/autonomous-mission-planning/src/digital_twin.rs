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

use serde::{Deserialize, Serialize};

/// Snapshot of spacecraft health used by the mission planner.
#[derive(Debug, Serialize, Deserialize)]
pub struct DigitalTwinState {
    pub satellite_id: String,
    pub solar_efficiency: f32,
    pub battery_capacity_fade: f32,
    pub radiator_thermal_resistance: f32,
}

impl DigitalTwinState {
    /// Returns a demonstration digital twin state.
    ///
    /// Production deployments retrieve this from real-time vehicle telemetry.
    pub fn demo() -> Self {
        Self {
            satellite_id: "SAT-0001".to_string(),
            solar_efficiency: 0.95,
            battery_capacity_fade: 0.90,
            radiator_thermal_resistance: 1.05,
        }
    }
}
