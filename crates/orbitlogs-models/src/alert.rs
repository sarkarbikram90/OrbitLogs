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

use crate::{ConjunctionRiskTier, ErrorCode, SatelliteId, SecondaryObjectType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            trace_id: None,
        }
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

pub fn load_secret(env_var: &str) -> Option<String> {
    if let Ok(val) = std::env::var(env_var) {
        if !val.trim().is_empty() {
            return Some(val.trim().to_string());
        }
    }
    if let Ok(file_path) = std::env::var(format!("{}_FILE", env_var)) {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            return Some(content.trim().to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConjunctionEvent {
    pub conjunction_id: Uuid,
    pub primary_satellite_id: SatelliteId,
    pub secondary_object_id: String,
    pub secondary_object_type: SecondaryObjectType,
    pub tca: DateTime<Utc>,
    pub miss_distance_km: f64,
    pub radial_miss_m: f64,
    pub intrack_miss_m: f64,
    pub crosstrack_miss_m: f64,
    pub collision_probability: f64,
    pub relative_velocity_kms: f64,
    pub risk_tier: ConjunctionRiskTier,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionImpactSummary {
    pub propellant_cost_kg: f64,
    pub payload_downtime_minutes: u32,
    pub post_burn_orbit_apogee_km: f64,
    pub post_burn_orbit_perigee_km: f64,
    pub downlink_window_disrupted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvoidanceManeuverRecommendation {
    pub recommendation_id: Uuid,
    pub conjunction_id: Uuid,
    pub satellite_id: SatelliteId,
    pub burn_execution_window_start: DateTime<Utc>,
    pub delta_v_vector: [f64; 3],
    pub post_burn_miss_distance_km: f64,
    pub post_burn_pc: f64,
    pub mission_impact: MissionImpactSummary,
    pub recommended_action: String,
    pub rationale: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TleElements {
    pub inclination: f64,
    pub raan: f64,
    pub eccentricity: f64,
    pub arg_of_perigee: f64,
    pub mean_anomaly: f64,
    pub mean_motion: f64,
}

impl TleElements {
    pub fn parse(line2: &str) -> Self {
        let inclination = if line2.len() >= 16 {
            line2[8..16].trim().parse::<f64>().unwrap_or(53.0)
        } else {
            53.0
        };
        let raan = if line2.len() >= 25 {
            line2[17..25].trim().parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        };
        let ecc = if line2.len() >= 33 {
            let ecc_str = line2[26..33].trim();
            format!("0.{}", ecc_str).parse::<f64>().unwrap_or(0.0001)
        } else {
            0.0001
        };
        let arg_of_perigee = if line2.len() >= 42 {
            line2[34..42].trim().parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        };
        let mean_anomaly = if line2.len() >= 51 {
            line2[43..51].trim().parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        };
        let mean_motion = if line2.len() >= 63 {
            line2[52..63].trim().parse::<f64>().unwrap_or(15.0)
        } else {
            15.0
        };

        Self {
            inclination,
            raan,
            eccentricity: ecc,
            arg_of_perigee,
            mean_anomaly,
            mean_motion,
        }
    }
}
