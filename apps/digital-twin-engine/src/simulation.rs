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
//!
//! The `DigitalTwinState` struct maintains recursive low-pass filtered
//! estimates of degradation parameters (solar efficiency, battery fade,
//! radiator thermal resistance) that converge over time as more telemetry
//! samples arrive.

use orbitlogs_models::SubsystemStatus;
use orbitlogs_models::TelemetryRecord;
use serde::{Deserialize, Serialize};

// =========================================================================
// Threshold Constants
// =========================================================================

/// Warning threshold for battery voltage (Volts).
pub const BATTERY_WARN_VOLTAGE: f64 = 11.5;
/// Critical threshold for battery voltage (Volts).
pub const BATTERY_CRIT_VOLTAGE: f64 = 10.5;
/// Warning threshold for component temperature (°C).
pub const THERMAL_WARN_TEMP: f64 = 75.0;
/// Critical threshold for component temperature (°C).
pub const THERMAL_CRIT_TEMP: f64 = 90.0;
/// Warning threshold for ADCS pitch drift (degrees).
pub const ADCS_WARN_PITCH_DRIFT: f32 = 10.0;
/// Warning threshold for communications packet loss (%).
pub const COMM_WARN_PACKET_LOSS: f32 = 10.0;
/// Critical threshold for communications packet loss (%).
pub const COMM_CRIT_PACKET_LOSS: f32 = 50.0;
/// Warning threshold for propellant remaining (%).
pub const PROPULSION_WARN_FUEL: f32 = 20.0;
/// Critical threshold for propellant remaining (%).
pub const PROPULSION_CRIT_FUEL: f32 = 5.0;
/// Warning threshold for CPU usage (%).
pub const FLIGHT_COMPUTER_WARN_CPU: f32 = 85.0;
/// Warning threshold for memory usage (%).
pub const FLIGHT_COMPUTER_WARN_MEM: f32 = 90.0;

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
// Subsystem Assessors (Table-Driven Architecture)
// =========================================================================

/// Internal assessment result from each subsystem evaluator.
struct Assessment {
    status: SubsystemStatus,
    score: i32,
    details: String,
    recommendations: Vec<String>,
    life_penalty: f64,
}

/// Assesses the power subsystem (battery voltage + solar panel output).
fn assess_power(rec: &TelemetryRecord, twin: Option<&DigitalTwinState>) -> Assessment {
    let battery = rec.battery_voltage.unwrap_or(13.8);
    let solar = rec.solar_panel_output.unwrap_or(100.0);

    // Guard against NaN/Inf sensor readings — treat as nominal-with-warning.
    if battery.is_nan() || battery.is_infinite() || solar.is_nan() || solar.is_infinite() {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 50,
            details: "Power telemetry contains invalid sensor readings (NaN/Inf).".to_string(),
            recommendations: vec![
                "Investigate flight computer ADC calibration for power sensors.".to_string(),
            ],
            life_penalty: 100.0,
        };
    }

    // Apply solar efficiency degradation from the twin model if available.
    let effective_solar = if let Some(ts) = twin {
        solar * ts.solar_efficiency as f64
    } else {
        solar
    };

    // Critical: battery under-voltage lockout OR solar generation failure
    if battery < BATTERY_CRIT_VOLTAGE || solar == 0.0 {
        return Assessment {
            status: SubsystemStatus::Failed,
            score: 20,
            details:
                "Battery under-voltage lockout threshold reached or solar array generation failure."
                    .to_string(),
            recommendations: vec![
                "Emergency power mode: Shed all non-vital systems immediately.".to_string(),
            ],
            life_penalty: 1300.0,
        };
    }

    // Warning: battery voltage low
    if battery < BATTERY_WARN_VOLTAGE {
        let mut recs = vec![
            "Charge cycle optimization: Disable secondary payloads to reduce battery strain."
                .to_string(),
        ];
        if let Some(ts) = twin {
            if ts.solar_efficiency < 0.85 {
                recs.push(format!(
                    "Solar array degradation detected (efficiency: {:.1}%). Schedule panel inspection.",
                    ts.solar_efficiency * 100.0
                ));
            }
        }
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 60,
            details: format!(
                "Battery voltage low: {:.1}V. Solar generating: {:.1}W (effective: {:.1}W).",
                battery, solar, effective_solar
            ),
            recommendations: recs,
            life_penalty: 300.0,
        };
    }

    Assessment {
        status: SubsystemStatus::Nominal,
        score: 100,
        details:
            "Solar arrays fully deployed and generating adequate wattage. Battery banks balanced."
                .to_string(),
        recommendations: vec![],
        life_penalty: 0.0,
    }
}

