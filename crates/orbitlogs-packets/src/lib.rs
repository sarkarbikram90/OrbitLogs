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

pub mod convert;
pub mod crc;
pub mod error;
pub mod frame;
pub mod packet;
pub mod tests;

pub use convert::{decode_frame_to_record, encode_record_to_frame};
pub use error::PacketError;
pub use frame::TelemetryFrame;
pub use packet::{
    parse_header, PacketHeader, TelemetryPacket, APID_ADCS, APID_COMMS, APID_FLIGHT_COMPUTER,
    APID_PAYLOAD, APID_POWER, APID_PROPULSION, APID_THERMAL, APID_UNIFIED,
};
