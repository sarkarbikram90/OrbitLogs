pub mod crc;

use chrono::{TimeZone, Utc};
use spaceops_models::TelemetryRecord;
use uuid::Uuid;

#[derive(Debug)]
pub enum PacketError {
    InvalidLength(String),
    InvalidCrc { expected: u16, calculated: u16 },
    UnknownApid(u16),
    SubsystemMismatch(String),
    ParserError(String),
}

impl std::fmt::Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketError::InvalidLength(s) => write!(f, "Data length too short: {}", s),
            PacketError::InvalidCrc { expected, calculated } => write!(f, "Invalid CRC-16 checksum: expected {:#06X}, got {:#06X}", expected, calculated),
            PacketError::UnknownApid(a) => write!(f, "Unknown APID: {}", a),
            PacketError::SubsystemMismatch(s) => write!(f, "Subsystem mismatch: {}", s),
            PacketError::ParserError(s) => write!(f, "Failed to parse field: {}", s),
        }
    }
}

impl std::error::Error for PacketError {}

/// APID constants for spacecraft subsystems
pub const APID_UNIFIED: u16 = 0;
pub const APID_POWER: u16 = 1;
pub const APID_THERMAL: u16 = 2;
pub const APID_COMMS: u16 = 3;
pub const APID_PROPULSION: u16 = 4;
pub const APID_ADCS: u16 = 5;
pub const APID_PAYLOAD: u16 = 6;
pub const APID_FLIGHT_COMPUTER: u16 = 7;

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryPacket {
    // Primary Header (6 bytes)
    pub packet_version: u8,        // 3 bits
    pub packet_type: u8,           // 1 bit
    pub sec_header_flag: bool,     // 1 bit
    pub apid: u16,                 // 11 bits
    pub sequence_flags: u8,        // 2 bits
    pub sequence_count: u16,       // 14 bits
    pub packet_length: u16,        // 16 bits (Length of user data + sec header + CRC - 1)

    // Secondary Header (8 bytes)
    pub timestamp_coarse: u32,     // 32 bits (Seconds since UNIX epoch)
    pub timestamp_fine: u32,       // 32 bits (Nanoseconds)

    // User Data Field (variable length)
    pub user_data: Vec<u8>,

    // Error Control (2 bytes)
    pub packet_error_control: u16, // CRC-16
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryFrame {
    // Frame Header (6 bytes)
    pub frame_version: u8,         // 2 bits
    pub spacecraft_id: u16,        // 10 bits
    pub virtual_channel_id: u8,    // 3 bits
    pub ocf_flag: bool,            // 1 bit
    pub master_frame_count: u8,    // 8 bits
    pub vc_frame_count: u8,        // 8 bits
    pub first_header_pointer: u16, // 11 bits (pointer) + 5 bits spare = 16 bits

    // Frame Data Field
    pub data_field: Vec<u8>,       // Serialized TelemetryPacket

    // Error Control (2 bytes)
    pub frame_error_control: u16,  // CRC-16
}

impl TelemetryPacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(6 + 8 + self.user_data.len() + 2);

        // 1. Pack Primary Header (6 bytes)
        let word1 = ((self.packet_version & 0x07) as u16) << 13
            | ((self.packet_type & 0x01) as u16) << 12
            | (self.sec_header_flag as u16) << 11
            | (self.apid & 0x07FF);
        
        let word2 = ((self.sequence_flags & 0x03) as u16) << 14
            | (self.sequence_count & 0x3FFF);

        bytes.extend_from_slice(&word1.to_be_bytes());
        bytes.extend_from_slice(&word2.to_be_bytes());
        bytes.extend_from_slice(&self.packet_length.to_be_bytes());

        // 2. Pack Secondary Header (8 bytes)
        if self.sec_header_flag {
            bytes.extend_from_slice(&self.timestamp_coarse.to_be_bytes());
            bytes.extend_from_slice(&self.timestamp_fine.to_be_bytes());
        }

        // 3. User Data
        bytes.extend_from_slice(&self.user_data);

        // 4. Calculate and Pack CRC-16 (2 bytes)
        let calculated_crc = crc::calculate_crc16(&bytes);
        bytes.extend_from_slice(&calculated_crc.to_be_bytes());

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PacketError> {
        if bytes.len() < 8 {
            return Err(PacketError::InvalidLength("Packet too short for minimal header".to_string()));
        }

        // 1. Unpack Primary Header
        let word1 = u16::from_be_bytes([bytes[0], bytes[1]]);
        let word2 = u16::from_be_bytes([bytes[2], bytes[3]]);
        let packet_length = u16::from_be_bytes([bytes[4], bytes[5]]);

        let packet_version = ((word1 >> 13) & 0x07) as u8;
        let packet_type = ((word1 >> 12) & 0x01) as u8;
        let sec_header_flag = ((word1 >> 11) & 0x01) != 0;
        let apid = word1 & 0x07FF;

        let sequence_flags = ((word2 >> 14) & 0x03) as u8;
        let sequence_count = word2 & 0x3FFF;

        let total_expected_len = 6 + (packet_length as usize) + 1;
        if bytes.len() < total_expected_len {
            return Err(PacketError::InvalidLength(format!(
                "Packet length mismatch: expected {}, got {}",
                total_expected_len,
                bytes.len()
            )));
        }

        // Slice to actual packet boundary
        let packet_bytes = &bytes[0..total_expected_len];

        // Validate CRC
        let expected_crc = u16::from_be_bytes([
            packet_bytes[total_expected_len - 2],
            packet_bytes[total_expected_len - 1],
        ]);
        let calculated_crc = crc::calculate_crc16(&packet_bytes[0..total_expected_len - 2]);
        if expected_crc != calculated_crc {
            return Err(PacketError::InvalidCrc {
                expected: expected_crc,
                calculated: calculated_crc,
            });
        }

        // 2. Unpack Secondary Header
        let mut offset = 6;
        let mut timestamp_coarse = 0;
        let mut timestamp_fine = 0;

        if sec_header_flag {
            if total_expected_len < 16 {
                return Err(PacketError::InvalidLength("Packet missing secondary header bytes".to_string()));
            }
            timestamp_coarse = u32::from_be_bytes([
                packet_bytes[6],
                packet_bytes[7],
                packet_bytes[8],
                packet_bytes[9],
            ]);
            timestamp_fine = u32::from_be_bytes([
                packet_bytes[10],
                packet_bytes[11],
                packet_bytes[12],
                packet_bytes[13],
            ]);
            offset = 14;
        }

        // 3. Extract User Data
        let user_data = packet_bytes[offset..total_expected_len - 2].to_vec();

        Ok(Self {
            packet_version,
            packet_type,
            sec_header_flag,
            apid,
            sequence_flags,
            sequence_count,
            packet_length,
            timestamp_coarse,
            timestamp_fine,
            user_data,
            packet_error_control: expected_crc,
        })
    }
}

