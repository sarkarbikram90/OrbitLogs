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

use crate::command::Command;
use crate::digital_twin::DigitalTwinState;
use crate::error::SafetyViolation;
use chrono::{DateTime, Utc};

pub const MAX_BATTERY_PERCENT: f32 = 100.0;
pub const MIN_SAFE_BATTERY: f32 = 20.0; // Minimum allowable SoC threshold (%)

/// Default demo schedule offsets (minutes from evaluation epoch).
pub const LOAD_SHED_DELAY_MINUTES: i64 = 5;
pub const RADIATOR_DELAY_MINUTES: i64 = 15;
pub const EXECUTE_BURN_DELAY_MINUTES: i64 = 30;

pub type ScheduledCommand = (DateTime<Utc>, Command);

/// Ordered sequence of spacecraft commands scheduled for onboard execution.
pub struct OnboardTimeline {
    pub schedule: Vec<ScheduledCommand>,
}

impl OnboardTimeline {
    pub fn new(schedule: Vec<ScheduledCommand>) -> Self {
        Self { schedule }
    }

    /// Simulates command timeline execution and verifies battery state of charge (SoC)
    /// safety constraints under degradation models.
    pub fn validate_safety_constraints(
        &self,
        twin: &DigitalTwinState,
    ) -> Result<(), SafetyViolation> {
        // Upper bound capacity ceiling based on battery degradation (capacity fade)
        let max_usable_capacity = MAX_BATTERY_PERCENT * twin.battery_capacity_fade.clamp(0.1, 1.0);
        let mut battery = max_usable_capacity;

        for (timestamp, command) in &self.schedule {
            let delta = command.battery_delta(twin.solar_efficiency);

            // Energy conservation law: battery SoC cannot exceed maximum physical capacity ceiling
            battery = (battery + delta).min(max_usable_capacity);

            if battery < MIN_SAFE_BATTERY {
                return Err(SafetyViolation::BatteryDepleted {
                    timestamp: *timestamp,
                    battery_level: battery,
                    min_safe_threshold: MIN_SAFE_BATTERY,
                });
            }
        }

        Ok(())
    }
}

/// Constructs a demonstration mission timeline.
pub fn build_demo_timeline(now: DateTime<Utc>) -> OnboardTimeline {
    OnboardTimeline::new(vec![
        (
            now + chrono::Duration::minutes(LOAD_SHED_DELAY_MINUTES),
            Command::ShedSecondaryLoad,
        ),
        (
            now + chrono::Duration::minutes(RADIATOR_DELAY_MINUTES),
            Command::AdjustRadiatorYaw {
                angle_degrees: 45.0,
            },
        ),
        (
            now + chrono::Duration::minutes(EXECUTE_BURN_DELAY_MINUTES),
            Command::ExecuteBurn {
                vector: [1.2, 0.0, 0.0],
            },
        ),
    ])
}
