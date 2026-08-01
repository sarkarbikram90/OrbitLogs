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

#[derive(Debug)]
pub enum PacketError {
    InvalidLength(String),
    InvalidCrc {
        expected: u16,
        calculated: u16,
    },
    UnknownApid(u16),
    SubsystemMismatch(String),
    ParserError(String),
    Alignment(String),
    /// Invalid raw bit pattern (e.g., zero-length telemetry for OCF frames).
    InvalidPattern(String),
}

impl std::fmt::Display for PacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketError::InvalidLength(s) => write!(f, "Data length is too short: {}", s),
            PacketError::InvalidCrc {
                expected,
                calculated,
            } => write!(
                f,
                "Invalid CRC-16 checksum: expected {:#06X}, got {:#06X}",
                expected, calculated
            ),
            PacketError::UnknownApid(a) => write!(f, "Unknown APID: {}", a),
            PacketError::SubsystemMismatch(s) => write!(f, "Subsystem mismatch: {}", s),
            PacketError::ParserError(s) => write!(f, "Failed to parse field: {}", s),
            PacketError::Alignment(s) => write!(f, "Alignment error: {}", s),
            PacketError::InvalidPattern(s) => write!(f, "Invalid pattern error: {}", s),
        }
    }
}

impl std::error::Error for PacketError {}
