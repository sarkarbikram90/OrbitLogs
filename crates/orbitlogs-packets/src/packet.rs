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

use crate::crc;
use crate::error::PacketError;

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
    pub(crate) packet_version: u8, // 3 bits
    pub packet_type: u8,           // 1 bit
    pub sec_header_flag: bool,     // 1 bit
    pub apid: u16,                 // 11 bits
    pub sequence_flags: u8,        // 2 bits
    pub sequence_count: u16,       // 14 bits
    pub packet_length: u16,        // 16 bits (Length of user data + sec header + CRC - 1)

    // Secondary Header (8 bytes)
    pub(crate) timestamp_coarse: u32,
    pub(crate) timestamp_fine: u32,

    // User Data Field (variable length)
    pub user_data: Vec<u8>,

    // Error Control (2 bytes)
    pub packet_error_control: u16, // CRC-16
}

impl TelemetryPacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(6 + 8 + self.user_data.len() + 2);

        // 1. Pack Primary Header (6 bytes)
        let word1 = ((self.packet_version & 0x07) as u16) << 13
            | ((self.packet_type & 0x01) as u16) << 12
            | (self.sec_header_flag as u16) << 11
            | (self.apid & 0x07FF);

        let word2 = ((self.sequence_flags & 0x03) as u16) << 14 | (self.sequence_count & 0x3FFF);

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
            return Err(PacketError::InvalidLength(
                "Packet is too short for minimal header".to_string(),
            ));
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
                "Packet length mismatch: expected {} bytes, received {} bytes",
                total_expected_len,
                bytes.len()
            )));
        }

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
                return Err(PacketError::InvalidLength(
                    "Packet is missing secondary header bytes".to_string(),
                ));
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

/// Fixed-size packet primary header for zero-copy access.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    pub word1: u16,
    pub word2: u16,
    pub packet_length: u16,
}

impl PacketHeader {
    /// Deserializes a `PacketHeader` from an unaligned or aligned byte slice without panicking or requiring alignment.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, PacketError> {
        let header_size = std::mem::size_of::<PacketHeader>();
        if bytes.len() < header_size {
            return Err(PacketError::InvalidLength(format!(
                "Buffer too short for PacketHeader: need {} bytes, got {}",
                header_size,
                bytes.len()
            )));
        }

        let word1 = u16::from_be_bytes([bytes[0], bytes[1]]);
        let word2 = u16::from_be_bytes([bytes[2], bytes[3]]);
        let packet_length = u16::from_be_bytes([bytes[4], bytes[5]]);

        Ok(Self {
            word1,
            word2,
            packet_length,
        })
    }

    pub fn apid(&self) -> u16 {
        self.word1 & 0x07FF
    }

    pub fn version(&self) -> u8 {
        ((self.word1 >> 13) & 0x07) as u8
    }

    pub fn sequence_count(&self) -> u16 {
        self.word2 & 0x3FFF
    }

    pub fn data_length(&self) -> u16 {
        self.packet_length
    }
}

pub fn parse_header(bytes: &[u8]) -> Result<PacketHeader, PacketError> {
    PacketHeader::from_slice(bytes)
}

#[cfg(test)]
mod header_tests {
    use super::*;

    #[test]
    fn parse_header_valid() {
        let bytes: Vec<u8> = vec![0x08, 0x01, 0xC0, 0x05, 0x00, 0x0F];
        let header = parse_header(&bytes).unwrap();
        assert_eq!(header.apid(), 1);
        assert_eq!(header.sequence_count(), 5);
        assert_eq!(header.data_length(), 15);
    }

    #[test]
    fn parse_header_unaligned_slice() {
        // Create an unaligned buffer by offsetting a vector by 1 byte
        let padded: Vec<u8> = vec![0xFF, 0x08, 0x01, 0xC0, 0x05, 0x00, 0x0F];
        let unaligned_slice = &padded[1..];
        let header = parse_header(unaligned_slice).unwrap();
        assert_eq!(header.apid(), 1);
        assert_eq!(header.sequence_count(), 5);
        assert_eq!(header.data_length(), 15);
    }

    #[test]
    fn parse_header_too_short() {
        let bytes = [0x08, 0x01];
        assert!(parse_header(&bytes).is_err());
    }

    #[test]
    fn parse_header_empty() {
        assert!(parse_header(&[]).is_err());
    }
}

