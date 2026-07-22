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
    use chrono::Utc;
    use uuid::Uuid;

    fn mock_valid_record() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: SatelliteId::new("SAT-0001"),
            subsystem_id: Subsystem::Unified,
            mission_id: MissionId::new("unknown"),
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
        rec.satellite_id = SatelliteId::new("");
        assert!(rec.validate().is_err());

        rec.satellite_id = SatelliteId::new("SAT#01");
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
        assert_eq!(deserialized.subsystem_id, Subsystem::Unified);
        assert_eq!(deserialized.mission_id, MissionId::new("unknown"));
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

    #[test]
    fn satellite_id_parse_valid() {
        let id = SatelliteId::parse("SAT-0001").unwrap();
        assert_eq!(id.as_str(), "SAT-0001");
    }

    #[test]
    fn satellite_id_parse_rejects_bad_prefix() {
        assert!(SatelliteId::parse("").is_err());
        assert!(SatelliteId::parse("SAT0001").is_err());
        assert!(SatelliteId::parse("SATELLITE-0001").is_err());
        assert!(SatelliteId::parse("DROP TABLE satellites").is_err());
    }

    #[test]
    fn station_id_parse_valid() {
        let id = StationId::parse("GS-0001").unwrap();
        assert_eq!(id.as_str(), "GS-0001");
    }

    #[test]
    fn station_id_parse_rejects_bad_prefix() {
        assert!(StationId::parse("").is_err());
        assert!(StationId::parse("GROUND-01").is_err());
    }

    #[test]
    fn mission_id_parse_valid() {
        let id = MissionId::parse("MISSION-ALPHA").unwrap();
        assert_eq!(id.as_str(), "MISSION-ALPHA");
    }

    #[test]
    fn mission_id_parse_rejects_bad_prefix() {
        assert!(MissionId::parse("").is_err());
        assert!(MissionId::parse("TASK-1").is_err());
    }

    #[test]
    fn newtype_backward_compat_new() {
        let id = SatelliteId::new("any-string");
        assert_eq!(id.as_str(), "any-string");
    }

    #[test]
    fn domain_error_conversion_chain() {
        let result: Result<SatelliteId, DomainError> =
            SatelliteId::parse("INVALID").map_err(|e| e.into());
        assert!(result.is_err());
    }
}

pub mod test_support {
    use crate::*;
    use chrono::Utc;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    pub struct MockTelemetryStore {
        pub inserted: Mutex<Vec<TelemetryRecord>>,
    }

    impl MockTelemetryStore {
        pub fn new() -> Self {
            Self {
                inserted: Mutex::new(Vec::new()),
            }
        }

        pub fn record_insert(&self, record: &TelemetryRecord) {
            self.inserted.lock().unwrap().push(record.clone());
        }

        pub fn inserted_count(&self) -> usize {
            self.inserted.lock().unwrap().len()
        }

        pub fn drain(&self) -> Vec<TelemetryRecord> {
            std::mem::take(&mut *self.inserted.lock().unwrap())
        }
    }

    pub fn mock_telemetry_record(sat_id: &str) -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: SatelliteId::new(sat_id),
            subsystem_id: Subsystem::Unified,
            mission_id: MissionId::new("unknown"),
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
    fn mock_store_captures_inserts() {
        let store = MockTelemetryStore::new();
        let record = mock_telemetry_record("SAT-0001");
        store.record_insert(&record);
        store.record_insert(&record);
        assert_eq!(store.inserted_count(), 2);

        let drained = store.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(store.inserted_count(), 0);
    }
}