/// Assesses the thermal subsystem (component temperature).
fn assess_thermal(rec: &TelemetryRecord, twin: Option<&DigitalTwinState>) -> Assessment {
    let temp = rec.temperature.unwrap_or(25.0);

    if temp.is_nan() || temp.is_infinite() {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 50,
            details: "Thermal telemetry contains invalid sensor readings (NaN/Inf).".to_string(),
            recommendations: vec!["Investigate thermal sensor calibration.".to_string()],
            life_penalty: 100.0,
        };
    }

    // Apply radiator degradation from the twin model.
    let thermal_note = twin
        .filter(|ts| ts.radiator_thermal_resistance > 1.2)
        .map(|ts| {
            format!(
                " Radiator thermal resistance elevated ({:.2}x nominal).",
                ts.radiator_thermal_resistance
            )
        })
        .unwrap_or_default();

    if temp > THERMAL_CRIT_TEMP {
        return Assessment {
            status: SubsystemStatus::Failed,
            score: 10,
            details: format!(
                "Critical hardware thermal limit exceeded: {:.1}°C. Risk of immediate chip fusion.{}",
                temp, thermal_note
            ),
            recommendations: vec![
                "Immediate reboot and deep power saving to prevent silicon decay.".to_string(),
            ],
            life_penalty: 1000.0,
        };
    }

    if temp > THERMAL_WARN_TEMP {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 55,
            details: format!(
                "Thermal radiator capacity constrained. Component temperature: {:.1}°C.{}",
                temp, thermal_note
            ),
            recommendations: vec![
                "Yaw satellite to align radiator arrays with cold space.".to_string()
            ],
            life_penalty: 200.0,
        };
    }

    Assessment {
        status: SubsystemStatus::Nominal,
        score: 100,
        details: "Radiators operating within design bounds. Internal temperatures nominal."
            .to_string(),
        recommendations: vec![],
        life_penalty: 0.0,
    }
}

/// Assesses the ADCS (Attitude Determination and Control System).
fn assess_adcs(rec: &TelemetryRecord) -> Assessment {
    let pitch = rec.orientation_pitch.unwrap_or(0.0);

    if pitch.is_nan() || pitch.is_infinite() {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 50,
            details: "ADCS telemetry contains invalid sensor readings (NaN/Inf).".to_string(),
            recommendations: vec!["Investigate star tracker and IMU sensor feeds.".to_string()],
            life_penalty: 100.0,
        };
    }

    if pitch.abs() > ADCS_WARN_PITCH_DRIFT {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 65,
            details: format!(
                "Attitude drift detected. Star tracker lock unstable. Pitch error: {:.1}°.",
                pitch
            ),
            recommendations: vec![
                "Trigger magnetorquer alignment sequence to dump reaction wheel momentum."
                    .to_string(),
            ],
            life_penalty: 150.0,
        };
    }

    Assessment {
        status: SubsystemStatus::Nominal,
        score: 100,
        details: "Attitude control holding target star lock. Reaction wheel torque minimal."
            .to_string(),
        recommendations: vec![],
        life_penalty: 0.0,
    }
}

/// Assesses the communications subsystem (packet loss, latency, signal).
fn assess_communications(rec: &TelemetryRecord) -> Assessment {
    let ploss = rec.packet_loss.unwrap_or(0.0);

    if ploss.is_nan() || ploss.is_infinite() {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 50,
            details: "Communications telemetry contains invalid sensor readings (NaN/Inf)."
                .to_string(),
            recommendations: vec!["Investigate communications modem diagnostics.".to_string()],
            life_penalty: 100.0,
        };
    }

    // Guard against negative packet loss (sensor glitch)
    let ploss = ploss.max(0.0).min(100.0);

    if ploss > COMM_CRIT_PACKET_LOSS {
        return Assessment {
            status: SubsystemStatus::Failed,
            score: 25,
            details: format!(
                "Severe communications degradation. Packet loss: {:.1}%. Link effectively down.",
                ploss
            ),
            recommendations: vec![
                "Switch to backup transponder. Initiate ground station handover.".to_string(),
            ],
            life_penalty: 400.0,
        };
    }

    if ploss > COMM_WARN_PACKET_LOSS {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 70,
            details: format!(
                "Uplink jitter detected. Packet loss: {:.1}%. Link margin tight.",
                ploss
            ),
            recommendations: vec![
                "Increase transmitter power or schedule passes with high-gain ground station arrays.".to_string(),
            ],
            life_penalty: 0.0,
        };
    }

    Assessment {
        status: SubsystemStatus::Nominal,
        score: 100,
        details: "Uplink and downlink carriers locked. Bit error rate at standard noise floor."
            .to_string(),
        recommendations: vec![],
        life_penalty: 0.0,
    }
}

