

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn default_subsystem() -> String {
    "unified".to_string()
}

fn default_mission() -> String {
    "unknown".to_string()
}

/// Canonical satellite telemetry record shared across all Open SpaceOps services.
///
/// This struct represents a single telemetry snapshot from a satellite subsystem, containing
/// subsystem-specific electrical power, attitude, or operational parameters.
///
/// # Validation
/// The `validate()` method enforces physical bounds checking on all present fields to reject
/// payloads that fall outside plausible satellite operational ranges.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryRecord {
    pub satellite_id: String,

    #[serde(default = "default_subsystem")]
    pub subsystem_id: String,

    #[serde(default = "default_mission")]
    pub mission_id: String,

    pub timestamp: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_voltage: Option<f64>, // Volts

    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_current: Option<f64>, // Amperes

    #[serde(skip_serializing_if = "Option::is_none")]
    pub solar_panel_output: Option<f64>, // Watts

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_usage: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>, // °C

    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_pitch: Option<f32>, // Degrees

    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_roll: Option<f32>, // Degrees

    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_yaw: Option<f32>, // Degrees

    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<f64>, // km/s

    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>, // km

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_remaining: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication_latency: Option<f32>, // ms

    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_strength: Option<f32>, // dBm

    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_power_draw: Option<f64>, // Watts

    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_active: Option<bool>,

    pub event_id: Uuid,
}

impl TelemetryRecord {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.satellite_id.trim().is_empty() {
            errors.push("satellite_id cannot be empty".to_string());
        } else if !self
            .satellite_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-')
        {
            errors.push("satellite_id must be alphanumeric or contain dashes".to_string());
        }

        if let Some(battery_voltage) = self.battery_voltage {
            if !(0.0..=100.0).contains(&battery_voltage) {
                errors.push(format!(
                    "battery_voltage must be between 0 and 100, got {}",
                    battery_voltage
                ));
            }
        }

        if let Some(cpu_usage) = self.cpu_usage {
            if !(0.0..=100.0).contains(&cpu_usage) {
                errors.push(format!(
                    "cpu_usage must be between 0 and 100, got {}",
                    cpu_usage
                ));
            }
        }

        if let Some(memory_usage) = self.memory_usage {
            if !(0.0..=100.0).contains(&memory_usage) {
                errors.push(format!(
                    "memory_usage must be between 0 and 100, got {}",
                    memory_usage
                ));
            }
        }

        if let Some(fuel_remaining) = self.fuel_remaining {
            if !(0.0..=100.0).contains(&fuel_remaining) {
                errors.push(format!(
                    "fuel_remaining must be between 0 and 100, got {}",
                    fuel_remaining
                ));
            }
        }

        if let Some(packet_loss) = self.packet_loss {
            if !(0.0..=100.0).contains(&packet_loss) {
                errors.push(format!(
                    "packet_loss must be between 0 and 100, got {}",
                    packet_loss
                ));
            }
        }

        if let Some(orientation_pitch) = self.orientation_pitch {
            if !(-360.0..=360.0).contains(&orientation_pitch) {
                errors.push(format!(
                    "orientation_pitch must be between -360 and 360, got {}",
                    orientation_pitch
                ));
            }
        }

        if let Some(orientation_roll) = self.orientation_roll {
            if !(-360.0..=360.0).contains(&orientation_roll) {
                errors.push(format!(
                    "orientation_roll must be between -360 and 360, got {}",
                    orientation_roll
                ));
            }
        }

        if let Some(orientation_yaw) = self.orientation_yaw {
            if !(-360.0..=360.0).contains(&orientation_yaw) {
                errors.push(format!(
                    "orientation_yaw must be between -360 and 360, got {}",
                    orientation_yaw
                ));
            }
        }

