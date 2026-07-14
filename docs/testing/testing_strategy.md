# OrbitLogs: Testing Strategy & Verification Guide
### Version: v4.5.0 (Latest)

This document describes the testing layers, integration strategies, validation pipelines, and benchmarking methodologies utilized to guarantee the performance and correctness of **OrbitLogs**.

---

## 1. Testing Hierarchy

Our verification plan enforces a strict testing hierarchy from isolated unit tests to high-concurrency load benchmarks:

```
    ┌──────────────────────────────────────────────┐
    │       Playwright Visual E2E Tests            │  ◄── 3D canvas, WebGL, WebSocket assertions
    └──────────────────────┬───────────────────────┘
                           ▼
    ┌──────────────────────────────────────────────┐
    │          k6 Ingestion Load Tests             │  ◄── Ingestion & write latency under stress
    └──────────────────────┬───────────────────────┘
                           ▼
    ┌──────────────────────────────────────────────┐
    │          E2E Integration Tests               │  ◄── End-to-end pipeline verification
    └──────────────────────┬───────────────────────┘
                           ▼
    ┌──────────────────────────────────────────────┐
    │     Property-Based Tests (proptest)          │  ◄── CCSDS packet fuzzing & model invariants
    └──────────────────────┬───────────────────────┘
                           ▼
    ┌──────────────────────────────────────────────┐
    │          Rust Unit Test Suite                │  ◄── Model constraints & RNG states
    └──────────────────────────────────────────────┘
```

---

## 2. Unit Testing Layer

Unit tests are written inside the respective crates and applications:
- **Digital Twin Projection Tests**: Inside `digital-twin-engine` ([simulation.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/digital-twin-engine/src/simulation.rs)), unit tests validate battery, thermal, ADCS drift, and fuel depletion threshold outputs.
- **RNG State Validation**: Inside `telemetry-simulator`, unit tests confirm that deterministic seeded loops yield matching telemetry streams.
- **Model Checks**: Inside `orbitlogs-models`, tests assert range boundaries for pitch/yaw, signal strength, and voltage attributes via bounded newtypes.
- **Orbit Operations Unit Tests**: Inside `orbit-ops-service`, comprehensive unit test suites cover:
  - `DirectorManager` ([director.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/orbit-ops-service/src/director.rs)): Telemetry evaluation rules and AI mission direction logic.
  - `HealthScoringManager` ([health.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/orbit-ops-service/src/health.rs)): Composite health scoring algorithms.
  - `TwinManager` ([twin.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/orbit-ops-service/src/twin.rs)): Command application, telemetry failure cascades, and twin state synchronization.
- **Command Service Tests**: Inside `mission-command-service`, tests validate constant-time comparison, role parsing, and JWT signature verification.

Run all workspace tests:
```bash
cargo test --all
```

---

## 3. Property-Based Testing (proptest)

The `orbitlogs-packets` crate uses the [`proptest`](https://crates.io/crates/proptest) framework for property-based fuzz testing:
- **Roundtrip Invariant**: Arbitrary telemetry records are generated, encoded to CCSDS binary frames with CRC-16 checksums, decoded back, and asserted to match the original input.
- **CRC Integrity**: Randomly mutated frame bytes are verified to fail CRC validation, confirming that the checksum catches all single-bit corruption.
- **Boundary Values**: Tests cover edge cases for packet lengths, subsystem IDs, and overflow conditions.

---

## 4. Integration Testing Layer

Integration tests are defined in the telemetry-collector testing module:
- **HTTP Endpoint Authentication**: Validates JWT role authorizations and token exclusions.
- **Ingress Limit Enforcement**: Verifies that payloads exceeding the 10KB limit are immediately rejected with `413 Payload Too Large`.
- **Ingestion Rate Limiting**: Verifies that requests exceeding 100 requests per second are throttled with `429 Too Many Requests`.
- **CORS Constraints**: Asserts origin headers align with `CORS_ALLOWED_ORIGIN` variables.
- **Binary CCSDS Codec**: Validates end-to-end binary frame decoding against known valid and invalid inputs.

Run integration tests:
```bash
cargo test -p telemetry-collector
```

---

## 5. Playwright E2E Visual Tests

A complete Playwright end-to-end visual test suite (`playwright.config.ts` and `visual-canvas.spec.ts`) in `apps/mission-control-ui` validates:
- **3D Three.js Canvas Dimensions**: Asserts that the WebGL canvas renders at the expected viewport dimensions.
- **Responsive Viewport Scaling**: Verifies canvas behavior across mobile, tablet, and desktop breakpoints.
- **WebGL Compatibility**: Confirms that the browser supports and initializes a WebGL context.
- **Real-Time WebSocket Communication**: Validates that live telemetry data flows from the backend to the 3D visualizer under load.

Run Playwright tests:
```bash
cd apps/mission-control-ui
npx playwright test
```

---

## 6. Shared Test Utilities (`crates/orbitlogs-test-utils`)

The `orbitlogs-test-utils` crate centralizes mock generation functions and test builders:
- Eliminates duplication of mock `TelemetryRecord`, `HealthAssessment`, and `Command` builders across multiple service test suites.
- Provides reusable test fixture factories using `#[cfg(test)]` compilation flags.
- Available to all workspace members via `orbitlogs-test-utils = { workspace = true }`.

---

## 7. High-Velocity Load Benchmarking

To validate the ingestion pipeline against target throughput metrics (1,000, 5,000, and 10,000 events/sec), we execute **k6** benchmark scenarios.

### k6 Configuration (`benchmarks/load_test.js`)
- **Arrival Rate Executor**: Enforces a constant target arrival rate independent of system latencies:
  ```javascript
  executor: 'constant-arrival-rate',
  rate: TARGET_RATE, // 1000, 5000, 10000
  timeUnit: '1s'
  ```
- **Virtual Users**: Pre-allocates VUs to ensure the benchmark engine is never a bottleneck.

### Running Benchmarks
Run the automated script to execute all rates sequentially and output JSON summary files:
```bash
./scripts/benchmark.sh
```

Performance goals include:
- **Error Ratio**: Less than **1.0%** overall failed requests.
- **Average Latency**: 95th percentile latency (p95) below **100ms** under peak 10,000 events/sec loads.