/// Assesses the propulsion subsystem (fuel remaining).
fn assess_propulsion(rec: &TelemetryRecord) -> Assessment {
    let fuel = rec.fuel_remaining.unwrap_or(100.0);

    if fuel.is_nan() || fuel.is_infinite() {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 50,
            details: "Propulsion telemetry contains invalid sensor readings (NaN/Inf).".to_string(),
            recommendations: vec!["Investigate fuel gauge sensor calibration.".to_string()],
            life_penalty: 100.0,
        };
    }

    // Guard against negative fuel (sensor glitch) — clamp to [0, 100].
    let fuel = fuel.max(0.0).min(100.0);

    if fuel < PROPULSION_CRIT_FUEL {
        return Assessment {
            status: SubsystemStatus::Failed,
            score: 15,
            details: format!(
                "Propellant critically low: {:.1}%. No maneuver capability remaining.",
                fuel
            ),
            recommendations: vec![
                "Initiate controlled deorbit planning. No further station-keeping burns possible."
                    .to_string(),
            ],
            life_penalty: 900.0,
        };
    }

    if fuel < PROPULSION_WARN_FUEL {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 40,
            details: format!(
                "Propellant supply depleted: {:.1}%. Maneuver window constrained.",
                fuel
            ),
            recommendations: vec![
                "Transition to high-efficiency sparse burns. Plan orbit decay path.".to_string(),
            ],
            life_penalty: 500.0,
        };
    }

    Assessment {
        status: SubsystemStatus::Nominal,
        score: 100,
        details: "Fuel levels adequate for scheduled orbital maintenance maneuvers.".to_string(),
        recommendations: vec![],
        life_penalty: 0.0,
    }
}

/// Assesses the payload subsystem (active status + power draw).
fn assess_payload(rec: &TelemetryRecord) -> Assessment {
    let active = rec.payload_active.unwrap_or(false);
    let power_draw = rec.payload_power_draw.unwrap_or(0.0);

    if active && power_draw < 2.0 {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 50,
            details: "Payload active but drawing minimal power. Sensor integration loop suspended."
                .to_string(),
            recommendations: vec!["Perform payload diagnostics self-test.".to_string()],
            life_penalty: 0.0,
        };
    }

    Assessment {
        status: SubsystemStatus::Nominal,
        score: 100,
        details: "Payload instrument telemetry streams active. Calibration logs clean.".to_string(),
        recommendations: vec![],
        life_penalty: 0.0,
    }
}

