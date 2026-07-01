use serde::{Serialize, Deserialize};
use dashmap::DashMap;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use orbitlogs_models::TelemetryRecord;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemTwin {
    pub name: String,
    pub status: String,
    pub health_score: i32,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinState {
    pub satellite_id: String,
    pub timestamp: DateTime<Utc>,
    pub subsystems: Vec<SubsystemTwin>,
    pub fuel_remaining_kg: f64,
    pub battery_charge_ah: f64,
    pub solar_pointing_angle: f64,
}

pub struct TwinManager {
    // satellite_id -> current twin state
    pub twins: DashMap<String, TelemetryRecord>,
    pub active_commands: DashMap<String, Vec<String>>, // sat_id -> commands waiting for feedback impact
}

impl TwinManager {
    pub fn new() -> Self {
        Self {
            twins: DashMap::new(),
            active_commands: DashMap::new(),
        }
    }

    pub fn update_twin(&self, sat_id: &str, mut rec: TelemetryRecord) -> TelemetryRecord {
        // Intercept and apply command impacts on the telemetry record
        if let Some(cmds) = self.active_commands.get(sat_id) {
            for cmd in cmds.value() {
                match cmd.as_str() {
                    "CMD_SHED_LOAD_SECONDARY" | "CMD_PAYLOAD_DEACTIVATE" => {
                        rec.payload_active = Some(false);
                        rec.payload_power_draw = Some(0.0);
                        rec.cpu_usage = Some(rec.cpu_usage.unwrap_or(20.0) * 0.7);
                    }
                    "CMD_ENTER_SAFE_MODE" => {
                        rec.payload_active = Some(false);
                        rec.payload_power_draw = Some(0.0);
                        rec.cpu_usage = Some(10.0); // minimal flight computer
                        rec.temperature = Some(rec.temperature.unwrap_or(20.0) * 0.8);
                    }
                    "CMD_RADIATOR_YAW_ADJUST" => {
                        // Cold space radiation pointing
                        rec.temperature = Some(rec.temperature.unwrap_or(25.0) * 0.9);
                    }
                    "CMD_TTC_POWER_BOOST" => {
                        rec.signal_strength = Some(rec.signal_strength.unwrap_or(-80.0) + 12.0);
                        rec.packet_loss = Some(rec.packet_loss.unwrap_or(0.0) * 0.2);
                    }
                    "CMD_PROP_THRUST" => {
                        if let Some(f) = rec.fuel_remaining {
                            rec.fuel_remaining = Some((f - 1.5).max(0.0));
                        }
                        rec.velocity = Some(rec.velocity.unwrap_or(7.6) + 0.15);
                    }
                    _ => {}
                }
            }
        }

        // Apply realistic failure cascades
        let temp = rec.temperature.unwrap_or(20.0);
        let battery = rec.battery_voltage.unwrap_or(13.8);

        // Cascade 1: Thermal spike causes CPU throttling & ADCS drift
        if temp > 70.0 {
            rec.cpu_usage = Some(rec.cpu_usage.unwrap_or(15.0) + 30.0);
            rec.orientation_pitch = Some(rec.orientation_pitch.unwrap_or(0.0) + 8.5); // drift
        }

        // Cascade 2: Battery drop triggers payload shutdown
        if battery < 11.0 {
            rec.payload_active = Some(false);
            rec.payload_power_draw = Some(0.0);
        }

        self.twins.insert(sat_id.to_string(), rec.clone());
        rec
    }

    pub fn apply_command(&self, sat_id: &str, cmd_type: &str) {
        self.active_commands.entry(sat_id.to_string()).or_insert_with(Vec::new).push(cmd_type.to_string());
    }

    pub fn clear_commands(&self, sat_id: &str) {
        self.active_commands.remove(sat_id);
    }
}

pub async fn get_twin_model(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<TwinState>, StatusCode> {
    let rec = state.twin.twins.get(&satellite).ok_or(StatusCode::NOT_FOUND)?.clone();

    // Model structured parameters
    let subsystems = vec![
        SubsystemTwin {
            name: "EPS (Power)".to_string(),
            status: if rec.battery_voltage.unwrap_or(13.5) < 11.5 { "DEGRADED".to_string() } else { "NOMINAL".to_string() },
            health_score: if rec.battery_voltage.unwrap_or(13.5) < 11.5 { 60 } else { 100 },
            parameters: serde_json::json!({
                "voltage": rec.battery_voltage,
                "current": rec.battery_current,
                "solar_generation_w": rec.solar_panel_output,
            }),
        },
        SubsystemTwin {
            name: "TCS (Thermal)".to_string(),
            status: if rec.temperature.unwrap_or(20.0) > 65.0 { "DEGRADED".to_string() } else { "NOMINAL".to_string() },
            health_score: if rec.temperature.unwrap_or(20.0) > 65.0 { 50 } else { 100 },
            parameters: serde_json::json!({
                "core_temp_c": rec.temperature,
            }),
        },
        SubsystemTwin {
            name: "ADCS (Attitude)".to_string(),
            status: if rec.orientation_pitch.unwrap_or(0.0).abs() > 10.0 { "DEGRADED".to_string() } else { "NOMINAL".to_string() },
            health_score: if rec.orientation_pitch.unwrap_or(0.0).abs() > 10.0 { 65 } else { 100 },
            parameters: serde_json::json!({
                "pitch_err": rec.orientation_pitch,
                "roll": rec.orientation_roll,
                "yaw": rec.orientation_yaw,
            }),
        },
    ];

    Ok(Json(TwinState {
        satellite_id: satellite,
        timestamp: Utc::now(),
        subsystems,
        fuel_remaining_kg: rec.fuel_remaining.unwrap_or(100.0) as f64,
        battery_charge_ah: rec.battery_voltage.unwrap_or(12.5) * 5.0, // approx multiplier
        solar_pointing_angle: rec.orientation_pitch.unwrap_or(0.0) as f64,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A helper function to create a default TelemetryRecord
    fn default_telemetry_record(sat_id: &str) -> TelemetryRecord {
        TelemetryRecord {
            event_id: uuid::Uuid::new_v4(),
            satellite_id: sat_id.to_string(),
            mission_id: "test-mission".to_string(),
            timestamp: Utc::now(),
            subsystem_id: "test-subsystem".to_string(),
            battery_voltage: Some(13.8),
            battery_current: Some(1.5),
            solar_panel_output: Some(120.0),
            cpu_usage: Some(25.0),
            memory_usage: Some(30.0),
            temperature: Some(22.0),
            orientation_pitch: Some(0.0),
            orientation_roll: Some(0.0),
            orientation_yaw: Some(0.0),
            velocity: Some(7.6),
            altitude: Some(500.0),
            fuel_remaining: Some(85.0),
            communication_latency: Some(2.5),
            packet_loss: Some(0.5),
            signal_strength: Some(-75.0),
            payload_power_draw: Some(15.0),
            payload_active: Some(true),
        }
    }

    #[test]
    fn test_twin_manager_new() {
        let manager = TwinManager::new();
        assert!(manager.twins.is_empty());
        assert!(manager.active_commands.is_empty());
    }

    #[test]
    fn test_apply_and_clear_commands() {
        let manager = TwinManager::new();
        manager.apply_command("SAT-001", "CMD_SHED_LOAD_SECONDARY");
        manager.apply_command("SAT-001", "CMD_RADIATOR_YAW_ADJUST");

        {
            let cmds = manager.active_commands.get("SAT-001").unwrap();
            assert_eq!(cmds.len(), 2);
            assert_eq!(cmds[0], "CMD_SHED_LOAD_SECONDARY");
            assert_eq!(cmds[1], "CMD_RADIATOR_YAW_ADJUST");
        }

        manager.clear_commands("SAT-001");
        assert!(manager.active_commands.get("SAT-001").is_none());
    }

    #[test]
    fn test_update_twin_nominal() {
        let manager = TwinManager::new();
        let rec = default_telemetry_record("SAT-001");
        let updated = manager.update_twin("SAT-001", rec.clone());

        assert_eq!(updated.satellite_id, "SAT-001");
        // Verify twin was cached
        let cached = manager.twins.get("SAT-001").unwrap();
        assert_eq!(cached.cpu_usage, rec.cpu_usage);
    }

    #[test]
    fn test_update_twin_with_commands() {
        let manager = TwinManager::new();
        let sat_id = "SAT-CMD";

        // 1. Shed load command
        manager.apply_command(sat_id, "CMD_SHED_LOAD_SECONDARY");
        let rec = default_telemetry_record(sat_id);
        let updated = manager.update_twin(sat_id, rec);
        assert_eq!(updated.payload_active, Some(false));
        assert_eq!(updated.payload_power_draw, Some(0.0));
        assert!(updated.cpu_usage.unwrap() < 25.0); // should be 25.0 * 0.7 = 17.5

        // Reset and test safe mode
        manager.clear_commands(sat_id);
        manager.apply_command(sat_id, "CMD_ENTER_SAFE_MODE");
        let rec = default_telemetry_record(sat_id);
        let updated = manager.update_twin(sat_id, rec);
        assert_eq!(updated.payload_active, Some(false));
        assert_eq!(updated.payload_power_draw, Some(0.0));
        assert_eq!(updated.cpu_usage, Some(10.0));
        assert_eq!(updated.temperature, Some(22.0 * 0.8));

        // Test radiator yaw adjustment
        manager.clear_commands(sat_id);
        manager.apply_command(sat_id, "CMD_RADIATOR_YAW_ADJUST");
        let rec = default_telemetry_record(sat_id);
        let updated = manager.update_twin(sat_id, rec);
        assert_eq!(updated.temperature, Some(22.0 * 0.9));

        // Test telemetry boost
        manager.clear_commands(sat_id);
        manager.apply_command(sat_id, "CMD_TTC_POWER_BOOST");
        let rec = default_telemetry_record(sat_id);
        let updated = manager.update_twin(sat_id, rec);
        assert_eq!(updated.signal_strength, Some(-63.0)); // -75.0 + 12.0
        assert_eq!(updated.packet_loss, Some(0.1));       // 0.5 * 0.2

        // Test propulsion thrust
        manager.clear_commands(sat_id);
        manager.apply_command(sat_id, "CMD_PROP_THRUST");
        let rec = default_telemetry_record(sat_id);
        let updated = manager.update_twin(sat_id, rec);
        assert_eq!(updated.fuel_remaining, Some(83.5)); // 85.0 - 1.5
        assert_eq!(updated.velocity, Some(7.6 + 0.15));
    }

    #[test]
    fn test_update_twin_failure_cascades() {
        let manager = TwinManager::new();
        let sat_id = "SAT-FAIL";

        // 1. Thermal spike
        let mut rec = default_telemetry_record(sat_id);
        rec.temperature = Some(75.0);
        let updated = manager.update_twin(sat_id, rec);
        assert!(updated.cpu_usage.unwrap() > 25.0);       // should be 25.0 + 30.0 = 55.0
        assert_eq!(updated.orientation_pitch, Some(8.5)); // 0.0 + 8.5

        // 2. Low battery
        let mut rec = default_telemetry_record(sat_id);
        rec.battery_voltage = Some(10.5);
        let updated = manager.update_twin(sat_id, rec);
        assert_eq!(updated.payload_active, Some(false));
        assert_eq!(updated.payload_power_draw, Some(0.0));
    }
}

