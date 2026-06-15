use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TelemetryVersion {
    V1,
    V2,
    V3,
}

#[derive(Error, Debug)]
pub enum SchemaError {
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid type for field {0}: expected {1}, got {2}")]
    InvalidType(String, String, String),
    #[error("Migration failed: {0}")]
    MigrationError(String),
}

pub struct SchemaRegistry;

impl SchemaRegistry {
    /// Validates JSON against a specific telemetry schema version
    pub fn validate(data: &Value, version: TelemetryVersion) -> Result<(), Vec<SchemaError>> {
        let mut errors = Vec::new();

        // Standard fields required by all versions
        let base_required = [
            ("satellite_id", "string"),
            ("timestamp", "string"),
            ("event_id", "string"),
        ];

        for (field, expected_type) in &base_required {
            if let Some(val) = data.get(field) {
                match *expected_type {
                    "string" if !val.is_string() => errors.push(SchemaError::InvalidType(
                        field.to_string(),
                        "string".to_string(),
                        format!("{:?}", val),
                    )),
                    _ => {}
                }
            } else {
                errors.push(SchemaError::MissingField(field.to_string()));
            }
        }

        // V1 required metric fields
        if version == TelemetryVersion::V1 {
            let v1_required = [
                "battery_voltage",
                "battery_current",
                "solar_panel_output",
                "cpu_usage",
                "memory_usage",
                "temperature",
                "orientation_pitch",
                "orientation_roll",
                "orientation_yaw",
                "velocity",
                "altitude",
                "fuel_remaining",
                "communication_latency",
                "packet_loss",
                "signal_strength",
            ];

            for field in &v1_required {
                if let Some(val) = data.get(field) {
                    if !val.is_number() {
                        errors.push(SchemaError::InvalidType(
                            field.to_string(),
                            "number".to_string(),
                            format!("{:?}", val),
                        ));
                    }
                } else {
                    errors.push(SchemaError::MissingField(field.to_string()));
                }
            }
        }

        // V2 & V3 type validation of optional/new fields
        if version >= TelemetryVersion::V2 {
            if let Some(val) = data.get("payload_power_draw") {
                if !val.is_number() && !val.is_null() {
                    errors.push(SchemaError::InvalidType(
                        "payload_power_draw".to_string(),
                        "number".to_string(),
                        format!("{:?}", val),
                    ));
                }
            }
            if let Some(val) = data.get("payload_active") {
                if !val.is_boolean() && !val.is_null() {
                    errors.push(SchemaError::InvalidType(
                        "payload_active".to_string(),
                        "boolean".to_string(),
                        format!("{:?}", val),
                    ));
                }
            }
        }

        if version >= TelemetryVersion::V3 {
            let v3_optionals = [
                "adcs_gyro_x",
                "adcs_gyro_y",
                "adcs_gyro_z",
                "thruster_pressure",
                "radiator_temp",
            ];
            for field in &v3_optionals {
                if let Some(val) = data.get(field) {
                    if !val.is_number() && !val.is_null() {
                        errors.push(SchemaError::InvalidType(
                            field.to_string(),
                            "number".to_string(),
                            format!("{:?}", val),
                        ));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Check if target schema version is backward compatible from source version
    pub fn is_compatible(from: TelemetryVersion, to: TelemetryVersion) -> bool {
        // V2 is backward compatible with V1 (it only adds optional fields or fields with defaults)
        // V3 is backward compatible with V2
        from <= to
    }

    /// Migrates telemetry data from one version to another
    pub fn migrate(
        mut data: Value,
        from: TelemetryVersion,
        to: TelemetryVersion,
    ) -> Result<Value, SchemaError> {
        if from == to {
            return Ok(data);
        }

        if !Self::is_compatible(from, to) {
            return Err(SchemaError::MigrationError(format!(
                "Incompatible migration path from {:?} to {:?}",
                from, to
            )));
        }

        let obj = data
            .as_object_mut()
            .ok_or_else(|| SchemaError::MigrationError("Data is not a JSON object".to_string()))?;

        // V1 -> V2 Migration
        if from == TelemetryVersion::V1 && to >= TelemetryVersion::V2 {
            if !obj.contains_key("subsystem_id") {
                obj.insert("subsystem_id".to_string(), Value::String("unified".to_string()));
            }
            if !obj.contains_key("mission_id") {
                obj.insert("mission_id".to_string(), Value::String("unknown".to_string()));
            }
            if !obj.contains_key("payload_power_draw") {
                obj.insert("payload_power_draw".to_string(), Value::Null);
            }
            if !obj.contains_key("payload_active") {
                obj.insert("payload_active".to_string(), Value::Null);
            }
        }

        // V2 -> V3 Migration
        if from <= TelemetryVersion::V2 && to == TelemetryVersion::V3 {
            let v3_fields = [
                "adcs_gyro_x",
                "adcs_gyro_y",
                "adcs_gyro_z",
                "thruster_pressure",
                "radiator_temp",
            ];
            for field in &v3_fields {
                if !obj.contains_key(*field) {
                    obj.insert(field.to_string(), Value::Null);
                }
            }
        }

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_v1_validation() {
        let v1_valid = json!({
            "satellite_id": "SAT-0001",
            "timestamp": "2026-06-15T20:00:00Z",
            "event_id": "550e8400-e29b-41d4-a716-446655440000",
            "battery_voltage": 12.5,
            "battery_current": 1.2,
            "solar_panel_output": 45.0,
            "cpu_usage": 15.0,
            "memory_usage": 33.3,
            "temperature": 25.0,
            "orientation_pitch": 0.0,
            "orientation_roll": 0.0,
            "orientation_yaw": 0.0,
            "velocity": 7.5,
            "altitude": 500.0,
            "fuel_remaining": 88.0,
            "communication_latency": 10.0,
            "packet_loss": 0.0,
            "signal_strength": -55.0
        });

        assert!(SchemaRegistry::validate(&v1_valid, TelemetryVersion::V1).is_ok());

        let v1_invalid = json!({
            "satellite_id": "SAT-0001",
            "timestamp": "2026-06-15T20:00:00Z"
        });
        assert!(SchemaRegistry::validate(&v1_invalid, TelemetryVersion::V1).is_err());
    }

    #[test]
    fn test_compatibility_and_migration() {
        assert!(SchemaRegistry::is_compatible(TelemetryVersion::V1, TelemetryVersion::V2));
        assert!(SchemaRegistry::is_compatible(TelemetryVersion::V1, TelemetryVersion::V3));
        assert!(!SchemaRegistry::is_compatible(TelemetryVersion::V3, TelemetryVersion::V1));

        let v1_data = json!({
            "satellite_id": "SAT-0001",
            "timestamp": "2026-06-15T20:00:00Z",
            "event_id": "550e8400-e29b-41d4-a716-446655440000",
            "battery_voltage": 12.5,
            "battery_current": 1.2,
            "solar_panel_output": 45.0,
            "cpu_usage": 15.0,
            "memory_usage": 33.3,
            "temperature": 25.0,
            "orientation_pitch": 0.0,
            "orientation_roll": 0.0,
            "orientation_yaw": 0.0,
            "velocity": 7.5,
            "altitude": 500.0,
            "fuel_remaining": 88.0,
            "communication_latency": 10.0,
            "packet_loss": 0.0,
            "signal_strength": -55.0
        });

        let migrated = SchemaRegistry::migrate(v1_data, TelemetryVersion::V1, TelemetryVersion::V3).unwrap();
        assert_eq!(migrated["subsystem_id"], "unified");
        assert_eq!(migrated["mission_id"], "unknown");
        assert!(migrated["adcs_gyro_x"].is_null());
    }
}