/// Assesses the flight computer subsystem (CPU and memory usage).
///
/// This subsystem was previously unmonitored despite telemetry fields being
/// available. High CPU or memory pressure on the flight computer can cause
/// watchdog resets or delayed command execution.
fn assess_flight_computer(rec: &TelemetryRecord) -> Assessment {
    let cpu = rec.cpu_usage.unwrap_or(0.0);
    let mem = rec.memory_usage.unwrap_or(0.0);

    // Guard against invalid readings
    if cpu.is_nan() || cpu.is_infinite() || mem.is_nan() || mem.is_infinite() {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 50,
            details: "Flight computer telemetry contains invalid sensor readings (NaN/Inf)."
                .to_string(),
            recommendations: vec!["Investigate flight computer self-diagnostics.".to_string()],
            life_penalty: 100.0,
        };
    }

    let cpu = cpu.max(0.0).min(100.0);
    let mem = mem.max(0.0).min(100.0);

    if cpu > FLIGHT_COMPUTER_WARN_CPU && mem > FLIGHT_COMPUTER_WARN_MEM {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 35,
            details: format!(
                "Flight computer under heavy load. CPU: {:.1}%, Memory: {:.1}%. Watchdog reset risk.",
                cpu, mem
            ),
            recommendations: vec![
                "Reduce payload processing cadence. Defer non-critical housekeeping tasks.".to_string(),
                "Schedule flight software garbage collection cycle.".to_string(),
            ],
            life_penalty: 200.0,
        };
    }

    if cpu > FLIGHT_COMPUTER_WARN_CPU {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 60,
            details: format!(
                "Flight computer CPU usage elevated: {:.1}%. Command processing may be delayed.",
                cpu
            ),
            recommendations: vec![
                "Monitor for watchdog timer resets. Reduce housekeeping task frequency."
                    .to_string(),
            ],
            life_penalty: 100.0,
        };
    }

    if mem > FLIGHT_COMPUTER_WARN_MEM {
        return Assessment {
            status: SubsystemStatus::Degraded,
            score: 60,
            details: format!(
                "Flight computer memory pressure: {:.1}%. Risk of OOM-triggered reboot.",
                mem
            ),
            recommendations: vec![
                "Purge telemetry buffers. Schedule flight software memory defragmentation."
                    .to_string(),
            ],
            life_penalty: 100.0,
        };
    }

    Assessment {
        status: SubsystemStatus::Nominal,
        score: 100,
        details: "Flight computer operating within nominal CPU and memory bounds.".to_string(),
        recommendations: vec![],
        life_penalty: 0.0,
    }
}

// =========================================================================
// Public API
// =========================================================================

