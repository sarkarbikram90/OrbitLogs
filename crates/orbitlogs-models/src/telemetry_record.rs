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
use uuid::Uuid;
use crate::{MissionId, SatelliteId, Subsystem};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryRecord {
    pub satellite_id: SatelliteId,

    #[serde(default)]
    pub subsystem_id: Subsystem,

    #[serde(default)]
    pub mission_id: MissionId,

    pub timestamp: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_voltage: Option<f64>, // Volts

    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_current: Option<f64>, // Amperes

    #[serde(skip_serializing_if = "Option::is_none")]
    pub solar_panel_output: Option<f64>, // Watts

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_usage: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>, // °C

    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_pitch: Option<f32>, // Degrees

    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_roll: Option<f32>, // Degrees

    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_yaw: Option<f32>, // Degrees

    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<f64>, // km/s

    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>, // km

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_remaining: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication_latency: Option<f32>, // ms

    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_strength: Option<f32>, // dBm

    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_power_draw: Option<f64>, // Watts

    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_active: Option<bool>,

    pub event_id: Uuid,
}

impl TelemetryRecord {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.satellite_id.as_str().trim().is_empty() {
            errors.push("satellite_id cannot be empty".to_string());
        } else if !self
            .satellite_id
            .as_str()
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-')
        {
            errors.push("satellite_id must be alphanumeric or contain dashes".to_string());
        }

        if let Some(battery_voltage) = self.battery_voltage {
            if !(0.0..=100.0).contains(&battery_voltage) {
                errors.push(format!(
                    "battery_voltage must be between 0 and 100, got {}",
                    battery_voltage
                ));
            }
        }

        if let Some(cpu_usage) = self.cpu_usage {
            if !(0.0..=100.0).contains(&cpu_usage) {
                errors.push(format!(
                    "cpu_usage must be between 0 and 100, got {}",
                    cpu_usage
                ));
            }
        }

        if let Some(memory_usage) = self.memory_usage {
            if !(0.0..=100.0).contains(&memory_usage) {
                errors.push(format!(
                    "memory_usage must be between 0 and 100, got {}",
                    memory_usage
                ));
            }
        }

        if let Some(fuel_remaining) = self.fuel_remaining {
            if !(0.0..=100.0).contains(&fuel_remaining) {
                errors.push(format!(
                    "fuel_remaining must be between 0 and 100, got {}",
                    fuel_remaining
                ));
            }
        }

        if let Some(packet_loss) = self.packet_loss {
            if !(0.0..=100.0).contains(&packet_loss) {
                errors.push(format!(
                    "packet_loss must be between 0 and 100, got {}",
                    packet_loss
                ));
            }
        }

        if let Some(orientation_pitch) = self.orientation_pitch {
            if !(-360.0..=360.0).contains(&orientation_pitch) {
                errors.push(format!(
                    "orientation_pitch must be between -360 and 360, got {}",
                    orientation_pitch
                ));
            }
        }

        if let Some(orientation_roll) = self.orientation_roll {
            if !(-360.0..=360.0).contains(&orientation_roll) {
                errors.push(format!(
                    "orientation_roll must be between -360 and 360, got {}",
                    orientation_roll
                ));
            }
        }

        if let Some(orientation_yaw) = self.orientation_yaw {
            if !(-360.0..=360.0).contains(&orientation_yaw) {
                errors.push(format!(
                    "orientation_yaw must be between -360 and 360, got {}",
                    orientation_yaw
                ));
            }
        }

        if self.timestamp > Utc::now() + chrono::Duration::minutes(5) {
            errors.push("Timestamp cannot be ahead of current time".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
