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
use chrono::Utc;
use orbitlogs_models::{MissionId, SatelliteId, Subsystem, TelemetryRecord};
use uuid::Uuid;

/// Helper to create a valid TelemetryRecord for testing.
pub fn mock_telemetry_record(sat_id: &str) -> TelemetryRecord {
    TelemetryRecord {
        satellite_id: SatelliteId::new(sat_id),
        subsystem_id: Subsystem::Unified,
        mission_id: MissionId::new("test-mission"),
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
