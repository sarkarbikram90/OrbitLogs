# OrbitLogs Test Utilities Crate 🧪🛠️

The **OrbitLogs Test Utilities** library provides centralized mock factories and test builders for the workspace.

## Core Features

- **Test Fixture Generation**: Exposes simple builders like `mock_telemetry_record(sat_id)` to quickly instantiate complete telemetry frames.
- **Dependency Reduction**: Avoids repeating mock data setups across multiple separate app and library test targets.

## Usage

Inherit in test profiles:
```toml
[dev-dependencies]
orbitlogs-test-utils = { workspace = true }
```

Sample test:
```rust
use orbitlogs_test_utils::mock_telemetry_record;

#[test]
fn test_some_processor() {
    let mock_rec = mock_telemetry_record("SAT-0001");
    assert_eq!(mock_rec.battery_voltage, Some(13.8));
}
```
