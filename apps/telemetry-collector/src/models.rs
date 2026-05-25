use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
