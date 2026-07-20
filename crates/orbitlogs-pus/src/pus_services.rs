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

//! ECSS PUS (Packet Utilisation Standard, ECSS-E-ST-70-41C) services parsing.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum PusError {
    #[error("Packet data too short for PUS header: required {required}, got {got}")]
    InsufficientData { required: usize, got: usize },
    #[error("Unsupported PUS Service Type: {0}")]
    UnsupportedService(u8),
    #[error("Invalid Subservice Type: {0}")]
    InvalidSubtype(u8),
}

/// ECSS PUS Service Types standard numbers
pub const PUS_SERVICE_1_TC_VERIFICATION: u8 = 1;
pub const PUS_SERVICE_3_HOUSEKEEPING: u8 = 3;
pub const PUS_SERVICE_5_EVENT_REPORTING: u8 = 5;
pub const PUS_SERVICE_8_FUNCTION_MGMT: u8 = 8;

/// PUS Secondary Header
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PusSecondaryHeader {
    pub pus_version: u8,
    pub service_type: u8,
    pub subservice_type: u8,
    pub source_id: u16,
}

impl PusSecondaryHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, PusError> {
        if bytes.len() < 5 {
            return Err(PusError::InsufficientData {
                required: 5,
                got: bytes.len(),
            });
        }
        let pus_version = (bytes[0] >> 4) & 0x07;
        let service_type = bytes[1];
        let subservice_type = bytes[2];
        let source_id = u16::from_be_bytes([bytes[3], bytes[4]]);

        Ok(Self {
            pus_version,
            service_type,
            subservice_type,
            source_id,
        })
    }
}

/// Service 1: Telecommand Verification Reports
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Service1Report {
    pub tc_packet_id: u16,
    pub tc_sequence_control: u16,
    pub success: bool,
    pub error_code: Option<u16>,
}

/// Service 3: Housekeeping Telemetry Data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Service3Housekeeping {
    pub hk_parameter_report_structure_id: u16,
    pub battery_voltage_v: f64,
    pub battery_current_a: f64,
    pub solar_output_w: f64,
    pub temp_c: f64,
    pub fuel_pct: f32,
}

/// Service 5: Event Reporting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Service5EventReport {
    pub event_definition_id: u16,
    pub severity_level: u8, // 1: Info, 2: Low, 3: Medium, 4: High Anomaly
    pub event_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PusPayload {
    TcVerification(Service1Report),
    Housekeeping(Service3Housekeeping),
    EventReport(Service5EventReport),
    FunctionExecution { function_id: u16, parameters: Vec<u8> },
    Raw(Vec<u8>),
}

pub fn parse_pus_payload(service_type: u8, subservice_type: u8, data: &[u8]) -> Result<PusPayload, PusError> {
    match service_type {
        PUS_SERVICE_1_TC_VERIFICATION => {
            if data.len() < 4 {
                return Err(PusError::InsufficientData { required: 4, got: data.len() });
            }
            let tc_packet_id = u16::from_be_bytes([data[0], data[1]]);
            let tc_sequence_control = u16::from_be_bytes([data[2], data[3]]);
            let success = subservice_type == 1 || subservice_type == 7; // [1,1] Accept success, [1,7] Execution success
            let error_code = if !success && data.len() >= 6 {
                Some(u16::from_be_bytes([data[4], data[5]]))
            } else {
                None
            };
            Ok(PusPayload::TcVerification(Service1Report {
                tc_packet_id,
                tc_sequence_control,
                success,
                error_code,
            }))
        }
        PUS_SERVICE_3_HOUSEKEEPING => {
            if data.len() < 10 {
                return Err(PusError::InsufficientData { required: 10, got: data.len() });
            }
            let struct_id = u16::from_be_bytes([data[0], data[1]]);
            let raw_v = u16::from_be_bytes([data[2], data[3]]);
            let raw_i = u16::from_be_bytes([data[4], data[5]]);
            let raw_w = u16::from_be_bytes([data[6], data[7]]);
            let raw_t = i16::from_be_bytes([data[8], data[9]]);

            Ok(PusPayload::Housekeeping(Service3Housekeeping {
                hk_parameter_report_structure_id: struct_id,
                battery_voltage_v: raw_v as f64 / 100.0,
                battery_current_a: raw_i as f64 / 1000.0,
                solar_output_w: raw_w as f64 / 10.0,
                temp_c: raw_t as f64 / 10.0,
                fuel_pct: if data.len() >= 11 { data[10] as f32 } else { 100.0 },
            }))
        }
        PUS_SERVICE_5_EVENT_REPORTING => {
            if data.len() < 3 {
                return Err(PusError::InsufficientData { required: 3, got: data.len() });
            }
            let event_def_id = u16::from_be_bytes([data[0], data[1]]);
            let severity_level = subservice_type;
            let event_data = data[2..].to_vec();

            Ok(PusPayload::EventReport(Service5EventReport {
                event_definition_id: event_def_id,
                severity_level,
                event_data,
            }))
        }
        PUS_SERVICE_8_FUNCTION_MGMT => {
            if data.len() < 2 {
                return Err(PusError::InsufficientData { required: 2, got: data.len() });
            }
            let function_id = u16::from_be_bytes([data[0], data[1]]);
            let parameters = data[2..].to_vec();
            Ok(PusPayload::FunctionExecution { function_id, parameters })
        }
        _ => Ok(PusPayload::Raw(data.to_vec())),
    }
}
