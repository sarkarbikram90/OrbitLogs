# Space Schema Registry Crate 🧬

The **Space Schema Registry** library implements runtime version check validation and format compatibility checks (Backward, Forward, or Full) for satellite telemetry formats.

## Key Features

- **Schema Versioning**: Manages transitions between V1 (Legacy), V2 (Standard), and V3 (Extended) telemetry layouts.
- **Compatibility Modes**:
  - `BACKWARD`: New software can read historical data.
  - `FORWARD`: Legacy systems can process new formats without crashes.
  - `FULL`: Combines both forward and backward requirements.
- **Runtime Schema Validation**: Scans payload attributes before ingestion loops to fail fast and reject format skewed packages.

## Usage

```toml
space-schema-registry = { workspace = true }
```
