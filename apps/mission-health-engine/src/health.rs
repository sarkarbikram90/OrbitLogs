#![allow(
    clippy::upper_case_acronyms,
    clippy::wrong_self_convention,
    clippy::inherent_to_string
)]
use chrono::{DateTime, Utc};
use orbitlogs_models::TelemetryRecord;
use serde::{Deserialize, Serialize};

pub use orbitlogs_models::GroundStationHealth as GroundStationState;
pub use orbitlogs_models::HealthState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubsystemHealth {
    pub state: HealthState,
    pub score: i32,
    pub explanation: String,
}

fn lerp(val: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    if in_max == in_min {
        return out_min;
    }
    let t = (val - in_min) / (in_max - in_min);
    let t = t.clamp(0.0, 1.0);
    out_min + t * (out_max - out_min)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteHealthAssessment {
    pub timestamp: DateTime<Utc>,
    pub satellite_id: orbitlogs_models::SatelliteId,
    pub mission_id: orbitlogs_models::MissionId,
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

// =========================================================================
// Subsystem Assessors
// =========================================================================

fn assess_power(record: &TelemetryRecord) -> SubsystemHealth {
    let v = record.battery_voltage.unwrap_or(13.8);
    if !(10.0..=16.0).contains(&v) {
        let score = if v < 10.0 {
            lerp(v, 8.0, 10.0, 0.0, 24.0) as i32
        } else {
            lerp(v, 16.0, 18.0, 24.0, 0.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!("Critical battery voltage: {:.2}V", v),
        }
    } else if v < 11.5 {
        let score = lerp(v, 10.0, 11.5, 25.0, 74.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!("Low battery voltage: {:.2}V", v),
        }
    } else {
        let score = if v < 13.8 {
            lerp(v, 11.5, 12.5, 75.0, 100.0) as i32
        } else {
            lerp(v, 15.0, 16.0, 100.0, 75.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!("Power system normal ({:.2}V)", v),
        }
    }
}

fn assess_thermal(record: &TelemetryRecord) -> SubsystemHealth {
    let t = record.temperature.unwrap_or(25.0);
    if !(-20.0..=85.0).contains(&t) {
        let score = if t < -20.0 {
            lerp(t, -40.0, -20.0, 0.0, 24.0) as i32
        } else {
            lerp(t, 85.0, 110.0, 24.0, 0.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!("Critical temperature: {:.2}°C", t),
        }
    } else if !(0.0..=60.0).contains(&t) {
        let score = if t < 0.0 {
            lerp(t, -20.0, 0.0, 25.0, 74.0) as i32
        } else {
            lerp(t, 60.0, 85.0, 74.0, 25.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!("Elevated temperature: {:.2}°C", t),
        }
    } else {
        let score = if t < 25.0 {
            lerp(t, 0.0, 15.0, 75.0, 100.0) as i32
        } else {
            lerp(t, 35.0, 60.0, 100.0, 75.0) as i32
        };
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!("Thermal system normal ({:.2}°C)", t),
        }
    }
}

fn assess_communications(
    record: &TelemetryRecord,
    gs_states: &std::collections::HashMap<String, GroundStationState>,
) -> SubsystemHealth {
    let loss = record.packet_loss.unwrap_or(0.0);
    let latency = record.communication_latency.unwrap_or(45.0);
    let signal = record.signal_strength.unwrap_or(-70.0);

    let linked_gs_id = record.satellite_id.linked_ground_station();
    let gs_state = gs_states.get(linked_gs_id.as_str());

    let is_degraded = loss > 5.0 || latency > 800.0 || signal < -95.0;
    let is_critical = loss > 15.0 || latency > 2000.0 || signal < -110.0;

    let loss_score = if loss > 15.0 {
        lerp(loss as f64, 15.0, 30.0, 24.0, 0.0) as i32
    } else if loss > 5.0 {
        lerp(loss as f64, 5.0, 15.0, 74.0, 25.0) as i32
    } else {
        lerp(loss as f64, 1.0, 5.0, 100.0, 75.0) as i32
    };

    let latency_score = if latency > 2000.0 {
        lerp(latency as f64, 2000.0, 5000.0, 24.0, 0.0) as i32
    } else if latency > 800.0 {
        lerp(latency as f64, 800.0, 2000.0, 74.0, 25.0) as i32
    } else {
        lerp(latency as f64, 100.0, 800.0, 100.0, 75.0) as i32
    };

    let signal_score = if signal < -110.0 {
        lerp(signal as f64, -120.0, -110.0, 0.0, 24.0) as i32
    } else if signal < -95.0 {
        lerp(signal as f64, -110.0, -95.0, 25.0, 74.0) as i32
    } else {
        lerp(signal as f64, -95.0, -75.0, 75.0, 100.0) as i32
    };

    let mut score = loss_score.min(latency_score).min(signal_score);
    let mut state = if is_critical {
        HealthState::Red
    } else if is_degraded {
        HealthState::Yellow
    } else {
        HealthState::Green
    };

    let explanation = if let Some(gs) = gs_state {
        if gs.status == orbitlogs_models::StationStatus::Outage
            || gs.status == orbitlogs_models::StationStatus::Degraded
        {
            if gs.status == orbitlogs_models::StationStatus::Outage {
                state = HealthState::Red;
                score = score.min(10);
            } else {
                state = HealthState::Yellow;
                score = score.min(45);
            }
            let prefix = if state == HealthState::Red {
                "Critical communications degradation"
            } else {
                "Communications degraded"
            };
            format!(
                "{} due to ground station {} {} (Weather: {})",
                prefix,
                gs.station_id,
                gs.status.to_string().to_lowercase(),
                gs.weather_state
            )
        } else if is_critical {
            format!("Critical comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
        } else if is_degraded {
            format!("Comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
        } else {
            format!(
                "Communications normal (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)",
                loss, latency, signal
            )
        }
    } else if is_critical {
        format!("Critical comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
    } else if is_degraded {
        format!("Comm degradation due to onboard transponder anomaly (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)", loss, latency, signal)
    } else {
        format!(
            "Communications normal (Loss: {:.1}%, Latency: {:.1}ms, Signal: {:.1}dBm)",
            loss, latency, signal
        )
    };

    SubsystemHealth {
        state,
        score,
        explanation,
    }
}

fn assess_propulsion(record: &TelemetryRecord) -> SubsystemHealth {
    let fuel = record.fuel_remaining.unwrap_or(95.0);
    let alt = record.altitude.unwrap_or(520.0);

    let is_critical = fuel < 5.0 || alt < 150.0;
    let is_degraded = fuel < 20.0 || alt < 300.0;

    let fuel_score = if fuel < 5.0 {
        lerp(fuel as f64, 0.0, 5.0, 0.0, 24.0) as i32
    } else if fuel < 20.0 {
        lerp(fuel as f64, 5.0, 20.0, 25.0, 74.0) as i32
    } else {
        lerp(fuel as f64, 20.0, 50.0, 75.0, 100.0) as i32
    };

    let alt_score = if alt < 150.0 {
        lerp(alt, 100.0, 150.0, 0.0, 24.0) as i32
    } else if alt < 300.0 {
        lerp(alt, 150.0, 300.0, 25.0, 74.0) as i32
    } else {
        lerp(alt, 300.0, 400.0, 75.0, 100.0) as i32
    };

    let score = fuel_score.min(alt_score);

    if is_critical {
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!(
                "Critical propulsion/orbital decay (Fuel: {:.1}%, Altitude: {:.1}km)",
                fuel, alt
            ),
        }
    } else if is_degraded {
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!(
                "Propulsion degraded/low orbit (Fuel: {:.1}%, Altitude: {:.1}km)",
                fuel, alt
            ),
        }
    } else {
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!(
                "Propulsion normal (Fuel: {:.1}%, Altitude: {:.1}km)",
                fuel, alt
            ),
        }
    }
}

fn assess_adcs(record: &TelemetryRecord) -> SubsystemHealth {
    let p = record.orientation_pitch.unwrap_or(0.0).abs();
    let r = record.orientation_roll.unwrap_or(0.0).abs();
    let y = record.orientation_yaw.unwrap_or(0.0).abs();
    let max_drift = p.max(r).max(y);

    if max_drift > 45.0 {
        let score = lerp(max_drift as f64, 45.0, 90.0, 24.0, 0.0) as i32;
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!(
                "Critical attitude drift (Pitch: {:.1}°, Roll: {:.1}°, Yaw: {:.1}°)",
                p, r, y
            ),
        }
    } else if max_drift > 15.0 {
        let score = lerp(max_drift as f64, 15.0, 45.0, 74.0, 25.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!(
                "Attitude drift (Pitch: {:.1}°, Roll: {:.1}°, Yaw: {:.1}°)",
                p, r, y
            ),
        }
    } else {
        let score = lerp(max_drift as f64, 5.0, 15.0, 100.0, 75.0) as i32;
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: "ADCS control loop nominal".to_string(),
        }
    }
}

fn assess_payload(record: &TelemetryRecord) -> SubsystemHealth {
    let active = record.payload_active.unwrap_or(true);
    let draw = record.payload_power_draw.unwrap_or(15.0);
    let v = record.battery_voltage.unwrap_or(13.8);

    if active && draw > 30.0 {
        let score = lerp(draw, 30.0, 50.0, 24.0, 0.0) as i32;
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!("Critical payload power draw: {:.1}W", draw),
        }
    } else if !active && v > 11.5 {
        SubsystemHealth {
            state: HealthState::Red,
            score: 15,
            explanation: "Payload is inactive despite healthy battery".to_string(),
        }
    } else if !active && v <= 11.5 {
        let score = lerp(v, 10.0, 11.5, 25.0, 74.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: "Payload auto-shutdown to protect battery".to_string(),
        }
    } else if active && draw > 20.0 {
        let score = lerp(draw, 20.0, 30.0, 74.0, 25.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!("Elevated payload power draw: {:.1}W", draw),
        }
    } else {
        let score = lerp(draw, 18.0, 20.0, 100.0, 75.0) as i32;
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!("Payload operating normally ({:.1}W)", draw),
        }
    }
}

fn assess_flight_computer(record: &TelemetryRecord) -> SubsystemHealth {
    let cpu = record.cpu_usage.unwrap_or(25.0);
    let mem = record.memory_usage.unwrap_or(45.0);
    let max_load = cpu.max(mem);

    if max_load > 95.0 {
        let score = lerp(max_load as f64, 95.0, 100.0, 24.0, 0.0) as i32;
        SubsystemHealth {
            state: HealthState::Red,
            score,
            explanation: format!(
                "Critical flight computer load (CPU: {:.1}%, Mem: {:.1}%)",
                cpu, mem
            ),
        }
    } else if max_load > 80.0 {
        let score = lerp(max_load as f64, 80.0, 95.0, 74.0, 25.0) as i32;
        SubsystemHealth {
            state: HealthState::Yellow,
            score,
            explanation: format!(
                "High flight computer load (CPU: {:.1}%, Mem: {:.1}%)",
                cpu, mem
            ),
        }
    } else {
        let score = lerp(max_load as f64, 60.0, 80.0, 100.0, 75.0) as i32;
        SubsystemHealth {
            state: HealthState::Green,
            score,
            explanation: format!(
                "Flight computer operational (CPU: {:.1}%, Mem: {:.1}%)",
                cpu, mem
            ),
        }
    }
}

pub trait SubsystemAssessor: Send + Sync {
    fn assess(
        &self,
        record: &TelemetryRecord,
        gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth;
}

pub struct PowerAssessor;
impl SubsystemAssessor for PowerAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_power(record)
    }
}

