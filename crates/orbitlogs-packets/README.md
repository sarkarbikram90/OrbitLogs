# OrbitLogs Packets Crate 🛰️📦

The **OrbitLogs Packets** library implements spacecraft binary telemetry serialization conforming to standard CCSDS 133.0-B-2 space packets.

## Key Features

- **CCSDS Header Encoding & Decoding**: Parses and structures space packet primary and secondary header attributes.
- **CRC-16 Integrity Checks**: Implements standard CRC-16-CCITT checksum validation routines on ground link boundaries to identify atmospheric transmission errors.
- **Performance**: High-throughput, zero-copy serialization mechanisms using buffers.

## Usage

Define sub-crates to inherit:
```toml
orbitlogs-packets = { workspace = true }
```

Serialize a record:
```rust
use orbitlogs_packets::{encode_record_to_frame, decode_frame_to_record};

let frame_bytes = encode_record_to_frame(&record, sequence_count)?;
let decoded_record = decode_frame_to_record(&frame_bytes)?;
```
