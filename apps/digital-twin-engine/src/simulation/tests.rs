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

#[cfg(test)]
mod tests {
    use super::super::*;
    use orbitlogs_models::SubsystemStatus;
    use orbitlogs_models::TelemetryRecord;
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

    #[test]
    fn test_degraded_battery_voltage() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(11.2);

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
        rec.battery_voltage = Some(10.2);
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

    #[test]
    fn test_degraded_and_failed_thermal_spike() {
        let mut rec = mock_nominal_telemetry();
        rec.temperature = Some(78.0);

        let proj1 = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj1.subsystems.thermal.status, SubsystemStatus::Degraded);
        assert_eq!(proj1.subsystems.thermal.score, 55);
        assert!(proj1
            .recommendations
            .iter()
            .any(|r| r.contains("Yaw satellite to align radiator")));
        assert_eq!(proj1.life_expectancy_days, 1800.0 - 200.0);

        rec.temperature = Some(92.0);
        let proj2 = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj2.subsystems.thermal.status, SubsystemStatus::Failed);
        assert_eq!(proj2.subsystems.thermal.score, 10);
        assert!(proj2
            .recommendations
            .iter()
            .any(|r| r.contains("Immediate reboot")));
        assert_eq!(proj2.life_expectancy_days, 1800.0 - 1000.0);
    }

    #[test]
    fn test_degraded_adcs_pitch() {
        let mut rec = mock_nominal_telemetry();
        rec.orientation_pitch = Some(12.5);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj.subsystems.adcs.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.adcs.score, 65);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("Trigger magnetorquer alignment")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 150.0);
    }

    #[test]
    fn test_degraded_propulsion_fuel() {
        let mut rec = mock_nominal_telemetry();
        rec.fuel_remaining = Some(15.0);

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
        rec.fuel_remaining = Some(3.0);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj.subsystems.propulsion.status, SubsystemStatus::Failed);
        assert_eq!(proj.subsystems.propulsion.score, 15);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("deorbit planning")));
    }

    #[test]
    fn test_degraded_communications() {
        let mut rec = mock_nominal_telemetry();
        rec.packet_loss = Some(25.0);

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
        rec.packet_loss = Some(75.0);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(
            proj.subsystems.communications.status,
            SubsystemStatus::Failed
        );
        assert_eq!(proj.subsystems.communications.score, 25);
    }

    #[test]
    fn test_degraded_flight_computer_cpu() {
        let mut rec = mock_nominal_telemetry();
        rec.cpu_usage = Some(90.0);

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
        rec.fuel_remaining = Some(-5.0);
        rec.packet_loss = Some(-10.0);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert_eq!(proj.subsystems.propulsion.status, SubsystemStatus::Failed);
        assert_eq!(
            proj.subsystems.communications.status,
            SubsystemStatus::Nominal
        );
    }

    #[test]
    fn test_multi_failure_compounding() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(10.0);
        rec.temperature = Some(95.0);
        rec.fuel_remaining = Some(2.0);

        let proj = project_twin_health("ORBITLOGS-9999".to_string(), &rec, None);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("MULTI-FAILURE ALERT")));
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
        assert!(proj.overall_score < 70);
    }

    #[test]
    fn test_digital_twin_solar_efficiency_estimation() {
        let mut twin = DigitalTwinState::new("SAT-0001".to_string());
        assert_eq!(twin.solar_efficiency, 1.0);

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

        for _ in 0..5000 {
            twin.estimate_solar_efficiency(96.0, 1.0);
        }

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
        twin.solar_efficiency = 0.7;

        let proj = project_twin_health("SAT-0001".to_string(), &rec, Some(&twin));
        assert_eq!(proj.subsystems.power.status, SubsystemStatus::Nominal);
    }
}
