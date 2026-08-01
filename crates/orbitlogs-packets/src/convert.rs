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

use chrono::{TimeZone, Utc};
use orbitlogs_models::TelemetryRecord;
use uuid::Uuid;

use crate::error::PacketError;
use crate::frame::TelemetryFrame;
use crate::packet::*;

pub fn scid_from_satellite_id(sat_id: &str) -> u16 {
    if let Some(num_str) = sat_id.strip_prefix("SAT-") {
        if let Ok(num) = num_str.parse::<u16>() {
            return num;
        }
    }
    0
}

pub fn satellite_id_from_scid(scid: u16) -> String {
    format!("SAT-{:04}", scid)
}

fn parse_f64(slice: &[u8]) -> Result<f64, PacketError> {
    slice
        .try_into()
        .map(f64::from_be_bytes)
        .map_err(|_| PacketError::ParserError("Failed to parse f64".to_string()))
}

fn parse_f32(slice: &[u8]) -> Result<f32, PacketError> {
    slice
        .try_into()
        .map(f32::from_be_bytes)
        .map_err(|_| PacketError::ParserError("Failed to parse f32".to_string()))
}

/// Serializes a TelemetryRecord to a binary Space Packet inside a Telemetry Frame.
pub fn encode_record_to_frame(
    record: &TelemetryRecord,
    seq_count: u16,
) -> Result<Vec<u8>, PacketError> {
    let scid = scid_from_satellite_id(record.satellite_id.as_str());

    let apid = match record.subsystem_id {
        orbitlogs_models::Subsystem::Power => APID_POWER,
        orbitlogs_models::Subsystem::Thermal => APID_THERMAL,
        orbitlogs_models::Subsystem::Communications => APID_COMMS,
        orbitlogs_models::Subsystem::Propulsion => APID_PROPULSION,
        orbitlogs_models::Subsystem::Adcs => APID_ADCS,
        orbitlogs_models::Subsystem::Payload => APID_PAYLOAD,
        orbitlogs_models::Subsystem::FlightComputer => APID_FLIGHT_COMPUTER,
        orbitlogs_models::Subsystem::Unified => APID_UNIFIED,
    };

    let mut user_data = Vec::new();
    user_data.push(apid as u8);

    match apid {
        APID_POWER => {
            let v = record.battery_voltage.unwrap_or(0.0);
            let c = record.battery_current.unwrap_or(0.0);
            let s = record.solar_panel_output.unwrap_or(0.0);
            user_data.extend_from_slice(&v.to_be_bytes());
            user_data.extend_from_slice(&c.to_be_bytes());
            user_data.extend_from_slice(&s.to_be_bytes());
        }
        APID_THERMAL => {
            let t = record.temperature.unwrap_or(0.0);
            user_data.extend_from_slice(&t.to_be_bytes());
        }
        APID_COMMS => {
            let latency = record.communication_latency.unwrap_or(0.0);
            let loss = record.packet_loss.unwrap_or(0.0);
            let signal = record.signal_strength.unwrap_or(0.0);
            user_data.extend_from_slice(&latency.to_be_bytes());
            user_data.extend_from_slice(&loss.to_be_bytes());
            user_data.extend_from_slice(&signal.to_be_bytes());
        }
        APID_PROPULSION => {
            let vel = record.velocity.unwrap_or(0.0);
            let alt = record.altitude.unwrap_or(0.0);
            let fuel = record.fuel_remaining.unwrap_or(0.0);
            user_data.extend_from_slice(&vel.to_be_bytes());
            user_data.extend_from_slice(&alt.to_be_bytes());
            user_data.extend_from_slice(&fuel.to_be_bytes());
        }
        APID_ADCS => {
            let pitch = record.orientation_pitch.unwrap_or(0.0);
            let roll = record.orientation_roll.unwrap_or(0.0);
            let yaw = record.orientation_yaw.unwrap_or(0.0);
            user_data.extend_from_slice(&pitch.to_be_bytes());
            user_data.extend_from_slice(&roll.to_be_bytes());
            user_data.extend_from_slice(&yaw.to_be_bytes());
        }
        APID_PAYLOAD => {
            let draw = record.payload_power_draw.unwrap_or(0.0);
            let active = record.payload_active.unwrap_or(false) as u8;
            user_data.extend_from_slice(&draw.to_be_bytes());
            user_data.push(active);
        }
        APID_FLIGHT_COMPUTER => {
            let cpu = record.cpu_usage.unwrap_or(0.0);
            let mem = record.memory_usage.unwrap_or(0.0);
            user_data.extend_from_slice(&cpu.to_be_bytes());
            user_data.extend_from_slice(&mem.to_be_bytes());
        }
        _ => {
            let v = record.battery_voltage.unwrap_or(0.0);
            let c = record.battery_current.unwrap_or(0.0);
            let s = record.solar_panel_output.unwrap_or(0.0);
            let t = record.temperature.unwrap_or(0.0);
            let latency = record.communication_latency.unwrap_or(0.0);
            let loss = record.packet_loss.unwrap_or(0.0);
            let signal = record.signal_strength.unwrap_or(0.0);
            let vel = record.velocity.unwrap_or(0.0);
            let alt = record.altitude.unwrap_or(0.0);
            let fuel = record.fuel_remaining.unwrap_or(0.0);
            let pitch = record.orientation_pitch.unwrap_or(0.0);
            let roll = record.orientation_roll.unwrap_or(0.0);
            let yaw = record.orientation_yaw.unwrap_or(0.0);
            let draw = record.payload_power_draw.unwrap_or(0.0);
            let active = record.payload_active.unwrap_or(false) as u8;
            let cpu = record.cpu_usage.unwrap_or(0.0);
            let mem = record.memory_usage.unwrap_or(0.0);

            user_data.extend_from_slice(&v.to_be_bytes());
            user_data.extend_from_slice(&c.to_be_bytes());
            user_data.extend_from_slice(&s.to_be_bytes());
            user_data.extend_from_slice(&t.to_be_bytes());
            user_data.extend_from_slice(&latency.to_be_bytes());
            user_data.extend_from_slice(&loss.to_be_bytes());
            user_data.extend_from_slice(&signal.to_be_bytes());
            user_data.extend_from_slice(&vel.to_be_bytes());
            user_data.extend_from_slice(&alt.to_be_bytes());
            user_data.extend_from_slice(&fuel.to_be_bytes());
            user_data.extend_from_slice(&pitch.to_be_bytes());
            user_data.extend_from_slice(&roll.to_be_bytes());
            user_data.extend_from_slice(&yaw.to_be_bytes());
            user_data.extend_from_slice(&draw.to_be_bytes());
            user_data.push(active);
            user_data.extend_from_slice(&cpu.to_be_bytes());
            user_data.extend_from_slice(&mem.to_be_bytes());
        }
    }

    let ts = record.timestamp;
    let coarse = ts.timestamp() as u32;
    let fine = ts.timestamp_subsec_nanos();

    let packet_length = (8 + user_data.len() + 2 - 1) as u16;

    let packet = TelemetryPacket {
        packet_version: 0,
        packet_type: 0,
        sec_header_flag: true,
        apid,
        sequence_flags: 3,
        sequence_count: seq_count,
        packet_length,
        timestamp_coarse: coarse,
        timestamp_fine: fine,
        user_data,
        packet_error_control: 0,
    };

    let serialized_packet = packet.to_bytes();

    let frame = TelemetryFrame {
        frame_version: 0,
        spacecraft_id: scid,
        virtual_channel_id: 0,
        ocf_flag: false,
        master_frame_count: (seq_count & 0xFF) as u8,
        vc_frame_count: (seq_count & 0xFF) as u8,
        first_header_pointer: 0,
        data_field: serialized_packet,
        frame_error_control: 0,
    };

    Ok(frame.to_bytes())
}

