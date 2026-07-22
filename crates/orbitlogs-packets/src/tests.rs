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
    use chrono::{TimeZone, Utc};
    use orbitlogs_models::TelemetryRecord;
    use uuid::Uuid;

    fn mock_record() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: orbitlogs_models::SatelliteId::new("SAT-0005"),
            subsystem_id: orbitlogs_models::Subsystem::Power,
            mission_id: orbitlogs_models::MissionId::new("MISSION-ALPHA"),
            timestamp: Utc::now(),
            battery_voltage: Some(13.45),
            battery_current: Some(-2.15),
            solar_panel_output: Some(95.4),
            cpu_usage: None,
            memory_usage: None,
            temperature: None,
            orientation_pitch: None,
            orientation_roll: None,
            orientation_yaw: None,
            velocity: None,
            altitude: None,
            fuel_remaining: None,
            communication_latency: None,
            packet_loss: None,
            signal_strength: None,
            payload_power_draw: None,
            payload_active: None,
            event_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let rec = mock_record();
        let encoded_bytes = encode_record_to_frame(&rec, 42).unwrap();
        let decoded_rec = decode_frame_to_record(&encoded_bytes).unwrap();

        assert_eq!(decoded_rec.satellite_id, rec.satellite_id);
        assert_eq!(decoded_rec.subsystem_id, rec.subsystem_id);
        assert_eq!(
            decoded_rec.battery_voltage.unwrap(),
            rec.battery_voltage.unwrap()
        );
        assert_eq!(
            decoded_rec.battery_current.unwrap(),
            rec.battery_current.unwrap()
        );
        assert_eq!(
            decoded_rec.solar_panel_output.unwrap(),
            rec.solar_panel_output.unwrap()
        );
        assert_eq!(decoded_rec.timestamp.timestamp(), rec.timestamp.timestamp());
    }

    #[test]
    fn test_corrupt_data_detection() {
        let rec = mock_record();
        let mut encoded_bytes = encode_record_to_frame(&rec, 101).unwrap();

        let len = encoded_bytes.len();
        encoded_bytes[len - 5] ^= 0x55;

        let res = decode_frame_to_record(&encoded_bytes);
        assert!(res.is_err());
        match res.unwrap_err() {
            PacketError::InvalidCrc { .. } => {}
            other => panic!("Expected CRC mismatch, received {:?}", other),
        }
    }

    proptest::proptest! {
        #[test]
        fn test_encode_decode_proptest(
            scid in 0u16..1023u16,
            subsystem_idx in 0..8usize,
            seq_count in 0u16..16383u16,
            ts_coarse in 0u32..u32::MAX,
            ts_fine in 0u32..999_999_999u32,
            battery_voltage in proptest::option::of(0.0..100.0f64),
            battery_current in proptest::option::of(-50.0..50.0f64),
            solar_panel_output in proptest::option::of(0.0..500.0f64),
            cpu_usage in proptest::option::of(0.0..100.0f32),
            memory_usage in proptest::option::of(0.0..100.0f32),
            temperature in proptest::option::of(-100.0..200.0f64),
            orientation_pitch in proptest::option::of(-360.0..360.0f32),
            orientation_roll in proptest::option::of(-360.0..360.0f32),
            orientation_yaw in proptest::option::of(-360.0..360.0f32),
            velocity in proptest::option::of(0.0..20.0f64),
            altitude in proptest::option::of(100.0..2000.0f64),
            fuel_remaining in proptest::option::of(0.0..100.0f32),
            communication_latency in proptest::option::of(0.0..5000.0f32),
            packet_loss in proptest::option::of(0.0..100.0f32),
            signal_strength in proptest::option::of(-150.0..0.0f32),
            payload_power_draw in proptest::option::of(0.0..100.0f64),
            payload_active in proptest::option::of(proptest::bool::ANY),
        ) {
            let sat_id = orbitlogs_models::SatelliteId::new(format!("SAT-{:04}", scid));
            let subsystems = [
                orbitlogs_models::Subsystem::Power,
                orbitlogs_models::Subsystem::Thermal,
                orbitlogs_models::Subsystem::Communications,
                orbitlogs_models::Subsystem::Propulsion,
                orbitlogs_models::Subsystem::Adcs,
                orbitlogs_models::Subsystem::Payload,
                orbitlogs_models::Subsystem::FlightComputer,
                orbitlogs_models::Subsystem::Unified,
            ];
            let subsystem_id = subsystems[subsystem_idx];

            let ts = Utc.timestamp_opt(ts_coarse as i64, ts_fine).single().unwrap_or_else(Utc::now);

            let rec = TelemetryRecord {
                satellite_id: sat_id,
                subsystem_id,
                mission_id: orbitlogs_models::MissionId::new("unknown"),
                timestamp: ts,
                battery_voltage,
                battery_current,
                solar_panel_output,
                cpu_usage,
                memory_usage,
                temperature,
                orientation_pitch,
                orientation_roll,
                orientation_yaw,
                velocity,
                altitude,
                fuel_remaining,
                communication_latency,
                packet_loss,
                signal_strength,
                payload_power_draw,
                payload_active,
                event_id: Uuid::new_v4(),
            };

            let encoded = encode_record_to_frame(&rec, seq_count).unwrap();
            let decoded = decode_frame_to_record(&encoded).unwrap();

            assert_eq!(decoded.satellite_id, rec.satellite_id);
            assert_eq!(decoded.subsystem_id, rec.subsystem_id);
            assert_eq!(decoded.timestamp.timestamp(), rec.timestamp.timestamp());
            assert_eq!(decoded.timestamp.timestamp_subsec_nanos(), rec.timestamp.timestamp_subsec_nanos());

            match rec.subsystem_id {
                orbitlogs_models::Subsystem::Power => {
                    assert_eq!(decoded.battery_voltage.unwrap_or(0.0), rec.battery_voltage.unwrap_or(0.0));
                    assert_eq!(decoded.battery_current.unwrap_or(0.0), rec.battery_current.unwrap_or(0.0));
                    assert_eq!(decoded.solar_panel_output.unwrap_or(0.0), rec.solar_panel_output.unwrap_or(0.0));
                }
                orbitlogs_models::Subsystem::Thermal => {
                    assert_eq!(decoded.temperature.unwrap_or(0.0), rec.temperature.unwrap_or(0.0));
                }
                orbitlogs_models::Subsystem::Communications => {
                    assert_eq!(decoded.communication_latency.unwrap_or(0.0), rec.communication_latency.unwrap_or(0.0));
                    assert_eq!(decoded.packet_loss.unwrap_or(0.0), rec.packet_loss.unwrap_or(0.0));
                    assert_eq!(decoded.signal_strength.unwrap_or(0.0), rec.signal_strength.unwrap_or(0.0));
                }
                orbitlogs_models::Subsystem::Propulsion => {
                    assert_eq!(decoded.velocity.unwrap_or(0.0), rec.velocity.unwrap_or(0.0));
                    assert_eq!(decoded.altitude.unwrap_or(0.0), rec.altitude.unwrap_or(0.0));
                    assert_eq!(decoded.fuel_remaining.unwrap_or(0.0), rec.fuel_remaining.unwrap_or(0.0));
                }
                orbitlogs_models::Subsystem::Adcs => {
                    assert_eq!(decoded.orientation_pitch.unwrap_or(0.0), rec.orientation_pitch.unwrap_or(0.0));
                    assert_eq!(decoded.orientation_roll.unwrap_or(0.0), rec.orientation_roll.unwrap_or(0.0));
                    assert_eq!(decoded.orientation_yaw.unwrap_or(0.0), rec.orientation_yaw.unwrap_or(0.0));
                }
                orbitlogs_models::Subsystem::Payload => {
                    assert_eq!(decoded.payload_power_draw.unwrap_or(0.0), rec.payload_power_draw.unwrap_or(0.0));
                    assert_eq!(decoded.payload_active.unwrap_or(false), rec.payload_active.unwrap_or(false));
                }
                orbitlogs_models::Subsystem::FlightComputer => {
                    assert_eq!(decoded.cpu_usage.unwrap_or(0.0), rec.cpu_usage.unwrap_or(0.0));
                    assert_eq!(decoded.memory_usage.unwrap_or(0.0), rec.memory_usage.unwrap_or(0.0));
                }
                orbitlogs_models::Subsystem::Unified => {
                    assert_eq!(decoded.battery_voltage.unwrap_or(0.0), rec.battery_voltage.unwrap_or(0.0));
                    assert_eq!(decoded.battery_current.unwrap_or(0.0), rec.battery_current.unwrap_or(0.0));
                    assert_eq!(decoded.solar_panel_output.unwrap_or(0.0), rec.solar_panel_output.unwrap_or(0.0));
                    assert_eq!(decoded.temperature.unwrap_or(0.0), rec.temperature.unwrap_or(0.0));
                    assert_eq!(decoded.communication_latency.unwrap_or(0.0), rec.communication_latency.unwrap_or(0.0));
                    assert_eq!(decoded.packet_loss.unwrap_or(0.0), rec.packet_loss.unwrap_or(0.0));
                    assert_eq!(decoded.signal_strength.unwrap_or(0.0), rec.signal_strength.unwrap_or(0.0));
                    assert_eq!(decoded.velocity.unwrap_or(0.0), rec.velocity.unwrap_or(0.0));
                    assert_eq!(decoded.altitude.unwrap_or(0.0), rec.altitude.unwrap_or(0.0));
                    assert_eq!(decoded.fuel_remaining.unwrap_or(0.0), rec.fuel_remaining.unwrap_or(0.0));
                    assert_eq!(decoded.orientation_pitch.unwrap_or(0.0), rec.orientation_pitch.unwrap_or(0.0));
                    assert_eq!(decoded.orientation_roll.unwrap_or(0.0), rec.orientation_roll.unwrap_or(0.0));
                    assert_eq!(decoded.orientation_yaw.unwrap_or(0.0), rec.orientation_yaw.unwrap_or(0.0));
                    assert_eq!(decoded.payload_power_draw.unwrap_or(0.0), rec.payload_power_draw.unwrap_or(0.0));
                    assert_eq!(decoded.payload_active.unwrap_or(false), rec.payload_active.unwrap_or(false));
                    assert_eq!(decoded.cpu_usage.unwrap_or(0.0), rec.cpu_usage.unwrap_or(0.0));
                    assert_eq!(decoded.memory_usage.unwrap_or(0.0), rec.memory_usage.unwrap_or(0.0));
                }
            }
        }
    }
}