impl TelemetryFrame {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(6 + self.data_field.len() + 2);

        // 1. Pack Frame Header (6 bytes)
        let word1 = ((self.frame_version & 0x03) as u16) << 14
            | ((self.spacecraft_id & 0x03FF) as u16) << 4
            | ((self.virtual_channel_id & 0x07) as u16) << 1
            | (self.ocf_flag as u16);

        bytes.extend_from_slice(&word1.to_be_bytes());
        bytes.push(self.master_frame_count);
        bytes.push(self.vc_frame_count);
        bytes.extend_from_slice(&self.first_header_pointer.to_be_bytes());

        // 2. Pack Data Field
        bytes.extend_from_slice(&self.data_field);

        // 3. Calculate and Pack CRC-16 (2 bytes)
        let calculated_crc = crc::calculate_crc16(&bytes);
        bytes.extend_from_slice(&calculated_crc.to_be_bytes());

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PacketError> {
        if bytes.len() < 8 {
            return Err(PacketError::InvalidLength("Frame too short for header + CRC".to_string()));
        }

        // Validate CRC
        let expected_crc = u16::from_be_bytes([bytes[bytes.len() - 2], bytes[bytes.len() - 1]]);
        let calculated_crc = crc::calculate_crc16(&bytes[0..bytes.len() - 2]);
        if expected_crc != calculated_crc {
            return Err(PacketError::InvalidCrc {
                expected: expected_crc,
                calculated: calculated_crc,
            });
        }

        // Unpack Header
        let word1 = u16::from_be_bytes([bytes[0], bytes[1]]);
        let frame_version = ((word1 >> 14) & 0x03) as u8;
        let spacecraft_id = (word1 >> 4) & 0x03FF;
        let virtual_channel_id = ((word1 >> 1) & 0x07) as u8;
        let ocf_flag = (word1 & 0x01) != 0;

        let master_frame_count = bytes[2];
        let vc_frame_count = bytes[3];
        let first_header_pointer = u16::from_be_bytes([bytes[4], bytes[5]]);

        let data_field = bytes[6..bytes.len() - 2].to_vec();

        Ok(Self {
            frame_version,
            spacecraft_id,
            virtual_channel_id,
            ocf_flag,
            master_frame_count,
            vc_frame_count,
            first_header_pointer,
            data_field,
            frame_error_control: expected_crc,
        })
    }
}

