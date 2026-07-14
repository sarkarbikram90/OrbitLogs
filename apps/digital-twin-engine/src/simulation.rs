use orbitlogs_models::TelemetryRecord;
use serde::Serialize;

// Threshold Constants for Health Projections
pub const BATTERY_WARN_VOLTAGE: f64 = 11.5;
pub const BATTERY_CRIT_VOLTAGE: f64 = 10.5;
pub const THERMAL_WARN_TEMP: f64 = 75.0;
pub const THERMAL_CRIT_TEMP: f64 = 90.0;
pub const ADCS_WARN_PITCH_DRIFT: f32 = 10.0;
pub const COMM_WARN_PACKET_LOSS: f32 = 10.0;
pub const PROPULSION_WARN_FUEL: f32 = 20.0;

#[derive(Debug, Serialize)]
pub struct TwinHealthProjection {
    pub satellite_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub subsystems: SubsystemsHealthProjection,
    pub life_expectancy_days: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SubsystemsHealthProjection {
    pub power: SubsystemState,
    pub thermal: SubsystemState,
    pub adcs: SubsystemState,
    pub communications: SubsystemState,
    pub payload: SubsystemState,
    pub propulsion: SubsystemState,
}

#[derive(Debug, Serialize)]
pub struct SubsystemState {
    pub status: orbitlogs_models::SubsystemStatus,
    pub score: i32,
    pub details: String,
}

pub fn project_twin_health(satellite: String, rec: &TelemetryRecord) -> TwinHealthProjection {
    use orbitlogs_models::SubsystemStatus;

    // Formulate a dynamic projection based on twin metrics
    let mut recommendations = Vec::new();
    let mut life_expectancy: f64 = 1800.0; // nominal 5 years

    // Power Subsystem Assessment
    let mut power_status = SubsystemStatus::Nominal;
    let mut power_score = 100;
    let mut power_details =
        "Solar arrays fully deployed and generating adequate wattage. Battery banks balanced."
            .to_string();

    let battery = rec.battery_voltage.unwrap_or(13.8);
    let solar = rec.solar_panel_output.unwrap_or(100.0);
    if battery < BATTERY_WARN_VOLTAGE {
        power_status = SubsystemStatus::Degraded;
        power_score = 60;
        power_details = format!(
            "Battery voltage low: {}V. Solar generating: {}W.",
            battery, solar
        );
        recommendations.push(
            "Charge cycle optimization: Disable secondary payloads to reduce battery strain."
                .to_string(),
        );
        life_expectancy -= 300.0;
    }
    if battery < BATTERY_CRIT_VOLTAGE || solar == 0.0 {
        power_status = SubsystemStatus::Failed;
        power_score = 20;
        power_details =
            "Battery under-voltage lockout threshold reached or solar array generation failure."
                .to_string();
        recommendations
            .push("Emergency power mode: Shed all non-vital systems immediately.".to_string());
        life_expectancy -= 1000.0;
    }

    // Thermal Subsystem Assessment
    let mut thermal_status = SubsystemStatus::Nominal;
    let mut thermal_score = 100;
    let mut thermal_details =
        "Radiators operating within design bounds. Internal temperatures nominal.".to_string();

    let temp = rec.temperature.unwrap_or(25.0);
    if temp > THERMAL_WARN_TEMP {
        thermal_status = SubsystemStatus::Degraded;
        thermal_score = 55;
        thermal_details = format!(
            "Thermal radiator capacity constrained. Component temperature: {}°C.",
            temp
        );
        recommendations.push("Yaw satellite to align radiator arrays with cold space.".to_string());
        life_expectancy -= 200.0;
    }
    if temp > THERMAL_CRIT_TEMP {
        thermal_status = SubsystemStatus::Failed;
        thermal_score = 10;
        thermal_details = format!(
            "Critical hardware thermal limit exceeded: {}°C. Risk of immediate chip fusion.",
            temp
        );
        recommendations
            .push("Immediate reboot and deep power saving to prevent silicon decay.".to_string());
        life_expectancy -= 800.0;
    }

    // ADCS Assessment
    let mut adcs_status = SubsystemStatus::Nominal;
    let mut adcs_score = 100;
    let mut adcs_details =
        "Attitude control holding target star lock. Reaction wheel torque minimal.".to_string();

    let pitch = rec.orientation_pitch.unwrap_or(0.0);
    if pitch.abs() > ADCS_WARN_PITCH_DRIFT {
        adcs_status = SubsystemStatus::Degraded;
        adcs_score = 65;
        adcs_details = format!(
            "Attitude drift detected. Star tracker lock unstable. Pitch error: {:.1}°.",
            pitch
        );
        recommendations.push(
            "Trigger magnetorquer alignment sequence to dump reaction wheel momentum.".to_string(),
        );
        life_expectancy -= 150.0;
    }

    // Communications Assessment
    let mut comm_status = SubsystemStatus::Nominal;
    let mut comm_score = 100;
    let mut comm_details =
        "Uplink and downlink carriers locked. Bit error rate at standard noise floor.".to_string();

    let ploss = rec.packet_loss.unwrap_or(0.0);
    if ploss > COMM_WARN_PACKET_LOSS {
        comm_status = SubsystemStatus::Degraded;
        comm_score = 70;
        comm_details = format!(
            "Uplink jitter detected. Packet loss: {:.1}%. Link margin tight.",
            ploss
        );
        recommendations.push(
            "Increase transmitter power or schedule passes with high-gain ground station arrays."
                .to_string(),
        );
    }

    // Propulsion Assessment
    let mut prop_status = SubsystemStatus::Nominal;
    let mut prop_score = 100;
    let mut prop_details =
        "Fuel levels adequate for scheduled orbital maintenance maneuvers.".to_string();

    let fuel = rec.fuel_remaining.unwrap_or(100.0);
    if fuel < PROPULSION_WARN_FUEL {
        prop_status = SubsystemStatus::Degraded;
        prop_score = 40;
        prop_details = format!(
            "Propellant supply depleted: {:.1}%. Maneuver window constrained.",
            fuel
        );
        recommendations
            .push("Transition to high-efficiency sparse burns. Plan orbit decay path.".to_string());
        life_expectancy -= 500.0;
    }

    // Payload Assessment
    let mut pay_status = SubsystemStatus::Nominal;
    let mut pay_score = 100;
    let mut pay_details =
        "Payload instrument telemetry streams active. Calibration logs clean.".to_string();

    if rec.payload_active.unwrap_or(false) && rec.payload_power_draw.unwrap_or(0.0) < 2.0 {
        pay_status = SubsystemStatus::Degraded;
        pay_score = 50;
        pay_details =
            "Payload active but drawing minimal power. Sensor integration loop suspended."
                .to_string();
        recommendations.push("Perform payload diagnostics self-test.".to_string());
    }

    TwinHealthProjection {
        satellite_id: satellite,
        timestamp: chrono::Utc::now(),
        subsystems: SubsystemsHealthProjection {
            power: SubsystemState {
                status: power_status,
                score: power_score,
                details: power_details,
            },
            thermal: SubsystemState {
                status: thermal_status,
                score: thermal_score,
                details: thermal_details,
            },
            adcs: SubsystemState {
                status: adcs_status,
                score: adcs_score,
                details: adcs_details,
            },
            communications: SubsystemState {
                status: comm_status,
                score: comm_score,
                details: comm_details,
            },
            payload: SubsystemState {
                status: pay_status,
                score: pay_score,
                details: pay_details,
            },
            propulsion: SubsystemState {
                status: prop_status,
                score: prop_score,
                details: prop_details,
            },
        },
        life_expectancy_days: life_expectancy.max(0.0),
        recommendations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbitlogs_models::SubsystemStatus;
    use uuid::Uuid;

    fn mock_nominal_telemetry() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: orbitlogs_models::SatelliteId::new("STARLINK-9999"),
            subsystem_id: orbitlogs_models::Subsystem::Adcs,
            mission_id: orbitlogs_models::MissionId::new("STARLINK-V2"),
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
        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);

        assert_eq!(proj.satellite_id, "STARLINK-9999");
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
        assert_eq!(proj.life_expectancy_days, 1800.0);
        assert!(proj.recommendations.is_empty());
    }

