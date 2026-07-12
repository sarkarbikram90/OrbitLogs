# OrbitLogs Models Crate 📦

The **OrbitLogs Models** library serves as the shared data model and type system layer across the entire monorepo workspace.

## Key Modules

- **`define_newtype_id!`**: A procedural-style macro generating validated, domain-specific newtype identifiers (e.g. `SatelliteId`, `MissionId`, `StationId`) with prefix validation ("Parse, Don't Validate").
- **`bounded`**: Implements safe wrapper types that enforce bounded ranges (e.g., `Percentage` [0.0-100.0], `Voltage`, `Temperature`, and `OrbitalVelocity`) at construction to make invalid states unrepresentable.
- **`command_state`**: Implements type-safe compile-time status states (`TypedCommand<S>`) using the typestate status machine pattern.
- **`db`**: Manages common Postgres/TimescaleDB connections and transaction pools.
- **`kafka`**: Implements builder abstractions for Kafka mutual TLS (mTLS) network configurations, broker settings, and consumer metrics.

## Usage

Define sub-crates to inherit:
```toml
orbitlogs-models = { workspace = true }
```
