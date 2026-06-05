#![allow(
    clippy::upper_case_acronyms,
    clippy::wrong_self_convention,
    clippy::inherent_to_string,
    clippy::manual_range_contains
)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use spaceops_models::TelemetryRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundStationState {
    pub timestamp: DateTime<Utc>,
    pub station_id: String,
    pub station_name: String,
    pub status: String,
    pub antenna_health: f32,
    pub weather_state: String,
    pub congestion: f32,
    pub latency: f32,
    pub bandwidth: f32,
    pub explanation: String,
}

fn get_linked_ground_station(satellite_id: &str) -> String {
    if let Some(num_str) = satellite_id.strip_prefix("SAT-") {
        if let Ok(num) = num_str.parse::<usize>() {
            let gs_num = ((num - 1) % 4) + 1;
            return format!("GS-{:04}", gs_num);
        }
    }
    "GS-0001".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthState {
    GREEN,
    YELLOW,
    RED,
}

impl HealthState {
    pub fn to_string(&self) -> String {
        match self {
            HealthState::GREEN => "GREEN".to_string(),
            HealthState::YELLOW => "YELLOW".to_string(),
            HealthState::RED => "RED".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubsystemHealth {
    pub state: HealthState,
    pub score: i32,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteHealthAssessment {
    pub timestamp: DateTime<Utc>,
    pub satellite_id: String,
    pub mission_id: String,
    pub overall_state: HealthState,
    pub overall_score: i32, // Average of subsystem scores
    pub power: SubsystemHealth,
    pub thermal: SubsystemHealth,
    pub communications: SubsystemHealth,
    pub propulsion: SubsystemHealth,
    pub adcs: SubsystemHealth,
    pub payload: SubsystemHealth,
    pub flight_computer: SubsystemHealth,
    pub explanation: String,
}

pub fn assess_satellite_health(
    record: &TelemetryRecord,
    gs_states: &std::collections::HashMap<String, GroundStationState>,
) -> SatelliteHealthAssessment {
    // 1. Assess Power Subsystem
    let power = {
        let v = record.battery_voltage.unwrap_or(13.8);
        if v < 10.0 || v > 16.0 {
            SubsystemHealth {
                state: HealthState::RED,
                score: 0,
                explanation: format!("Critical battery voltage: {:.2}V", v),
            }
        } else if v < 11.5 {
            SubsystemHealth {
                state: HealthState::YELLOW,
                score: 50,
                explanation: format!("Low battery voltage: {:.2}V", v),
            }
        } else {
            SubsystemHealth {
                state: HealthState::GREEN,
                score: 100,
                explanation: format!("Power system normal ({:.2}V)", v),
            }
        }
    };

    // 2. Assess Thermal Subsystem
    let thermal = {
        let t = record.temperature.unwrap_or(25.0);
        if t < -20.0 || t > 85.0 {
            SubsystemHealth {
                state: HealthState::RED,
                score: 0,
                explanation: format!("Critical temperature: {:.2}°C", t),
            }
        } else if t < 0.0 || t > 60.0 {
            SubsystemHealth {
                state: HealthState::YELLOW,
                score: 50,
                explanation: format!("Elevated temperature: {:.2}°C", t),
            }
        } else {
            SubsystemHealth {
                state: HealthState::GREEN,
                score: 100,
                explanation: format!("Thermal system normal ({:.2}°C)", t),
            }
        }
    };

    // 3. Assess Communications Subsystem
    let communications = {
        let loss = record.packet_loss.unwrap_or(0.0);
        let latency = record.communication_latency.unwrap_or(45.0);
        let signal = record.signal_strength.unwrap_or(-70.0);

        let linked_gs_id = get_linked_ground_station(&record.satellite_id);
        let gs_state = gs_states.get(&linked_gs_id);

        let is_degraded = loss > 5.0 || latency > 800.0 || signal < -95.0;
        let is_critical = loss > 15.0 || latency > 2000.0 || signal < -110.0;

        if is_critical {
            let explanation = if let Some(gs) = gs_state {
                if gs.status == "OUTAGE" || gs.status == "DEGRADED" {
                    format!("Critical communications degradation due to ground station {} {} (Weather: {})", gs.station_id, gs.status.to_lowercase(), gs.weather_state)
                } else {
                    format!("Critical comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
                }
            } else {
                format!("Critical comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
            };

            SubsystemHealth {
                state: HealthState::RED,
                score: 0,
                explanation,
            }
        } else if is_degraded {
            let explanation = if let Some(gs) = gs_state {
                if gs.status == "OUTAGE" || gs.status == "DEGRADED" {
                    format!(
                        "Communications degraded due to ground station {} {} (Weather: {})",
                        gs.station_id,
                        gs.status.to_lowercase(),
                        gs.weather_state
                    )
                } else {
                    format!("Comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
                }
            } else {
                format!("Comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
            };

            SubsystemHealth {
                state: HealthState::YELLOW,
                score: 50,
                explanation,
            }
        } else {
            SubsystemHealth {
                state: HealthState::GREEN,
                score: 100,
                explanation: format!(
                    "Communications normal (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)",
                    loss, latency, signal
                ),
            }
        }
    };

    // 4. Assess Propulsion Subsystem
    let propulsion = {
        let fuel = record.fuel_remaining.unwrap_or(95.0);
        let alt = record.altitude.unwrap_or(520.0);

        if fuel < 5.0 || alt < 150.0 {
            SubsystemHealth {
                state: HealthState::RED,
                score: 0,
                explanation: format!(
                    "Critical propulsion/orbital decay (Fuel: {:.1}%, Altitude: {:.1}km)",
                    fuel, alt
                ),
            }
        } else if fuel < 20.0 || alt < 300.0 {
            SubsystemHealth {
                state: HealthState::YELLOW,
                score: 50,
                explanation: format!(
                    "Propulsion degraded/low orbit (Fuel: {:.1}%, Altitude: {:.1}km)",
                    fuel, alt
                ),
            }
        } else {
            SubsystemHealth {
                state: HealthState::GREEN,
                score: 100,
                explanation: format!(
                    "Propulsion normal (Fuel: {:.1}%, Altitude: {:.1}km)",
                    fuel, alt
                ),
            }
        }
    };

    // 5. Assess ADCS Subsystem
    let adcs = {
        let p = record.orientation_pitch.unwrap_or(0.0).abs();
        let r = record.orientation_roll.unwrap_or(0.0).abs();
        let y = record.orientation_yaw.unwrap_or(0.0).abs();

        if p > 45.0 || r > 45.0 || y > 45.0 {
            SubsystemHealth {
                state: HealthState::RED,
                score: 0,
                explanation: format!(
                    "Critical attitude drift (Pitch: {:.1}°, Roll: {:.1}°, Yaw: {:.1}°)",
                    p, r, y
                ),
            }
        } else if p > 15.0 || r > 15.0 || y > 15.0 {
            SubsystemHealth {
                state: HealthState::YELLOW,
                score: 50,
                explanation: format!(
                    "Attitude drift (Pitch: {:.1}°, Roll: {:.1}°, Yaw: {:.1}°)",
                    p, r, y
                ),
            }
        } else {
            SubsystemHealth {
                state: HealthState::GREEN,
                score: 100,
                explanation: "ADCS control loop nominal".to_string(),
            }
        }
    };

    // 6. Assess Payload Subsystem
    let payload = {
        let active = record.payload_active.unwrap_or(true);
        let draw = record.payload_power_draw.unwrap_or(15.0);
        let v = record.battery_voltage.unwrap_or(13.8);

        if active && draw > 30.0 {
            SubsystemHealth {
                state: HealthState::RED,
                score: 0,
                explanation: format!("Critical payload power draw: {:.1}W", draw),
            }
        } else if !active && v > 11.5 {
            SubsystemHealth {
                state: HealthState::RED,
                score: 0,
                explanation: "Payload is inactive despite healthy battery".to_string(),
            }
        } else if !active && v <= 11.5 {
            SubsystemHealth {
                state: HealthState::YELLOW,
                score: 50,
                explanation: "Payload auto-shutdown to protect battery".to_string(),
            }
        } else if active && draw > 20.0 {
            SubsystemHealth {
                state: HealthState::YELLOW,
                score: 50,
                explanation: format!("Elevated payload power draw: {:.1}W", draw),
            }
        } else {
            SubsystemHealth {
                state: HealthState::GREEN,
                score: 100,
                explanation: format!("Payload operating normally ({:.1}W)", draw),
            }
        }
    };

    // 7. Assess Flight Computer Subsystem
    let flight_computer = {
        let cpu = record.cpu_usage.unwrap_or(25.0);
        let mem = record.memory_usage.unwrap_or(45.0);

        if cpu > 95.0 || mem > 95.0 {
            SubsystemHealth {
                state: HealthState::RED,
                score: 0,
                explanation: format!(
                    "Critical flight computer load (CPU: {:.1}%, Mem: {:.1}%)",
                    cpu, mem
                ),
            }
        } else if cpu > 80.0 || mem > 80.0 {
            SubsystemHealth {
                state: HealthState::YELLOW,
                score: 50,
                explanation: format!(
                    "High flight computer load (CPU: {:.1}%, Mem: {:.1}%)",
                    cpu, mem
                ),
            }
        } else {
            SubsystemHealth {
                state: HealthState::GREEN,
                score: 100,
                explanation: format!(
                    "Flight computer operational (CPU: {:.1}%, Mem: {:.1}%)",
                    cpu, mem
                ),
            }
        }
    };

    // Calculate overall satellite state (worst-case subsystem state)
    let overall_state = {
        if power.state == HealthState::RED
            || thermal.state == HealthState::RED
            || communications.state == HealthState::RED
            || propulsion.state == HealthState::RED
            || adcs.state == HealthState::RED
            || payload.state == HealthState::RED
            || flight_computer.state == HealthState::RED
        {
            HealthState::RED
        } else if power.state == HealthState::YELLOW
            || thermal.state == HealthState::YELLOW
            || communications.state == HealthState::YELLOW
            || propulsion.state == HealthState::YELLOW
            || adcs.state == HealthState::YELLOW
            || payload.state == HealthState::YELLOW
            || flight_computer.state == HealthState::YELLOW
        {
            HealthState::YELLOW
        } else {
            HealthState::GREEN
        }
    };

    // Calculate overall score (average of subsystem scores)
    let overall_score = (power.score
        + thermal.score
        + communications.score
        + propulsion.score
        + adcs.score
        + payload.score
        + flight_computer.score)
        / 7;

    // Build explainable unified status string
    let explanation = {
        let mut issues = Vec::new();
        if power.state != HealthState::GREEN {
            issues.push(format!("Power: {}", power.explanation));
        }
        if thermal.state != HealthState::GREEN {
            issues.push(format!("Thermal: {}", thermal.explanation));
        }
        if communications.state != HealthState::GREEN {
            issues.push(format!("Comms: {}", communications.explanation));
        }
        if propulsion.state != HealthState::GREEN {
            issues.push(format!("Propulsion: {}", propulsion.explanation));
        }
        if adcs.state != HealthState::GREEN {
            issues.push(format!("ADCS: {}", adcs.explanation));
        }
        if payload.state != HealthState::GREEN {
            issues.push(format!("Payload: {}", payload.explanation));
        }
        if flight_computer.state != HealthState::GREEN {
            issues.push(format!("FlightComp: {}", flight_computer.explanation));
        }

        if issues.is_empty() {
            "All subsystems operating within normal parameters.".to_string()
        } else {
            issues.join("; ")
        }
    };

    SatelliteHealthAssessment {
        timestamp: record.timestamp,
        satellite_id: record.satellite_id.clone(),
        mission_id: record.mission_id.clone(),
        overall_state,
        overall_score,
        power,
        thermal,
        communications,
        propulsion,
        adcs,
        payload,
        flight_computer,
        explanation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn mock_nominal_record() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: "SAT-0001".to_string(),
            subsystem_id: "unified".to_string(),
            mission_id: "MISSION-ALPHA".to_string(),
            timestamp: Utc::now(),
            battery_voltage: Some(13.8),
            battery_current: Some(2.1),
            solar_panel_output: Some(110.0),
            cpu_usage: Some(25.4),
            memory_usage: Some(45.1),
            temperature: Some(28.5),
            orientation_pitch: Some(1.2),
            orientation_roll: Some(-2.3),
            orientation_yaw: Some(0.5),
            velocity: Some(7.67),
            altitude: Some(520.4),
            fuel_remaining: Some(92.5),
            communication_latency: Some(35.1),
            packet_loss: Some(0.1),
            signal_strength: Some(-68.0),
            payload_power_draw: Some(15.0),
            payload_active: Some(true),
            event_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_nominal_health_assessment() {
        let rec = mock_nominal_record();
        let gs_states = std::collections::HashMap::new();
        let health = assess_satellite_health(&rec, &gs_states);

        assert_eq!(health.overall_state, HealthState::GREEN);
        assert_eq!(health.overall_score, 100);
        assert_eq!(
            health.explanation,
            "All subsystems operating within normal parameters."
        );
    }

    #[test]
    fn test_thermal_yellow_health() {
        let mut rec = mock_nominal_record();
        rec.temperature = Some(65.0); // Yellow range (>60)
        let gs_states = std::collections::HashMap::new();
        let health = assess_satellite_health(&rec, &gs_states);

        assert_eq!(health.overall_state, HealthState::YELLOW);
        assert_eq!(health.thermal.state, HealthState::YELLOW);
        assert_eq!(health.overall_score, 92); // (6*100 + 50) / 7 = 92
        assert!(health.explanation.contains("Thermal: Elevated temperature"));
    }

    #[test]
    fn test_power_critical_red_health() {
        let mut rec = mock_nominal_record();
        rec.battery_voltage = Some(9.5); // Red range (<10)
        let gs_states = std::collections::HashMap::new();
        let health = assess_satellite_health(&rec, &gs_states);

        assert_eq!(health.overall_state, HealthState::RED);
        assert_eq!(health.power.state, HealthState::RED);
        assert_eq!(health.overall_score, 85); // (6*100 + 0) / 7 = 85
        assert!(health
            .explanation
            .contains("Power: Critical battery voltage"));
    }

    #[test]
    fn test_payload_degraded_battery_dependency() {
        let mut rec = mock_nominal_record();
        rec.battery_voltage = Some(11.2); // Low battery, payload shutdown active
        rec.payload_active = Some(false);
        rec.payload_power_draw = Some(0.0);
        let gs_states = std::collections::HashMap::new();
        let health = assess_satellite_health(&rec, &gs_states);

        assert_eq!(health.overall_state, HealthState::YELLOW);
        assert_eq!(health.payload.state, HealthState::YELLOW);
        assert!(health
            .explanation
            .contains("Payload: Payload auto-shutdown to protect battery"));
    }

    #[test]
    fn test_comms_degraded_due_to_ground_station_outage() {
        let mut rec = mock_nominal_record();
        rec.packet_loss = Some(20.0);
        rec.communication_latency = Some(2500.0);
        rec.signal_strength = Some(-115.0);

        let mut gs_states = std::collections::HashMap::new();
        gs_states.insert(
            "GS-0001".to_string(),
            GroundStationState {
                timestamp: Utc::now(),
                station_id: "GS-0001".to_string(),
                station_name: "Svalbard".to_string(),
                status: "OUTAGE".to_string(),
                antenna_health: 12.5,
                weather_state: "Clear".to_string(),
                congestion: 0.0,
                latency: 2500.0,
                bandwidth: 0.0,
                explanation: "Antenna alignment failure".to_string(),
            },
        );

        let health = assess_satellite_health(&rec, &gs_states);
        assert_eq!(health.overall_state, HealthState::RED);
        assert_eq!(health.communications.state, HealthState::RED);
        assert!(health.explanation.contains("Critical communications degradation due to ground station GS-0001 outage (Weather: Clear)"), "Explanation was: {}", health.explanation);
    }

    #[test]
    fn test_comms_degraded_onboard_transponder_anomaly() {
        let mut rec = mock_nominal_record();
        rec.packet_loss = Some(20.0);
        rec.communication_latency = Some(2500.0);
        rec.signal_strength = Some(-115.0);

        let mut gs_states = std::collections::HashMap::new();
        gs_states.insert(
            "GS-0001".to_string(),
            GroundStationState {
                timestamp: Utc::now(),
                station_id: "GS-0001".to_string(),
                station_name: "Svalbard".to_string(),
                status: "ONLINE".to_string(),
                antenna_health: 100.0,
                weather_state: "Clear".to_string(),
                congestion: 0.0,
                latency: 42.0,
                bandwidth: 1000.0,
                explanation: "All operational".to_string(),
            },
        );

        let health = assess_satellite_health(&rec, &gs_states);
        assert_eq!(health.overall_state, HealthState::RED);
        assert_eq!(health.communications.state, HealthState::RED);
        assert!(
            health
                .explanation
                .contains("Critical comm degradation due to onboard transponder anomaly"),
            "Explanation was: {}",
            health.explanation
        );
    }
}
