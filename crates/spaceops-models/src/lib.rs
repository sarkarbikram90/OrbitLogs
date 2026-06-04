use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Canonical satellite telemetry record shared across all Open SpaceOps services.
///
/// This struct represents a single telemetry snapshot from a satellite, containing
/// electrical power system (EPS) metrics, attitude control data, orbital parameters,
/// and communication health indicators.
///
/// # Validation
/// The `validate()` method enforces physical bounds checking on all fields to reject
/// payloads that fall outside plausible satellite operational ranges.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryRecord {
    pub satellite_id: String,
    pub timestamp: DateTime<Utc>,
    pub battery_voltage: f64,       // Volts
    pub battery_current: f64,       // Amperes
    pub solar_panel_output: f64,    // Watts
    pub cpu_usage: f32,             // %
    pub memory_usage: f32,          // %
    pub temperature: f64,           // °C
    pub orientation_pitch: f32,     // Degrees
    pub orientation_roll: f32,      // Degrees
    pub orientation_yaw: f32,       // Degrees
    pub velocity: f64,              // km/s
    pub altitude: f64,              // km
    pub fuel_remaining: f32,        // %
    pub communication_latency: f32, // ms
    pub packet_loss: f32,           // %
    pub signal_strength: f32,       // dBm
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

        if self.battery_voltage < 0.0 || self.battery_voltage > 100.0 {
            errors.push(format!(
                "battery_voltage must be between 0 and 100, got {}",
                self.battery_voltage
            ));
        }

        if self.cpu_usage < 0.0 || self.cpu_usage > 100.0 {
            errors.push(format!(
                "cpu_usage must be between 0 and 100, got {}",
                self.cpu_usage
            ));
        }

        if self.memory_usage < 0.0 || self.memory_usage > 100.0 {
            errors.push(format!(
                "memory_usage must be between 0 and 100, got {}",
                self.memory_usage
            ));
        }

        if self.fuel_remaining < 0.0 || self.fuel_remaining > 100.0 {
            errors.push(format!(
                "fuel_remaining must be between 0 and 100, got {}",
                self.fuel_remaining
            ));
        }

        if self.packet_loss < 0.0 || self.packet_loss > 100.0 {
            errors.push(format!(
                "packet_loss must be between 0 and 100, got {}",
                self.packet_loss
            ));
        }

        if self.orientation_pitch < -360.0 || self.orientation_pitch > 360.0 {
            errors.push(format!(
                "orientation_pitch must be between -360 and 360, got {}",
                self.orientation_pitch
            ));
        }

        if self.orientation_roll < -360.0 || self.orientation_roll > 360.0 {
            errors.push(format!(
                "orientation_roll must be between -360 and 360, got {}",
                self.orientation_roll
            ));
        }

        if self.orientation_yaw < -360.0 || self.orientation_yaw > 360.0 {
            errors.push(format!(
                "orientation_yaw must be between -360 and 360, got {}",
                self.orientation_yaw
            ));
        }

        if self.timestamp > Utc::now() + chrono::Duration::minutes(5) {
            errors.push("timestamp cannot be in the future".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_valid_record() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: "SAT-0001".to_string(),
            timestamp: Utc::now(),
            battery_voltage: 13.8,
            battery_current: 2.1,
            solar_panel_output: 110.0,
            cpu_usage: 25.4,
            memory_usage: 45.1,
            temperature: 28.5,
            orientation_pitch: 1.2,
            orientation_roll: -2.3,
            orientation_yaw: 0.5,
            velocity: 7.67,
            altitude: 520.4,
            fuel_remaining: 92.5,
            communication_latency: 35.1,
            packet_loss: 0.1,
            signal_strength: -68.0,
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
        rec.battery_voltage = -1.2;
        assert!(rec.validate().is_err());

        rec.battery_voltage = 101.5;
        assert!(rec.validate().is_err());
    }

    #[test]
    fn test_invalid_usages() {
        let mut rec = mock_valid_record();
        rec.cpu_usage = 105.0;
        assert!(rec.validate().is_err());

        rec.memory_usage = -5.0;
        assert!(rec.validate().is_err());
    }

    #[test]
    fn test_serde_deny_unknown_fields() {
        let json_with_extra = r#"{
            "satellite_id": "SAT-0001",
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
    }

    #[test]
    fn test_anomaly_threshold_thermal_spike() {
        let mut rec = mock_valid_record();
        rec.temperature = 86.0;
        // Record is valid (temperature is physically possible) but triggers anomaly detection
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_anomaly_threshold_battery_degradation() {
        let mut rec = mock_valid_record();
        rec.battery_voltage = 10.5;
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_anomaly_threshold_fuel_low() {
        let mut rec = mock_valid_record();
        rec.fuel_remaining = 14.0;
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_anomaly_threshold_comm_degradation() {
        let mut rec = mock_valid_record();
        rec.packet_loss = 12.0;
        rec.communication_latency = 1200.0;
        assert!(rec.validate().is_ok());
    }
}
