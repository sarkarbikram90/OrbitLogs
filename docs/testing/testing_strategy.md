# Open SpaceOps: Testing Strategy & Verification Guide

This document describes the testing layers, integration strategies, validation pipelines, and benchmarking methodologies utilized to guarantee the performance and correctness of the **Open SpaceOps** platform.

---

## 1. Testing Hierarchy

Our verification plan enforces a strict testing hierarchy from isolated units to high-concurrency load benchmarks:

```
    ┌──────────────────────────────────────────────┐
    │          k6 Ingestion Load Tests             │  ◄── Ingestion & write latency under stress
    └──────────────────────┬───────────────────────┘
                           ▼
    ┌──────────────────────────────────────────────┐
    │          E2E Integration Tests               │  ◄── End-to-end pipeline verification
    └──────────────────────┬───────────────────────┘
                           ▼
    ┌──────────────────────────────────────────────┐
    │          Rust Unit Test Suite                │  ◄── Model constraints & RNG states
    └──────────────────────────────────────────────┘
```

---

## 2. Unit Testing Layer

Unit tests are written inside the respective crates (`telemetry-simulator` and `telemetry-collector`) to keep feedback loops fast.
- **Simulator RNG Assertions**:
  - Validates that two simulator tasks running with identical seeds produce identical pseudo-random telemetry streams.
  - Ensures scenario state transitions (like linear fuel leakage and temperature spikes) evolve correctly across ticks.
- **Model Validation Assertions**:
  - Tests bounds limits on data fields (such as pitch/roll/yaw, voltages, signal strengths).
  - Asserts that malformed inputs trigger detailed, multi-field validation error collections.

Run the unit tests:
```bash
cargo test --lib
```

---

## 3. Integration Testing Layer

Integration tests are defined in the `apps/telemetry-collector/tests/` directory:
- **Scope**: Verifies integration between payload parser structures and validation rules.
- **Execution**: Can be run locally alongside standard test routines.

Run the integration tests:
```bash
cargo test --test integration_tests
```

---

## 4. High-Velocity Load Benchmarking

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
