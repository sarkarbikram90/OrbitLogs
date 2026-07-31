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

//! OrbitLogs Ground Station Dish Antenna Edge Gateway Crate.
//!
//! Provides zero-copy CCSDS deframing, smart telemetry compression, and
//! QoS packet prioritization for edge deployment on WASM / ARM64 dish controllers.

pub mod compressor;
pub mod deframer;
pub mod priority;

pub use compressor::{compress_telemetry_frames, CompressedBatch};
pub use deframer::{deframe_ccsds_frame, find_asm_offset, DeframedFrame, DeframerError, CCSDS_ASM};
pub use priority::{classify_pus_packet, ClassifiedPacket, PacketPriority};
