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

/// Structured safety constraint violations for flight telemetry and audit logging.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyViolation {
    BatteryDepleted {
        timestamp: DateTime<Utc>,
        battery_level: f32,
        min_safe_threshold: f32,
    },
}

impl std::fmt::Display for SafetyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BatteryDepleted {
                timestamp,
                battery_level,
                min_safe_threshold,
            } => {
                write!(
                    f,
                    "Constraint violated at {timestamp}: battery level ({battery_level:.1}%) dropped below safe operating threshold ({min_safe_threshold:.1}%).",
                )
            }
        }
    }
}

impl std::error::Error for SafetyViolation {}
