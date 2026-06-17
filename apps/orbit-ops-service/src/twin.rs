use serde::{Serialize, Deserialize};
use dashmap::DashMap;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use spaceops_models::TelemetryRecord;
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
