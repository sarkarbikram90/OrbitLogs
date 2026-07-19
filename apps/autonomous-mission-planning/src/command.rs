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

/// Flight safety parameters & battery constants
pub const EXECUTE_BURN_COST: f32 = 25.0; // Battery percentage drop for thruster actuators
pub const LOAD_SHED_RECOVERY: f32 = 5.0; // Battery percentage saved from load shedding
pub const BASE_SOLAR_RECOVERY: f32 = 2.0; // Base battery percentage gained from solar orientation

/// High-level spacecraft commands.
///
/// Every variant represents a valid onboard command sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    ShedSecondaryLoad,

    AdjustRadiatorYaw {
        angle_degrees: f32,
    },

    OrientSolarPanels {
        heading_degrees: f32,
    },

    ExecuteBurn {
        vector: [f64; 3],
    },
}

impl Command {
    /// Returns the net battery SoC delta (%) resulting from command execution,
    /// factoring in hardware efficiency parameters.
    pub fn battery_delta(&self, solar_efficiency: f32) -> f32 {
        match self {
            Self::ExecuteBurn { .. } => -EXECUTE_BURN_COST,

            Self::ShedSecondaryLoad => LOAD_SHED_RECOVERY,

            Self::OrientSolarPanels { heading_degrees } => {
                // Efficiency scaling based on sun vector alignment (0-180 deg) and solar degradation
                let alignment_factor = (heading_degrees.to_radians().cos().abs()).max(0.1);
                BASE_SOLAR_RECOVERY * solar_efficiency * alignment_factor
            }

            Self::AdjustRadiatorYaw { .. } => 0.0,
        }
    }
}
