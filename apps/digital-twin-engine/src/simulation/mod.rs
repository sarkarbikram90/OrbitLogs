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

//! Health projection and digital twin state modelling.
//!
//! The health projection engine evaluates each spacecraft subsystem against
//! configurable thresholds and produces a structured health report with
//! per-subsystem scores, remaining life expectancy, and actionable
//! operator recommendations.

pub mod assessors;
mod tests;

pub use assessors::{
    ADCS_WARN_PITCH_DRIFT, BATTERY_CRIT_VOLTAGE, BATTERY_WARN_VOLTAGE, COMM_CRIT_PACKET_LOSS,
    COMM_WARN_PACKET_LOSS, FLIGHT_COMPUTER_WARN_CPU, FLIGHT_COMPUTER_WARN_MEM, PROPULSION_CRIT_FUEL,
    PROPULSION_WARN_FUEL, THERMAL_CRIT_TEMP, THERMAL_WARN_TEMP,
};

use assessors::*;
use orbitlogs_models::SubsystemStatus;
use orbitlogs_models::TelemetryRecord;
use serde::{Deserialize, Serialize};

/// Nominal life expectancy baseline in days (~5 years).
const NOMINAL_LIFE_EXPECTANCY_DAYS: f64 = 1800.0;

// =========================================================================
// Health Projection Output Types
// =========================================================================

/// Complete health projection for a single satellite's digital twin.
#[derive(Debug, Serialize)]
pub struct TwinHealthProjection {
    pub satellite_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub subsystems: SubsystemsHealthProjection,
    pub overall_score: i32,
    pub life_expectancy_days: f64,
    pub recommendations: Vec<String>,
}

/// Per-subsystem health assessments.
#[derive(Debug, Serialize)]
pub struct SubsystemsHealthProjection {
    pub power: SubsystemState,
    pub thermal: SubsystemState,
    pub adcs: SubsystemState,
    pub communications: SubsystemState,
    pub payload: SubsystemState,
    pub propulsion: SubsystemState,
    pub flight_computer: SubsystemState,
}

/// Health assessment for a single subsystem.
#[derive(Debug, Serialize)]
pub struct SubsystemState {
    pub status: SubsystemStatus,
    pub score: i32,
    pub details: String,
}

// =========================================================================
// Public API
// =========================================================================

/// Computes a full health projection for a satellite's digital twin.
pub fn project_twin_health(
    satellite: String,
    rec: &TelemetryRecord,
    twin_state: Option<&DigitalTwinState>,
) -> TwinHealthProjection {
    let power = assess_power(rec, twin_state);
    let thermal = assess_thermal(rec, twin_state);
    let adcs = assess_adcs(rec);
    let comms = assess_communications(rec);
    let propulsion = assess_propulsion(rec);
    let payload = assess_payload(rec);
    let fc = assess_flight_computer(rec);

    let mut recommendations = Vec::new();
    let assessments = [&power, &thermal, &adcs, &comms, &propulsion, &payload, &fc];
    for a in &assessments {
        recommendations.extend(a.recommendations.clone());
    }

    let total_penalty: f64 = assessments.iter().map(|a| a.life_penalty).sum();
    let failed_count = assessments
        .iter()
        .filter(|a| a.status == SubsystemStatus::Failed)
        .count();

    let compound_factor = if failed_count > 1 {
        1.0 + (failed_count as f64 - 1.0) * 0.15
    } else {
        1.0
    };

    let life_expectancy = (NOMINAL_LIFE_EXPECTANCY_DAYS - total_penalty * compound_factor).max(0.0);

    if failed_count >= 2 {
        recommendations.push(format!(
            "MULTI-FAILURE ALERT: {} subsystems in FAILED state. Invoke autonomous safe-mode procedure.",
            failed_count
        ));
    }

    let scores: Vec<i32> = assessments.iter().map(|a| a.score).collect();
    let overall_score = if scores.is_empty() {
        0
    } else {
        scores.iter().sum::<i32>() / scores.len() as i32
    };

    TwinHealthProjection {
        satellite_id: satellite,
        timestamp: chrono::Utc::now(),
        subsystems: SubsystemsHealthProjection {
            power: SubsystemState {
                status: power.status,
                score: power.score,
                details: power.details,
            },
            thermal: SubsystemState {
                status: thermal.status,
                score: thermal.score,
                details: thermal.details,
            },
            adcs: SubsystemState {
                status: adcs.status,
                score: adcs.score,
                details: adcs.details,
            },
            communications: SubsystemState {
                status: comms.status,
                score: comms.score,
                details: comms.details,
            },
            payload: SubsystemState {
                status: payload.status,
                score: payload.score,
                details: payload.details,
            },
            propulsion: SubsystemState {
                status: propulsion.status,
                score: propulsion.score,
                details: propulsion.details,
            },
            flight_computer: SubsystemState {
                status: fc.status,
                score: fc.score,
                details: fc.details,
            },
        },
        overall_score,
        life_expectancy_days: life_expectancy,
        recommendations,
    }
}

// =========================================================================
// DigitalTwinState — Recursive Degradation Model
// =========================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DigitalTwinState {
    pub satellite_id: String,
    pub solar_efficiency: f32,
    pub battery_capacity_fade: f32,
    pub radiator_thermal_resistance: f32,
}

impl DigitalTwinState {
    pub fn new(satellite_id: String) -> Self {
        Self {
            satellite_id,
            solar_efficiency: 1.0,
            battery_capacity_fade: 1.0,
            radiator_thermal_resistance: 1.0,
        }
    }

    pub fn estimate_solar_efficiency(&mut self, actual_solar_w: f32, solar_exposure_factor: f32) {
        if solar_exposure_factor > 0.1
            && actual_solar_w.is_finite()
            && solar_exposure_factor.is_finite()
        {
            let theoretical_max = 120.0 * solar_exposure_factor;
            let observed_efficiency = (actual_solar_w / theoretical_max).clamp(0.0, 1.5);
            self.solar_efficiency = 0.999 * self.solar_efficiency + 0.001 * observed_efficiency;
        }
    }

    pub fn estimate_battery_capacity_fade(&mut self, discharge_current: f32, voltage_drop: f32) {
        if discharge_current > 0.5
            && discharge_current.is_finite()
            && voltage_drop.is_finite()
            && voltage_drop >= 0.0
        {
            let observed_resistance = voltage_drop / discharge_current;
            let fade_factor = 0.1 / observed_resistance.max(0.01);
            self.battery_capacity_fade =
                0.999 * self.battery_capacity_fade + 0.001 * fade_factor.min(1.0);
        }
    }

    pub fn estimate_radiator_thermal_resistance(&mut self, temp_c: f32, cpu_power_w: f32) {
        if cpu_power_w > 5.0 && temp_c.is_finite() && cpu_power_w.is_finite() {
            let delta_t = temp_c - 3.0;
            let observed_r_th = delta_t / cpu_power_w;
            let ratio = observed_r_th / 0.5;
            self.radiator_thermal_resistance =
                0.999 * self.radiator_thermal_resistance + 0.001 * ratio;
        }
    }
}