        if self.timestamp > Utc::now() + chrono::Duration::minutes(5) {
            errors.push("Timestamp cannot be ahead of current time".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundStationHealth {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backhaul_health: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_quality: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<bool>,
}

pub fn load_secret(env_var: &str) -> Option<String> {
    if let Ok(val) = std::env::var(env_var) {
        if !val.trim().is_empty() {
            return Some(val.trim().to_string());
        }
    }
    if let Ok(file_path) = std::env::var(format!("{}_FILE", env_var)) {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            return Some(content.trim().to_string());
        }
    }
    None
}
pub mod auth;
pub mod db;
pub mod kafka;
pub mod telemetry;

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_valid_record() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: "SAT-0001".to_string(),
            subsystem_id: "unified".to_string(),
            mission_id: "unknown".to_string(),
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
    fn test_valid_payload() {
        let rec = mock_valid_record();
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_invalid_satellite_id() {
        let mut rec = mock_valid_record();
        rec.satellite_id = "".to_string();
        assert!(rec.validate().is_err());

        rec.satellite_id = "SAT#01".to_string();
        assert!(rec.validate().is_err());
    }

    #[test]
    fn test_invalid_voltages() {
        let mut rec = mock_valid_record();
        rec.battery_voltage = Some(-1.2);
        assert!(rec.validate().is_err());

        rec.battery_voltage = Some(101.5);
        assert!(rec.validate().is_err());
    }

    #[test]
    fn test_invalid_usages() {
        let mut rec = mock_valid_record();
        rec.cpu_usage = Some(105.0);
        assert!(rec.validate().is_err());

        rec.memory_usage = Some(-5.0);
        assert!(rec.validate().is_err());
    }

    #[test]
    fn test_serde_deny_unknown_fields() {
        let json_with_extra = r#"{
            "satellite_id": "SAT-0001",
            "subsystem_id": "unified",
            "mission_id": "unknown",
            "timestamp": "2026-01-01T00:00:00Z",
            "battery_voltage": 13.8,
            "battery_current": 2.1,
            "solar_panel_output": 110.0,
            "cpu_usage": 25.4,
            "memory_usage": 45.1,
            "temperature": 28.5,
            "orientation_pitch": 1.2,
            "orientation_roll": -2.3,
            "orientation_yaw": 0.5,
            "velocity": 7.67,
            "altitude": 520.4,
            "fuel_remaining": 92.5,
            "communication_latency": 35.1,
            "packet_loss": 0.1,
            "signal_strength": -68.0,
            "payload_power_draw": 15.0,
            "payload_active": true,
            "event_id": "550e8400-e29b-41d4-a716-446655440000",
            "unknown_field": "should_be_rejected"
        }"#;

        let result: Result<TelemetryRecord, _> = serde_json::from_str(json_with_extra);
        assert!(result.is_err(), "Unknown fields must be rejected");
    }

    #[test]
    fn test_serde_roundtrip() {
        let rec = mock_valid_record();
        let json = serde_json::to_string(&rec).unwrap();
        let deserialized: TelemetryRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.satellite_id, deserialized.satellite_id);
        assert_eq!(rec.battery_voltage, deserialized.battery_voltage);
        assert_eq!(rec.temperature, deserialized.temperature);
        assert_eq!(rec.subsystem_id, deserialized.subsystem_id);
        assert_eq!(rec.mission_id, deserialized.mission_id);
    }

    #[test]
    fn test_serde_defaults() {
        let json_minimal = r#"{
            "satellite_id": "SAT-0001",
            "timestamp": "2026-01-01T00:00:00Z",
            "event_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;

        let deserialized: TelemetryRecord = serde_json::from_str(json_minimal).unwrap();
        assert_eq!(deserialized.subsystem_id, "unified");
        assert_eq!(deserialized.mission_id, "unknown");
        assert!(deserialized.battery_voltage.is_none());
    }

    #[test]
    fn test_anomaly_threshold_thermal_spike() {
        let mut rec = mock_valid_record();
        rec.temperature = Some(86.0);
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_anomaly_threshold_battery_degradation() {
        let mut rec = mock_valid_record();
        rec.battery_voltage = Some(10.5);
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_anomaly_threshold_fuel_low() {
        let mut rec = mock_valid_record();
        rec.fuel_remaining = Some(14.0);
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_anomaly_threshold_comm_degradation() {
        let mut rec = mock_valid_record();
        rec.packet_loss = Some(12.0);
        rec.communication_latency = Some(1200.0);
        assert!(rec.validate().is_ok());
    }
}
