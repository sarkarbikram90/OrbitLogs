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
pub const PUS_SERVICE_11_TIME_SCHEDULING: u8 = 11;
pub const PUS_SERVICE_12_MONITORING: u8 = 12;
pub const PUS_SERVICE_17_CONNECTION_TEST: u8 = 17;
pub const PUS_SERVICE_20_PARAMETER_MGMT: u8 = 20;

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

/// Service 11: Time-Based Scheduling (ECSS-E-ST-70-41C §6.11)
///
/// Supports time-tagged telecommand scheduling for autonomous on-board execution.
/// Used by ISRO ISTRAC for pre-programmed eclipse entry/exit maneuvers and
/// scheduled payload activations when the spacecraft is out of ground contact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Service11TimeSchedule {
    /// Scheduled absolute execution time as CUC (CCSDS Unsegmented Code) seconds since epoch
    pub execution_time_cuc: u32,
    /// Identifier of the telecommand to be executed at the scheduled time
    pub scheduled_tc_id: u16,
    /// Sub-schedule group identifier (allows grouping related scheduled commands)
    pub sub_schedule_id: u8,
    /// Whether this entry is currently enabled for execution
    pub enabled: bool,
}

/// Service 12: On-Board Monitoring (ECSS-E-ST-70-41C §6.12)
///
/// Defines parameter monitoring limits for autonomous anomaly detection.
/// The on-board computer continuously checks telemetry against these limits
/// and triggers Event Reports (Service 5) when violations are detected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Service12MonitorDefinition {
    /// Parameter ID being monitored
    pub parameter_id: u16,
    /// Lower warning limit (raw engineering value × 100)
    pub lower_limit: i32,
    /// Upper warning limit (raw engineering value × 100)
    pub upper_limit: i32,
    /// Number of consecutive violations before alert triggers
    pub check_validity_count: u8,
    /// Whether this monitor is currently active
    pub monitoring_enabled: bool,
}

/// Service 17: Connection Test (ECSS-E-ST-70-41C §6.17)
///
/// Simple heartbeat/ping service used to verify the telecommand uplink
/// and telemetry downlink paths are operational. ISTRAC ground stations
/// send [17,1] (are-you-alive) and expect [17,2] (alive) in response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Service17ConnectionTest {
    /// true = [17,2] alive response received; false = [17,1] request sent
    pub is_alive_response: bool,
    /// Round-trip latency in milliseconds (measured by ground station)
    pub round_trip_ms: Option<u16>,
}

/// Service 20: Parameter Management (ECSS-E-ST-70-41C §6.20)
///
/// Allows ground operators to read and modify on-board parameter values.
/// Used by ISRO for in-orbit reconfiguration of attitude control gains,
/// thermal setpoints, and communication link parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Service20ParameterReport {
    /// Number of parameters in this report
    pub parameter_count: u16,
    /// List of (parameter_id, raw_value) pairs
    pub parameters: Vec<(u16, i32)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PusPayload {
    TcVerification(Service1Report),
    Housekeeping(Service3Housekeeping),
    EventReport(Service5EventReport),
    FunctionExecution { function_id: u16, parameters: Vec<u8> },
    TimeSchedule(Service11TimeSchedule),
    OnBoardMonitor(Service12MonitorDefinition),
    ConnectionTest(Service17ConnectionTest),
    ParameterReport(Service20ParameterReport),
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
        PUS_SERVICE_11_TIME_SCHEDULING => {
            // [11,4] Detail report of a time-based schedule entry
            if data.len() < 7 {
                return Err(PusError::InsufficientData { required: 7, got: data.len() });
            }
            let execution_time_cuc = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            let scheduled_tc_id = u16::from_be_bytes([data[4], data[5]]);
            let sub_schedule_id = data[6] >> 4;
            let enabled = (data[6] & 0x01) == 1;

            Ok(PusPayload::TimeSchedule(Service11TimeSchedule {
                execution_time_cuc,
                scheduled_tc_id,
                sub_schedule_id,
                enabled,
            }))
        }
        PUS_SERVICE_12_MONITORING => {
            // [12,12] Check transition report (parameter monitoring definition)
            if data.len() < 8 {
                return Err(PusError::InsufficientData { required: 8, got: data.len() });
            }
            let parameter_id = u16::from_be_bytes([data[0], data[1]]);
            let lower_limit = i32::from_be_bytes([data[2], data[3], data[4], data[5]]);
            let upper_limit = if data.len() >= 10 {
                i32::from_be_bytes([data[6], data[7], data[8], data[9]])
            } else {
                // Short form: upper limit packed as i16
                i32::from(i16::from_be_bytes([data[6], data[7]]))
            };
            let check_validity_count = if data.len() > 10 { data[10] } else { 1 };
            let monitoring_enabled = if data.len() > 11 { data[11] != 0 } else { true };

            Ok(PusPayload::OnBoardMonitor(Service12MonitorDefinition {
                parameter_id,
                lower_limit,
                upper_limit,
                check_validity_count,
                monitoring_enabled,
            }))
        }
        PUS_SERVICE_17_CONNECTION_TEST => {
            // [17,1] are-you-alive test → [17,2] I-am-alive response
            let is_alive_response = subservice_type == 2;
            let round_trip_ms = if data.len() >= 2 {
                Some(u16::from_be_bytes([data[0], data[1]]))
            } else {
                None
            };

            Ok(PusPayload::ConnectionTest(Service17ConnectionTest {
                is_alive_response,
                round_trip_ms,
            }))
        }
        PUS_SERVICE_20_PARAMETER_MGMT => {
            // [20,2] Parameter value report
            if data.len() < 2 {
                return Err(PusError::InsufficientData { required: 2, got: data.len() });
            }
            let parameter_count = u16::from_be_bytes([data[0], data[1]]);
            let mut parameters = Vec::new();
            let mut offset = 2;
            for _ in 0..parameter_count {
                if offset + 6 > data.len() {
                    break;
                }
                let param_id = u16::from_be_bytes([data[offset], data[offset + 1]]);
                let raw_val = i32::from_be_bytes([
                    data[offset + 2],
                    data[offset + 3],
                    data[offset + 4],
                    data[offset + 5],
                ]);
                parameters.push((param_id, raw_val));
                offset += 6;
            }

            Ok(PusPayload::ParameterReport(Service20ParameterReport {
                parameter_count,
                parameters,
            }))
        }
        _ => Ok(PusPayload::Raw(data.to_vec())),
    }
}