/// Decodes binary frame data back to a TelemetryRecord.
pub fn decode_frame_to_record(bytes: &[u8]) -> Result<TelemetryRecord, PacketError> {
    let frame = TelemetryFrame::from_bytes(bytes)?;
    let packet = TelemetryPacket::from_bytes(&frame.data_field)?;

    let satellite_id =
        orbitlogs_models::SatelliteId::new(satellite_id_from_scid(frame.spacecraft_id));
    let subsystem_id = match packet.apid {
        APID_POWER => orbitlogs_models::Subsystem::Power,
        APID_THERMAL => orbitlogs_models::Subsystem::Thermal,
        APID_COMMS => orbitlogs_models::Subsystem::Communications,
        APID_PROPULSION => orbitlogs_models::Subsystem::Propulsion,
        APID_ADCS => orbitlogs_models::Subsystem::Adcs,
        APID_PAYLOAD => orbitlogs_models::Subsystem::Payload,
        APID_FLIGHT_COMPUTER => orbitlogs_models::Subsystem::FlightComputer,
        _ => orbitlogs_models::Subsystem::Unified,
    };

    let ts = Utc
        .timestamp_opt(packet.timestamp_coarse as i64, packet.timestamp_fine)
        .single()
        .unwrap_or_else(Utc::now);

    let mut record = TelemetryRecord {
        satellite_id,
        subsystem_id,
        mission_id: orbitlogs_models::MissionId::new("unknown"),
        timestamp: ts,
        battery_voltage: None,
        battery_current: None,
        solar_panel_output: None,
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
    };

    let ud = &packet.user_data;
    if ud.is_empty() {
        return Err(PacketError::ParserError(
            "User data field is empty".to_string(),
        ));
    }
    let data_apid = ud[0] as u16;
    if data_apid != packet.apid {
        return Err(PacketError::SubsystemMismatch(format!(
            "The payload header apid {} does not match the packet apid {}",
            data_apid, packet.apid,
        )));
    }

    let payload = &ud[1..];

    match packet.apid {
        APID_POWER => {
            if payload.len() < 24 {
                return Err(PacketError::ParserError(
                    "Power payload is too short".to_string(),
                ));
            }
            record.battery_voltage = Some(parse_f64(&payload[0..8])?);
            record.battery_current = Some(parse_f64(&payload[8..16])?);
            record.solar_panel_output = Some(parse_f64(&payload[16..24])?);
        }
        APID_THERMAL => {
            if payload.len() < 8 {
                return Err(PacketError::ParserError(
                    "Thermal payload is too short".to_string(),
                ));
            }
            record.temperature = Some(parse_f64(&payload[0..8])?);
        }
        APID_COMMS => {
            if payload.len() < 12 {
                return Err(PacketError::ParserError(
                    "Comms payload is too short".to_string(),
                ));
            }
            record.communication_latency = Some(parse_f32(&payload[0..4])?);
            record.packet_loss = Some(parse_f32(&payload[4..8])?);
            record.signal_strength = Some(parse_f32(&payload[8..12])?);
        }
        APID_PROPULSION => {
            if payload.len() < 20 {
                return Err(PacketError::ParserError(
                    "Propulsion payload is too short".to_string(),
                ));
            }
            record.velocity = Some(parse_f64(&payload[0..8])?);
            record.altitude = Some(parse_f64(&payload[8..16])?);
            record.fuel_remaining = Some(parse_f32(&payload[16..20])?);
        }
        APID_ADCS => {
            if payload.len() < 12 {
                return Err(PacketError::ParserError(
                    "ADCS payload is too short".to_string(),
                ));
            }
            record.orientation_pitch = Some(parse_f32(&payload[0..4])?);
            record.orientation_roll = Some(parse_f32(&payload[4..8])?);
            record.orientation_yaw = Some(parse_f32(&payload[8..12])?);
        }
        APID_PAYLOAD => {
            if payload.len() < 9 {
                return Err(PacketError::ParserError("Payload is too short".to_string()));
            }
            record.payload_power_draw = Some(parse_f64(&payload[0..8])?);
            record.payload_active = Some(payload[8] != 0);
        }
        APID_FLIGHT_COMPUTER => {
            if payload.len() < 8 {
                return Err(PacketError::ParserError(
                    "Flight computer payload is too short".to_string(),
                ));
            }
            record.cpu_usage = Some(parse_f32(&payload[0..4])?);
            record.memory_usage = Some(parse_f32(&payload[4..8])?);
        }
        _ => {
            if payload.len() < 93 {
                return Err(PacketError::ParserError(
                    "Unified payload is too short".to_string(),
                ));
            }
            record.battery_voltage = Some(parse_f64(&payload[0..8])?);
            record.battery_current = Some(parse_f64(&payload[8..16])?);
            record.solar_panel_output = Some(parse_f64(&payload[16..24])?);
            record.temperature = Some(parse_f64(&payload[24..32])?);
            record.communication_latency = Some(parse_f32(&payload[32..36])?);
            record.packet_loss = Some(parse_f32(&payload[36..40])?);
            record.signal_strength = Some(parse_f32(&payload[40..44])?);
            record.velocity = Some(parse_f64(&payload[44..52])?);
            record.altitude = Some(parse_f64(&payload[52..60])?);
            record.fuel_remaining = Some(parse_f32(&payload[60..64])?);
            record.orientation_pitch = Some(parse_f32(&payload[64..68])?);
            record.orientation_roll = Some(parse_f32(&payload[68..72])?);
            record.orientation_yaw = Some(parse_f32(&payload[72..76])?);
            record.payload_power_draw = Some(parse_f64(&payload[76..84])?);
            record.payload_active = Some(payload[84] != 0);
            record.cpu_usage = Some(parse_f32(&payload[85..89])?);
            record.memory_usage = Some(parse_f32(&payload[89..93])?);
        }
    }

    Ok(record)
}
