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

//! Subsystem health assessment evaluators.

use orbitlogs_models::SubsystemStatus;
use orbitlogs_models::TelemetryRecord;
use super::DigitalTwinState;

// Threshold Constants
pub const BATTERY_WARN_VOLTAGE: f64 = 11.5;
pub const BATTERY_CRIT_VOLTAGE: f64 = 10.5;
pub const THERMAL_WARN_TEMP: f64 = 75.0;
pub const THERMAL_CRIT_TEMP: f64 = 90.0;
pub const ADCS_WARN_PITCH_DRIFT: f32 = 10.0;
pub const COMM_WARN_PACKET_LOSS: f32 = 10.0;
pub const COMM_CRIT_PACKET_LOSS: f32 = 50.0;
pub const PROPULSION_WARN_FUEL: f32 = 20.0;
pub const PROPULSION_CRIT_FUEL: f32 = 5.0;
pub const FLIGHT_COMPUTER_WARN_CPU: f32 = 85.0;
pub const FLIGHT_COMPUTER_WARN_MEM: f32 = 90.0;

/// Internal assessment result from each subsystem evaluator.
pub struct Assessment {
    pub status: SubsystemStatus,
    pub score: i32,
    pub details: String,
    pub recommendations: Vec<String>,
    pub life_penalty: f64,
}

/// Assesses the power subsystem (battery voltage + solar panel output).
pub fn assess_power(rec: &TelemetryRecord, twin: Option<&DigitalTwinState>) -> Assessment {
    let battery = rec.battery_voltage.unwrap_or(13.8);
    let solar = rec.solar_panel_output.unwrap_or(100.0);

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

    let effective_solar = if let Some(ts) = twin {
        solar * ts.solar_efficiency as f64
    } else {
        solar
    };

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
pub fn assess_thermal(rec: &TelemetryRecord, twin: Option<&DigitalTwinState>) -> Assessment {
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
pub fn assess_adcs(rec: &TelemetryRecord) -> Assessment {
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
pub fn assess_communications(rec: &TelemetryRecord) -> Assessment {
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
pub fn assess_propulsion(rec: &TelemetryRecord) -> Assessment {
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
pub fn assess_payload(rec: &TelemetryRecord) -> Assessment {
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
pub fn assess_flight_computer(rec: &TelemetryRecord) -> Assessment {
    let cpu = rec.cpu_usage.unwrap_or(0.0);
    let mem = rec.memory_usage.unwrap_or(0.0);

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
