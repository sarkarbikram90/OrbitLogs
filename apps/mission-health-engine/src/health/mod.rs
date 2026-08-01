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

#![allow(
    clippy::upper_case_acronyms,
    clippy::wrong_self_convention,
    clippy::inherent_to_string
)]

pub mod assessors;
pub mod types;

pub use assessors::*;
use orbitlogs_models::TelemetryRecord;
pub use types::*;

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

    let overall_score = (power.score
        + thermal.score
        + communications.score
        + propulsion.score
        + adcs.score
        + payload.score
        + flight_computer.score)
        / 7;

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
    use chrono::Utc;
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
        rec.temperature = Some(65.0);
        let gs_states = std::collections::HashMap::new();
        let health = assess_satellite_health(&rec, &gs_states);

        assert_eq!(health.overall_state, HealthState::Yellow);
        assert_eq!(health.thermal.state, HealthState::Yellow);
        assert_eq!(health.overall_score, 94);
        assert!(health.explanation.contains("Thermal: Elevated temperature"));
    }

    #[test]
    fn test_power_critical_red_health() {
        let mut rec = mock_nominal_record();
        rec.battery_voltage = Some(9.5);
        let gs_states = std::collections::HashMap::new();
        let health = assess_satellite_health(&rec, &gs_states);

        assert_eq!(health.overall_state, HealthState::Red);
        assert_eq!(health.power.state, HealthState::Red);
        assert_eq!(health.overall_score, 88);
        assert!(health
            .explanation
            .contains("Power: Critical battery voltage"));
    }

    #[test]
    fn test_payload_degraded_battery_dependency() {
        let mut rec = mock_nominal_record();
        rec.battery_voltage = Some(11.2);
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
        assert!(health.explanation.contains("Critical communications degradation due to ground station GS-0001 outage (Weather: Clear)"));
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
        assert!(health
            .explanation
            .contains("Critical comm degradation due to onboard transponder anomaly"));
    }
}
