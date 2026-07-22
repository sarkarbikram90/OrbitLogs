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

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryFrame {
    // Frame Header (6 bytes)
    pub frame_version: u8,         // 2 bits
    pub spacecraft_id: u16,        // 10 bits
    pub virtual_channel_id: u8,    // 3 bits
    pub ocf_flag: bool,            // 1 bit
    pub master_frame_count: u8,    // 8 bits
    pub vc_frame_count: u8,        // 8 bits
    pub first_header_pointer: u16, // 11 bits + 5 spare

    // Frame Data Field
    pub data_field: Vec<u8>,

    // Error Control (2 bytes)
    pub frame_error_control: u16, // CRC-16
}

impl TelemetryFrame {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(6 + self.data_field.len() + 2);

        // 1. Pack Frame Header (6 bytes)
        let word1 = ((self.frame_version & 0x03) as u16) << 14
            | (self.spacecraft_id & 0x03FF) << 4
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
            return Err(PacketError::InvalidLength(
                "Frame is too short for the header + CRC".to_string(),
            ));
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