    #[test]
    fn test_degraded_battery_voltage() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(11.2); // < 11.5

        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);
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

        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj.subsystems.power.status, SubsystemStatus::Failed);
        assert_eq!(proj.subsystems.power.score, 20);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("Emergency power mode")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 300.0 - 1000.0);
    }

    #[test]
    fn test_degraded_and_failed_thermal_spike() {
        let mut rec = mock_nominal_telemetry();
        rec.temperature = Some(78.0); // > 75.0

        let proj1 = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj1.subsystems.thermal.status, SubsystemStatus::Degraded);
        assert_eq!(proj1.subsystems.thermal.score, 55);
        assert!(proj1
            .recommendations
            .iter()
            .any(|r| r.contains("Yaw satellite to align radiator")));
        assert_eq!(proj1.life_expectancy_days, 1800.0 - 200.0);

        rec.temperature = Some(92.0); // > 90.0
        let proj2 = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj2.subsystems.thermal.status, SubsystemStatus::Failed);
        assert_eq!(proj2.subsystems.thermal.score, 10);
        assert!(proj2
            .recommendations
            .iter()
            .any(|r| r.contains("Immediate reboot")));
        assert_eq!(proj2.life_expectancy_days, 1800.0 - 200.0 - 800.0);
    }

    #[test]
    fn test_degraded_adcs_pitch() {
        let mut rec = mock_nominal_telemetry();
        rec.orientation_pitch = Some(12.5); // > 10.0

        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);
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
        rec.fuel_remaining = Some(15.0); // < 20.0

        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj.subsystems.propulsion.status, SubsystemStatus::Degraded);
        assert_eq!(proj.subsystems.propulsion.score, 40);
        assert!(proj
            .recommendations
            .iter()
            .any(|r| r.contains("Transition to high-efficiency sparse burns")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 500.0);
    }

    #[test]
    fn test_digital_twin_solar_efficiency_estimation() {
        let mut twin = DigitalTwinState::new("SAT-0001".to_string());
        assert_eq!(twin.solar_efficiency, 1.0);
        
        // Simulating actual output of 96W when theoretical max should be 120W (exposure factor = 1.0)
        // observed_efficiency = 96 / 120 = 0.8
        twin.estimate_solar_efficiency(96.0, 1.0);
        assert!(twin.solar_efficiency < 1.0);
        assert!((twin.solar_efficiency - 0.9998).abs() < 1e-4);
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
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

    /// Recursively updates parameters based on actual solar generation telemetry.
    pub fn estimate_solar_efficiency(&mut self, actual_solar_w: f32, solar_exposure_factor: f32) {
        if solar_exposure_factor > 0.1 {
            let theoretical_max = 120.0 * solar_exposure_factor; // nominal maximum output
            let observed_efficiency = actual_solar_w / theoretical_max;
            
            // Apply recursive low-pass filter (learning factor alpha = 0.001)
            self.solar_efficiency = 0.999 * self.solar_efficiency + 0.001 * observed_efficiency;
        }
    }

    /// Estimate battery capacity fade recursively based on voltage drop
    pub fn estimate_battery_capacity_fade(&mut self, discharge_current: f32, voltage_drop: f32) {
        if discharge_current > 0.5 {
            let observed_resistance = voltage_drop / discharge_current;
            let fade_factor = 0.1 / observed_resistance.max(0.01);
            self.battery_capacity_fade = 0.999 * self.battery_capacity_fade + 0.001 * fade_factor.min(1.0);
        }
    }

    /// Estimate radiator thermal resistance recursively based on temperature and power draw
    pub fn estimate_radiator_thermal_resistance(&mut self, temp_c: f32, cpu_power_w: f32) {
        if cpu_power_w > 5.0 {
            let delta_t = temp_c - 3.0; // background space temp is around 3C in shadow/sats
            let observed_r_th = delta_t / cpu_power_w;
            let ratio = observed_r_th / 0.5;
            self.radiator_thermal_resistance = 0.999 * self.radiator_thermal_resistance + 0.001 * ratio;
        }
    }
}