/// Serializes a TelemetryRecord to a binary Space Packet inside a Telemetry Frame.
pub fn encode_record_to_frame(record: &TelemetryRecord, seq_count: u16) -> Result<Vec<u8>, PacketError> {
    // 1. Map satellite_id to SCID (e.g. "SAT-0001" -> 1)
    let scid = scid_from_satellite_id(&record.satellite_id);

    // 2. Map subsystem to APID
    let apid = match record.subsystem_id.to_lowercase().as_str() {
        "power" => APID_POWER,
        "thermal" => APID_THERMAL,
        "communications" | "comms" => APID_COMMS,
        "propulsion" => APID_PROPULSION,
        "adcs" => APID_ADCS,
        "payload" => APID_PAYLOAD,
        "flight_computer" | "flightcomputer" | "flight computer" => APID_FLIGHT_COMPUTER,
        _ => APID_UNIFIED,
    };

    // 3. Serialize user data field based on APID
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
            // Unified packing
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

    // 4. Create Packet
    let ts = record.timestamp;
    let coarse = ts.timestamp() as u32;
    let fine = ts.timestamp_subsec_nanos();
    
    // Packet length = secondary header size (8) + user data size + CRC size (2) - 1
    let packet_length = (8 + user_data.len() + 2 - 1) as u16;

    let packet = TelemetryPacket {
        packet_version: 0,
        packet_type: 0,
        sec_header_flag: true,
        apid,
        sequence_flags: 3, // 11 binary
        sequence_count: seq_count,
        packet_length,
        timestamp_coarse: coarse,
        timestamp_fine: fine,
        user_data,
        packet_error_control: 0, // Calculated inside to_bytes
    };

    let serialized_packet = packet.to_bytes();

    // 5. Create Transfer Frame
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

    let satellite_id = satellite_id_from_scid(frame.spacecraft_id);
    let subsystem_id = match packet.apid {
        APID_POWER => "power".to_string(),
        APID_THERMAL => "thermal".to_string(),
        APID_COMMS => "communications".to_string(),
        APID_PROPULSION => "propulsion".to_string(),
        APID_ADCS => "adcs".to_string(),
        APID_PAYLOAD => "payload".to_string(),
        APID_FLIGHT_COMPUTER => "flight_computer".to_string(),
        _ => "unified".to_string(),
    };

    // Recover timestamp
    let ts = Utc.timestamp_opt(packet.timestamp_coarse as i64, packet.timestamp_fine)
        .single()
        .unwrap_or_else(Utc::now);

    let mut record = TelemetryRecord {
        satellite_id,
        subsystem_id,
        mission_id: "unknown".to_string(),
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

    // Unpack User Data
    let ud = &packet.user_data;
    if ud.is_empty() {
        return Err(PacketError::ParserError("User data field empty".to_string()));
    }
    let data_apid = ud[0] as u16;
    if data_apid != packet.apid {
        return Err(PacketError::SubsystemMismatch(format!(
            "Payload header apid {} does not match packet apid {}",
            data_apid, packet.apid
        )));
    }

    let payload = &ud[1..];

    match packet.apid {
        APID_POWER => {
            if payload.len() < 24 {
                return Err(PacketError::ParserError("Power payload too short".to_string()));
            }
            record.battery_voltage = Some(f64::from_be_bytes(payload[0..8].try_into().unwrap()));
            record.battery_current = Some(f64::from_be_bytes(payload[8..16].try_into().unwrap()));
            record.solar_panel_output = Some(f64::from_be_bytes(payload[16..24].try_into().unwrap()));
        }
        APID_THERMAL => {
            if payload.len() < 8 {
                return Err(PacketError::ParserError("Thermal payload too short".to_string()));
            }
            record.temperature = Some(f64::from_be_bytes(payload[0..8].try_into().unwrap()));
        }
        APID_COMMS => {
            if payload.len() < 12 {
                return Err(PacketError::ParserError("Comms payload too short".to_string()));
            }
            record.communication_latency = Some(f32::from_be_bytes(payload[0..4].try_into().unwrap()));
            record.packet_loss = Some(f32::from_be_bytes(payload[4..8].try_into().unwrap()));
            record.signal_strength = Some(f32::from_be_bytes(payload[8..12].try_into().unwrap()));
        }
        APID_PROPULSION => {
            if payload.len() < 20 {
                return Err(PacketError::ParserError("Propulsion payload too short".to_string()));
            }
            record.velocity = Some(f64::from_be_bytes(payload[0..8].try_into().unwrap()));
            record.altitude = Some(f64::from_be_bytes(payload[8..16].try_into().unwrap()));
            record.fuel_remaining = Some(f32::from_be_bytes(payload[16..20].try_into().unwrap()));
        }
        APID_ADCS => {
            if payload.len() < 12 {
                return Err(PacketError::ParserError("ADCS payload too short".to_string()));
            }
            record.orientation_pitch = Some(f32::from_be_bytes(payload[0..4].try_into().unwrap()));
            record.orientation_roll = Some(f32::from_be_bytes(payload[4..8].try_into().unwrap()));
            record.orientation_yaw = Some(f32::from_be_bytes(payload[8..12].try_into().unwrap()));
        }
        APID_PAYLOAD => {
            if payload.len() < 9 {
                return Err(PacketError::ParserError("Payload payload too short".to_string()));
            }
            record.payload_power_draw = Some(f64::from_be_bytes(payload[0..8].try_into().unwrap()));
            record.payload_active = Some(payload[8] != 0);
        }
        APID_FLIGHT_COMPUTER => {
            if payload.len() < 8 {
                return Err(PacketError::ParserError("Flight computer payload too short".to_string()));
            }
            record.cpu_usage = Some(f32::from_be_bytes(payload[0..4].try_into().unwrap()));
            record.memory_usage = Some(f32::from_be_bytes(payload[4..8].try_into().unwrap()));
        }
        _ => {
            if payload.len() < 93 {
                return Err(PacketError::ParserError("Unified payload too short".to_string()));
            }
            record.battery_voltage = Some(f64::from_be_bytes(payload[0..8].try_into().unwrap()));
            record.battery_current = Some(f64::from_be_bytes(payload[8..16].try_into().unwrap()));
            record.solar_panel_output = Some(f64::from_be_bytes(payload[16..24].try_into().unwrap()));
            record.temperature = Some(f64::from_be_bytes(payload[24..32].try_into().unwrap()));
            record.communication_latency = Some(f32::from_be_bytes(payload[32..36].try_into().unwrap()));
            record.packet_loss = Some(f32::from_be_bytes(payload[36..40].try_into().unwrap()));
            record.signal_strength = Some(f32::from_be_bytes(payload[40..44].try_into().unwrap()));
            record.velocity = Some(f64::from_be_bytes(payload[44..52].try_into().unwrap()));
            record.altitude = Some(f64::from_be_bytes(payload[52..60].try_into().unwrap()));
            record.fuel_remaining = Some(f32::from_be_bytes(payload[60..64].try_into().unwrap()));
            record.orientation_pitch = Some(f32::from_be_bytes(payload[64..68].try_into().unwrap()));
            record.orientation_roll = Some(f32::from_be_bytes(payload[68..72].try_into().unwrap()));
            record.orientation_yaw = Some(f32::from_be_bytes(payload[72..76].try_into().unwrap()));
            record.payload_power_draw = Some(f64::from_be_bytes(payload[76..84].try_into().unwrap()));
            record.payload_active = Some(payload[84] != 0);
            record.cpu_usage = Some(f32::from_be_bytes(payload[85..89].try_into().unwrap()));
            record.memory_usage = Some(f32::from_be_bytes(payload[89..93].try_into().unwrap()));
        }
    }

    Ok(record)
}

fn scid_from_satellite_id(sat_id: &str) -> u16 {
    if let Some(num_str) = sat_id.strip_prefix("SAT-") {
        if let Ok(num) = num_str.parse::<u16>() {
            return num;
        }
    }
    0
}

fn satellite_id_from_scid(scid: u16) -> String {
    format!("SAT-{:04}", scid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_record() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: "SAT-0005".to_string(),
            subsystem_id: "power".to_string(),
            mission_id: "MISSION-ALPHA".to_string(),
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
        assert_eq!(decoded_rec.battery_voltage.unwrap(), rec.battery_voltage.unwrap());
        assert_eq!(decoded_rec.battery_current.unwrap(), rec.battery_current.unwrap());
        assert_eq!(decoded_rec.solar_panel_output.unwrap(), rec.solar_panel_output.unwrap());
        assert_eq!(decoded_rec.timestamp.timestamp(), rec.timestamp.timestamp());
    }

    #[test]
    fn test_corrupt_data_detection() {
        let rec = mock_record();
        let mut encoded_bytes = encode_record_to_frame(&rec, 101).unwrap();
        
        // Corrupt one byte of user data
        let len = encoded_bytes.len();
        encoded_bytes[len - 5] ^= 0x55;

        let res = decode_frame_to_record(&encoded_bytes);
        assert!(res.is_err());
        match res.unwrap_err() {
            PacketError::InvalidCrc { .. } => {} // Expected
            other => panic!("Expected CRC mismatch, got {:?}", other),
        }
    }
}
