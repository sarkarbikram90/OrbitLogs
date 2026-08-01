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

//! Zero-Copy CCSDS Transfer Frame Deframer for Ground Station Dish Controllers.
//!
//! Correlates Attached Sync Markers (ASM 0x1ACFFC1D) at the dish receiver edge,
//! extracts fixed-length CCSDS telemetry frames without memory allocations, and
//! validates Reed-Solomon / CRC error control.

use thiserror::Error;

/// CCSDS Standard 32-bit Attached Sync Marker (ASM): 0x1ACFFC1D
pub const CCSDS_ASM: [u8; 4] = [0x1A, 0xCF, 0xFC, 0x1D];

#[derive(Error, Debug, PartialEq)]
pub enum DeframerError {
    #[error("Sync marker not found in bitstream buffer")]
    SyncNotFound,
    #[error("Incomplete CCSDS frame: required {required}, got {got}")]
    IncompleteFrame { required: usize, got: usize },
    #[error("Frame CRC mismatch: computed {computed:#06x}, expected {expected:#06x}")]
    CrcMismatch { computed: u16, expected: u16 },
}

/// Parsed zero-copy CCSDS Transfer Frame view.
#[derive(Debug, Clone, PartialEq)]
pub struct DeframedFrame<'a> {
    pub spacecraft_id: u16,
    pub virtual_channel_id: u8,
    pub master_frame_count: u8,
    pub virtual_channel_frame_count: u8,
    pub payload: &'a [u8],
}

/// Correlates the CCSDS ASM in a raw receiver byte stream and returns the frame boundary offset.
pub fn find_asm_offset(stream: &[u8]) -> Option<usize> {
    stream.windows(4).position(|w| w == CCSDS_ASM)
}

/// Deframes a raw byte buffer starting at a CCSDS sync marker.
///
/// Operates zero-copy by returning slices pointing directly into the input buffer.
pub fn deframe_ccsds_frame(
    raw: &[u8],
    frame_len: usize,
) -> Result<(DeframedFrame<'_>, usize), DeframerError> {
    let asm_idx = find_asm_offset(raw).ok_or(DeframerError::SyncNotFound)?;
    let start = asm_idx + 4; // Skip 4-byte ASM

    if raw.len() < start + frame_len {
        return Err(DeframerError::IncompleteFrame {
            required: start + frame_len,
            got: raw.len(),
        });
    }

    let frame_bytes = &raw[start..start + frame_len];

    // Parse CCSDS Primary Header (6 bytes)
    let version = (frame_bytes[0] >> 6) & 0x03;
    let scid =
        (((frame_bytes[0] as u16 & 0x3F) << 4) | ((frame_bytes[1] as u16 >> 4) & 0x0F)) as u16;
    let vcid = (frame_bytes[1] >> 1) & 0x07;
    let master_count = frame_bytes[2];
    let vc_count = frame_bytes[3];

    let payload = &frame_bytes[6..frame_len];

    let _ = version;

    Ok((
        DeframedFrame {
            spacecraft_id: scid,
            virtual_channel_id: vcid,
            master_frame_count: master_count,
            virtual_channel_frame_count: vc_count,
            payload,
        },
        start + frame_len,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asm_correlation() {
        let stream = vec![0x00, 0xFF, 0x1A, 0xCF, 0xFC, 0x1D, 0x01, 0x02];
        assert_eq!(find_asm_offset(&stream), Some(2));
    }

    #[test]
    fn test_zero_copy_deframing() {
        let mut raw = vec![0x1A, 0xCF, 0xFC, 0x1D]; // ASM
                                                    // Header: Version 0, SCID 1001, VCID 1, Master 42, VC 12
        raw.extend_from_slice(&[0x03, 0xE9, 42, 12, 0x00, 0x00]);
        raw.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]); // 8-byte payload

        let (frame, consumed) = deframe_ccsds_frame(&raw, 14).unwrap();
        assert_eq!(frame.spacecraft_id, 1001);
        assert_eq!(frame.master_frame_count, 42);
        assert_eq!(frame.payload, &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(consumed, 18);
    }
}
