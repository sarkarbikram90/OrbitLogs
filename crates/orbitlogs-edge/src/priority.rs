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

//! Critical Packet Prioritization & Quality of Service (QoS) Classifier.
//!
//! Classifies incoming edge telemetry packets into priority queues so that
//! critical anomalies (PUS Service 5) and command verification reports (PUS Service 1)
//! bypass bandwidth throttling on satellite backhaul links.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PacketPriority {
    /// Nominal Housekeeping (PUS Service 3) — backhaul low priority (can be compressed/delayed)
    Low = 0,
    /// Function execution reports & parameter dumps — medium priority
    Medium = 1,
    /// Event reporting anomalies (PUS Service 5) & command acknowledgments (PUS Service 1) — high priority
    High = 2,
    /// Critical anomaly alert / safe-mode trigger — instant emergency transmit
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedPacket {
    pub sequence_number: u32,
    pub priority: PacketPriority,
    pub service_type: u8,
    pub subservice_type: u8,
    pub payload_len: usize,
    pub timestamp_ms: u64,
}

/// Classifies a PUS packet based on service type and payload flags.
pub fn classify_pus_packet(service_type: u8, subservice_type: u8, payload: &[u8]) -> PacketPriority {
    match service_type {
        1 => PacketPriority::High, // Telecommand verification
        5 => {
            // Event reporting: subservice indicates severity
            match subservice_type {
                4 => PacketPriority::Critical, // High severity anomaly
                3 => PacketPriority::High,     // Medium severity anomaly
                _ => PacketPriority::Medium,
            }
        }
        3 => {
            // Check if battery voltage or temp indicates emergency
            if payload.len() >= 10 {
                let temp_raw = i16::from_be_bytes([payload[8], payload[9]]);
                if temp_raw > 650 || temp_raw < -200 { // > 65.0°C or < -20.0°C
                    return PacketPriority::High;
                }
            }
            PacketPriority::Low
        }
        8 => PacketPriority::Medium,
        17 => PacketPriority::High, // Heartbeat / connection test
        _ => PacketPriority::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_priority_classification() {
        assert_eq!(classify_pus_packet(1, 1, &[]), PacketPriority::High);
        assert_eq!(classify_pus_packet(5, 4, &[]), PacketPriority::Critical);
        assert_eq!(classify_pus_packet(3, 25, &[0; 10]), PacketPriority::Low);
    }
}