/// Computes a full health projection for a satellite's digital twin.
///
/// Evaluates all subsystems, aggregates recommendations, computes an
/// overall score (weighted average), and calculates adjusted life
/// expectancy based on cumulative degradation penalties.
///
/// An optional `DigitalTwinState` can be provided to incorporate the
/// recursive degradation model estimates (solar efficiency, battery fade,
/// thermal resistance) into the assessments.
pub fn project_twin_health(
    satellite: String,
    rec: &TelemetryRecord,
    twin_state: Option<&DigitalTwinState>,
) -> TwinHealthProjection {
    // Run all subsystem assessors
    let power = assess_power(rec, twin_state);
    let thermal = assess_thermal(rec, twin_state);
    let adcs = assess_adcs(rec);
    let comms = assess_communications(rec);
    let propulsion = assess_propulsion(rec);
    let payload = assess_payload(rec);
    let fc = assess_flight_computer(rec);

    // Aggregate recommendations
    let mut recommendations = Vec::new();
    let assessments = [&power, &thermal, &adcs, &comms, &propulsion, &payload, &fc];
    for a in &assessments {
        recommendations.extend(a.recommendations.clone());
    }

    // Compute life expectancy with compounding penalty for multi-failure
    let total_penalty: f64 = assessments.iter().map(|a| a.life_penalty).sum();
    let failed_count = assessments
        .iter()
        .filter(|a| a.status == SubsystemStatus::Failed)
        .count();

    // Multi-failure compounding: each additional simultaneous failure
    // accelerates degradation by 15%.
    let compound_factor = if failed_count > 1 {
        1.0 + (failed_count as f64 - 1.0) * 0.15
    } else {
        1.0
    };

    let life_expectancy = (NOMINAL_LIFE_EXPECTANCY_DAYS - total_penalty * compound_factor).max(0.0);

    // Multi-failure cross-system recommendation
    if failed_count >= 2 {
        recommendations.push(format!(
            "MULTI-FAILURE ALERT: {} subsystems in FAILED state. Invoke autonomous safe-mode procedure.",
            failed_count
        ));
    }

    // Compute overall score as weighted average (all subsystems equal weight).
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

/// Maintains recursive low-pass filtered estimates of spacecraft degradation
/// parameters. Each call to `estimate_*` updates the internal state using an
/// exponential moving average (alpha = 0.001), causing estimates to converge
/// over thousands of telemetry samples.
///
/// This is analogous to a simplified Kalman filter for hardware degradation
/// tracking across the constellation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DigitalTwinState {
    pub satellite_id: String,
    /// Estimated solar panel efficiency relative to beginning-of-life (0.0–1.0).
    pub solar_efficiency: f32,
    /// Estimated battery capacity relative to beginning-of-life (0.0–1.0).
    pub battery_capacity_fade: f32,
    /// Estimated radiator thermal resistance relative to nominal (1.0 = nominal).
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

    /// Updates the solar efficiency estimate based on observed solar panel output.
    ///
    /// `solar_exposure_factor` should be in (0.0, 1.0] representing what fraction
    /// of the orbit the satellite is in sunlight. Values ≤ 0.1 are ignored to
    /// avoid dividing by near-zero.
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

    /// Updates the battery capacity fade estimate based on discharge current
    /// and observed voltage drop.
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

    /// Updates the radiator thermal resistance estimate based on component
    /// temperature and CPU power draw.
    pub fn estimate_radiator_thermal_resistance(&mut self, temp_c: f32, cpu_power_w: f32) {
        if cpu_power_w > 5.0 && temp_c.is_finite() && cpu_power_w.is_finite() {
            let delta_t = temp_c - 3.0; // Background space temperature
            let observed_r_th = delta_t / cpu_power_w;
            let ratio = observed_r_th / 0.5; // Nominal thermal resistance
            self.radiator_thermal_resistance =
                0.999 * self.radiator_thermal_resistance + 0.001 * ratio;
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use orbitlogs_models::SubsystemStatus;
    use uuid::Uuid;

    fn mock_nominal_telemetry() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: orbitlogs_models::SatelliteId::new("ORBITLOGS-9999"),
            subsystem_id: orbitlogs_models::Subsystem::Adcs,
            mission_id: orbitlogs_models::MissionId::new("ORBITLOGS-V2"),
            timestamp: chrono::Utc::now(),
            battery_voltage: Some(13.8),
            battery_current: Some(1.2),
            solar_panel_output: Some(120.0),
            cpu_usage: Some(15.0),
            memory_usage: Some(30.0),
            temperature: Some(25.0),
            orientation_pitch: Some(0.0),
            orientation_roll: Some(0.0),
            orientation_yaw: Some(0.0),
            velocity: Some(7.6),
            altitude: Some(550.0),
            fuel_remaining: Some(85.0),
            communication_latency: Some(25.0),
            packet_loss: Some(0.1),
            signal_strength: Some(-65.0),
            payload_power_draw: Some(15.0),
            payload_active: Some(true),
            event_id: Uuid::new_v4(),
        }
    }

    // ----- Nominal Case -----

    #[test]
    fn test_nominal_health_projection() {
        let rec = mock_nominal_telemetry();
        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);

        assert_eq!(proj.satellite_id, "ORBITLOGS-9999");
        assert_eq!(proj.subsystems.power.status, SubsystemStatus::Nominal);
        assert_eq!(proj.subsystems.power.score, 100);
        assert_eq!(proj.subsystems.thermal.status, SubsystemStatus::Nominal);
        assert_eq!(proj.subsystems.thermal.score, 100);
        assert_eq!(proj.subsystems.adcs.status, SubsystemStatus::Nominal);
        assert_eq!(proj.subsystems.adcs.score, 100);
        assert_eq!(
            proj.subsystems.communications.status,
            SubsystemStatus::Nominal
        );
        assert_eq!(proj.subsystems.communications.score, 100);
        assert_eq!(proj.subsystems.propulsion.status, SubsystemStatus::Nominal);
        assert_eq!(proj.subsystems.propulsion.score, 100);
        assert_eq!(proj.subsystems.payload.status, SubsystemStatus::Nominal);
        assert_eq!(proj.subsystems.payload.score, 100);
        assert_eq!(
            proj.subsystems.flight_computer.status,
            SubsystemStatus::Nominal
        );
        assert_eq!(proj.subsystems.flight_computer.score, 100);
        assert_eq!(proj.life_expectancy_days, 1800.0);
        assert!(proj.recommendations.is_empty());
        assert_eq!(proj.overall_score, 100);
    }

    // ----- Power Subsystem -----

    #[test]
    fn test_degraded_battery_voltage() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(11.2); // < 11.5

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj.subsystems.power.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.power.score, 60);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("Charge cycle optimization")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 300.0);
    }

    #[test]
    fn test_failed_battery_voltage_and_solar() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(10.2); // < 10.5
        rec.solar_panel_output = Some(0.0);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj.subsystems.power.status, SubsystemStatus::Failed);
        assert_eq!(proj.subsystems.power.score, 20);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("Emergency power mode")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 1300.0);
    }

    // ----- Thermal Subsystem -----

    #[test]
    fn test_degraded_and_failed_thermal_spike() {
        let mut rec = mock_nominal_telemetry();
        rec.temperature = Some(78.0); // > 75.0

        let proj1 = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj1.subsystems.thermal.status, SubsystemStatus::Degraded);
        assert_eq!(proj1.subsystems.thermal.score, 55);
        assert!(proj1
            .recommendations
            .iter()
            .any(|r| r.contains("Yaw satellite to align radiator")));
        assert_eq!(proj1.life_expectancy_days, 1800.0 - 200.0);

        rec.temperature = Some(92.0); // > 90.0
        let proj2 = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj2.subsystems.thermal.status, SubsystemStatus::Failed);
        assert_eq!(proj2.subsystems.thermal.score, 10);
        assert!(proj2
            .recommendations
            .iter()
            .any(|r| r.contains("Immediate reboot")));
        assert_eq!(proj2.life_expectancy_days, 1800.0 - 1000.0);
    }

    // ----- ADCS -----

    #[test]
    fn test_degraded_adcs_pitch() {
        let mut rec = mock_nominal_telemetry();
        rec.orientation_pitch = Some(12.5); // > 10.0

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj.subsystems.adcs.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.adcs.score, 65);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("Trigger magnetorquer alignment")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 150.0);
    }

    // ----- Propulsion -----

    #[test]
    fn test_degraded_propulsion_fuel() {
        let mut rec = mock_nominal_telemetry();
        rec.fuel_remaining = Some(15.0); // < 20.0

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj.subsystems.propulsion.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.propulsion.score, 40);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("Transition to high-efficiency sparse burns")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 500.0);
    }

    #[test]
    fn test_critical_propulsion_fuel() {
        let mut rec = mock_nominal_telemetry();
        rec.fuel_remaining = Some(3.0); // < 5.0

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj.subsystems.propulsion.status, SubsystemStatus::Failed);
        assert_eq!(proj.subsystems.propulsion.score, 15);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("deorbit planning")));
    }

    // ----- Communications -----

    #[test]
    fn test_degraded_communications() {
        let mut rec = mock_nominal_telemetry();
        rec.packet_loss = Some(25.0); // > 10.0

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(
            proj.subsystems.communications.status,
            SubsystemStatus::Degraded
        );
        assert_eq!(proj.subsystems.communications.score, 70);
    }

    #[test]
    fn test_failed_communications() {
        let mut rec = mock_nominal_telemetry();
        rec.packet_loss = Some(75.0); // > 50.0

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(
            proj.subsystems.communications.status,
            SubsystemStatus::Failed
        );
        assert_eq!(proj.subsystems.communications.score, 25);
    }

    // ----- Flight Computer -----

    #[test]
    fn test_degraded_flight_computer_cpu() {
        let mut rec = mock_nominal_telemetry();
        rec.cpu_usage = Some(90.0); // > 85.0

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(
            proj.subsystems.flight_computer.status,
            SubsystemStatus::Degraded
        );
        assert_eq!(proj.subsystems.flight_computer.score, 60);
    }

    #[test]
    fn test_degraded_flight_computer_both() {
        let mut rec = mock_nominal_telemetry();
        rec.cpu_usage = Some(92.0);
        rec.memory_usage = Some(95.0);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(
            proj.subsystems.flight_computer.status,
            SubsystemStatus::Degraded
        );
        assert_eq!(proj.subsystems.flight_computer.score, 35);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("Watchdog reset risk")));
    }

    // ----- Edge Cases -----

    #[test]
    fn test_nan_telemetry_values() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(f64::NAN);
        rec.temperature = Some(f64::NAN);
        rec.orientation_pitch = Some(f32::NAN);
        rec.packet_loss = Some(f32::NAN);
        rec.fuel_remaining = Some(f32::NAN);
        rec.cpu_usage = Some(f32::NAN);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);

        // All subsystems with NaN should be Degraded, not panic
        assert_eq!(proj.subsystems.power.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.thermal.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.adcs.status, SubsystemStatus::Degraded);
        assert_eq!(
            proj.subsystems.communications.status,
            SubsystemStatus::Degraded
        );
        assert_eq!(proj.subsystems.propulsion.status, SubsystemStatus::Degraded);
        assert_eq!(
            proj.subsystems.flight_computer.status,
            SubsystemStatus::Degraded
        );
    }

    #[test]
    fn test_infinity_telemetry_values() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(f64::INFINITY);
        rec.temperature = Some(f64::NEG_INFINITY);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj.subsystems.power.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.thermal.status, SubsystemStatus::Degraded);
    }

    #[test]
    fn test_negative_sensor_values_clamped() {
        let mut rec = mock_nominal_telemetry();
        rec.fuel_remaining = Some(-5.0); // negative fuel
        rec.packet_loss = Some(-10.0); // negative packet loss

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        // Negative fuel should be clamped to 0.0 → critical
        assert_eq!(proj.subsystems.propulsion.status, SubsystemStatus::Failed);
        // Negative packet loss should be clamped to 0.0 → nominal
        assert_eq!(
            proj.subsystems.communications.status,
            SubsystemStatus::Nominal
        );
    }

    #[test]
    fn test_multi_failure_compounding() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(10.0); // Failed
        rec.temperature = Some(95.0); // Failed
        rec.fuel_remaining = Some(2.0); // Failed

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("MULTI-FAILURE ALERT")));
        // Life expectancy should be lower than sum of individual penalties
        // due to compounding
        assert!(proj.life_expectancy_days < 1800.0 - 1300.0 - 1000.0 - 900.0);
    }

    #[test]
    fn test_all_subsystems_degraded_simultaneously() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(11.0);
        rec.temperature = Some(80.0);
        rec.orientation_pitch = Some(12.0);
        rec.packet_loss = Some(15.0);
        rec.fuel_remaining = Some(15.0);
        rec.cpu_usage = Some(90.0);
        rec.payload_active = Some(true);
        rec.payload_power_draw = Some(1.0);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        // All subsystems should be degraded
        assert_eq!(proj.subsystems.power.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.thermal.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.adcs.status, SubsystemStatus::Degraded);
        assert_eq!(
            proj.subsystems.communications.status,
            SubsystemStatus::Degraded
        );
        assert_eq!(proj.subsystems.propulsion.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.payload.status, SubsystemStatus::Degraded);
        assert_eq!(
            proj.subsystems.flight_computer.status,
            SubsystemStatus::Degraded
        );
        // Overall score should be well below 100
        assert!(proj.overall_score < 70);
    }

    // ----- DigitalTwinState Tests -----

    #[test]
    fn test_digital_twin_solar_efficiency_estimation() {
        let mut twin = DigitalTwinState::new("SAT-0001".to_string());
        assert_eq!(twin.solar_efficiency, 1.0);

        // Simulating actual output of 96W when theoretical max is 120W
        twin.estimate_solar_efficiency(96.0, 1.0);
        assert!(twin.solar_efficiency < 1.0);
        assert!((twin.solar_efficiency - 0.9998).abs() < 1e-4);
    }

    #[test]
    fn test_digital_twin_rejects_nan_inputs() {
        let mut twin = DigitalTwinState::new("SAT-0001".to_string());
        let original_efficiency = twin.solar_efficiency;

        twin.estimate_solar_efficiency(f32::NAN, 1.0);
        assert_eq!(twin.solar_efficiency, original_efficiency);

        twin.estimate_battery_capacity_fade(f32::NAN, 1.0);
        assert_eq!(twin.battery_capacity_fade, 1.0);

        twin.estimate_radiator_thermal_resistance(f32::NAN, 10.0);
        assert_eq!(twin.radiator_thermal_resistance, 1.0);
    }

    #[test]
    fn test_digital_twin_convergence() {
        let mut twin = DigitalTwinState::new("SAT-0001".to_string());

        // Simulate 5000 samples at 80% solar efficiency
        for _ in 0..5000 {
            twin.estimate_solar_efficiency(96.0, 1.0); // 96/120 = 0.8
        }

        // After many samples, estimate should converge toward 0.8
        assert!(
            (twin.solar_efficiency - 0.8).abs() < 0.02,
            "Expected convergence near 0.8, got {}",
            twin.solar_efficiency
        );
    }

    #[test]
    fn test_health_projection_with_twin_state() {
        let rec = mock_nominal_telemetry();
        let mut twin = DigitalTwinState::new("SAT-0001".to_string());
        twin.solar_efficiency = 0.7; // Degraded panels

        let proj = project_twin_health("SAT-0001".to_string(), &rec, Some(&twin));
        // With good battery voltage, should still be nominal
        assert_eq!(proj.subsystems.power.status, SubsystemStatus::Nominal);
    }
}
