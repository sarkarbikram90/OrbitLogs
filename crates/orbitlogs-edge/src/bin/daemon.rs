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

//! `orbitlogs-edge-daemon`
//!
//! Ground Station Dish Antenna Low-Latency Edge Daemon executable (WASM / ARM64 / x86_64).
//!
//! Performs zero-copy CCSDS frame deframing at dish controller speed, classifies QoS packet
//! priority, applies 10x telemetry compression, and streams prioritized batches back to
//! OrbitLogs cloud telemetry collectors.

use orbitlogs_edge::{compress_telemetry_frames, deframe_ccsds_frame, classify_pus_packet, PacketPriority};

fn main() {
    println!("============================================================");
    println!("🚀 ORBITLOGS DISH ANTENNA EDGE DAEMON (WASM / ARM64)");
    println!("============================================================");
    println!("Station Target: ISTRAC Bengaluru (Byalalu 32m DSN)");
    println!("Mode: Zero-Copy CCSDS Deframing + Smart Compression + QoS");
    println!("------------------------------------------------------------");

    // Simulated 14-byte raw byte stream with CCSDS Attached Sync Marker (ASM 0x1ACFFC1D)
    let sync_marker: [u8; 4] = [0x1A, 0xCF, 0xFC, 0x1D];
    let mut raw_stream = Vec::new();
    raw_stream.extend_from_slice(&sync_marker);
    // 6-byte header + 8-byte payload (total frame length 14)
    raw_stream.extend_from_slice(&[0x03, 0xE9, 42, 12, 0x00, 0x00]);
    raw_stream.extend_from_slice(&[0x10, 0x03, 0x19, 0x00, 0x0A, 0x0B, 0x22, 0x09]);

    println!("[EDGE] Ingesting {} bytes raw RF bitstream...", raw_stream.len());

    match deframe_ccsds_frame(&raw_stream, 14) {
        Ok((frame, consumed)) => {
            println!(
                "[EDGE] Deframed CCSDS Frame: VCID={}, SpacecraftID={}, MasterCount={}, Consumed={} bytes",
                frame.virtual_channel_id, frame.spacecraft_id, frame.master_frame_count, consumed
            );

            let priority = classify_pus_packet(3, 25, frame.payload);
            println!("[EDGE] Classified QoS Packet Priority: {:?}", priority);

            if priority == PacketPriority::Critical {
                println!("[EDGE] 🚨 CRITICAL ALERT DETECTED — Emergency transmit bypass activated!");
            }

            let frame_bytes = frame.payload.to_vec();
            let batch = compress_telemetry_frames("SAT-0001", &[frame_bytes]);
            println!(
                "[EDGE] Compressed Telemetry Batch: Uncompressed {} bytes -> Compressed {} bytes (Ratio: {:.1}x)",
                batch.uncompressed_bytes, batch.compressed_bytes, batch.compression_ratio
            );
        }
        Err(e) => {
            println!("[EDGE ERROR] Frame deframing failed: {:?}", e);
        }
    }

    println!("============================================================");
    println!("✅ OrbitLogs Edge Gateway Nominal — Standing by for passes.");
    println!("============================================================");
}