pub struct ThermalAssessor;
impl SubsystemAssessor for ThermalAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_thermal(record)
    }
}

pub struct CommunicationsAssessor;
impl SubsystemAssessor for CommunicationsAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_communications(record, gs_states)
    }
}

pub struct PropulsionAssessor;
impl SubsystemAssessor for PropulsionAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_propulsion(record)
    }
}

pub struct AdcsAssessor;
impl SubsystemAssessor for AdcsAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_adcs(record)
    }
}

pub struct PayloadAssessor;
impl SubsystemAssessor for PayloadAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_payload(record)
    }
}

pub struct FlightComputerAssessor;
impl SubsystemAssessor for FlightComputerAssessor {
    fn assess(
        &self,
        record: &TelemetryRecord,
        _gs_states: &std::collections::HashMap<String, GroundStationState>,
    ) -> SubsystemHealth {
        assess_flight_computer(record)
    }
}

pub fn assess_satellite_health(
    record: &TelemetryRecord,
    gs_states: &std::collections::HashMap<String, GroundStationState>,
) -> SatelliteHealthAssessment {
    let power = PowerAssessor.assess(record, gs_states);
    let thermal = ThermalAssessor.assess(record, gs_states);
    let communications = CommunicationsAssessor.assess(record, gs_states);
    let propulsion = PropulsionAssessor.assess(record, gs_states);
    let adcs = AdcsAssessor.assess(record, gs_states);
    let payload = PayloadAssessor.assess(record, gs_states);
    let flight_computer = FlightComputerAssessor.assess(record, gs_states);

    // Calculate overall satellite state (worst-case subsystem state)
    let overall_state = {
        if power.state == HealthState::Red
            || thermal.state == HealthState::Red
            || communications.state == HealthState::Red
            || propulsion.state == HealthState::Red
            || adcs.state == HealthState::Red
            || payload.state == HealthState::Red
            || flight_computer.state == HealthState::Red
        {
            HealthState::Red
        } else if power.state == HealthState::Yellow
            || thermal.state == HealthState::Yellow
            || communications.state == HealthState::Yellow
            || propulsion.state == HealthState::Yellow
            || adcs.state == HealthState::Yellow
            || payload.state == HealthState::Yellow
            || flight_computer.state == HealthState::Yellow
        {
            HealthState::Yellow
        } else {
            HealthState::Green
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
        if power.state != HealthState::Green {
            issues.push(format!("Power: {}", power.explanation));
        }
        if thermal.state != HealthState::Green {
            issues.push(format!("Thermal: {}", thermal.explanation));
        }
        if communications.state != HealthState::Green {
            issues.push(format!("Comms: {}", communications.explanation));
        }
        if propulsion.state != HealthState::Green {
            issues.push(format!("Propulsion: {}", propulsion.explanation));
        }
        if adcs.state != HealthState::Green {
            issues.push(format!("ADCS: {}", adcs.explanation));
        }
        if payload.state != HealthState::Green {
            issues.push(format!("Payload: {}", payload.explanation));
        }
        if flight_computer.state != HealthState::Green {
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
            satellite_id: orbitlogs_models::SatelliteId::new("SAT-0001"),
            subsystem_id: orbitlogs_models::Subsystem::Unified,
            mission_id: orbitlogs_models::MissionId::new("MISSION-ALPHA"),
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

        assert_eq!(health.overall_state, HealthState::Green);
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

        assert_eq!(health.overall_state, HealthState::Yellow);
        assert_eq!(health.thermal.state, HealthState::Yellow);
        assert_eq!(health.overall_score, 94); // (6*100 + 64) / 7 = 94
        assert!(health.explanation.contains("Thermal: Elevated temperature"));
    }

    #[test]
    fn test_power_critical_red_health() {
        let mut rec = mock_nominal_record();
        rec.battery_voltage = Some(9.5); // Red range (<10)
        let gs_states = std::collections::HashMap::new();
        let health = assess_satellite_health(&rec, &gs_states);

        assert_eq!(health.overall_state, HealthState::Red);
        assert_eq!(health.power.state, HealthState::Red);
        assert_eq!(health.overall_score, 88); // (6*100 + 18) / 7 = 88
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

        assert_eq!(health.overall_state, HealthState::Yellow);
        assert_eq!(health.payload.state, HealthState::Yellow);
        assert_eq!(health.overall_score, 89);
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
                station_id: orbitlogs_models::StationId::new("GS-0001"),
                station_name: "Svalbard".to_string(),
                status: orbitlogs_models::StationStatus::Outage,
                antenna_health: 12.5,
                weather_state: orbitlogs_models::WeatherState::Clear,
                congestion: 0.0,
                latency: 2500.0,
                bandwidth: 0.0,
                explanation: "Antenna alignment failure".to_string(),
                backhaul_health: None,
                packet_loss: None,
                signal_quality: None,
                maintenance: None,
            },
        );

        let health = assess_satellite_health(&rec, &gs_states);
        assert_eq!(health.overall_state, HealthState::Red);
        assert_eq!(health.communications.state, HealthState::Red);
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
                station_id: orbitlogs_models::StationId::new("GS-0001"),
                station_name: "Svalbard".to_string(),
                status: orbitlogs_models::StationStatus::Online,
                antenna_health: 100.0,
                weather_state: orbitlogs_models::WeatherState::Clear,
                congestion: 0.0,
                latency: 42.0,
                bandwidth: 1000.0,
                explanation: "All operational".to_string(),
                backhaul_health: None,
                packet_loss: None,
                signal_quality: None,
                maintenance: None,
            },
        );

        let health = assess_satellite_health(&rec, &gs_states);
        assert_eq!(health.overall_state, HealthState::Red);
        assert_eq!(health.communications.state, HealthState::Red);
        assert!(
            health
                .explanation
                .contains("Critical comm degradation due to onboard transponder anomaly"),
            "Explanation was: {}",
            health.explanation
        );
    }
}
