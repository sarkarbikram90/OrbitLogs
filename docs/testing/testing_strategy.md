# OrbitLogs: Testing Strategy & Verification Guide
### Version: v4.0.0 (Latest)

This document describes the testing layers, integration strategies, validation pipelines, and benchmarking methodologies utilized to guarantee the performance and correctness of **OrbitLogs**.

---

## 1. Testing Hierarchy

Our verification plan enforces a strict testing hierarchy from isolated unit tests to high-concurrency load benchmarks:

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

Unit tests are written inside the respective crates and applications:
- **Digital Twin Projection Tests**: Inside `digital-twin-engine` ([main.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/digital-twin-engine/src/main.rs)), unit tests validate battery, thermal, ADCS drift, and fuel depletion threshold outputs.
- **RNG State Validation**: Inside `telemetry-simulator`, unit tests confirm that deterministic seeded loops yield matching telemetry streams.
- **Model Checks**: Inside `orbitlogs-models`, tests assert range boundaries for pitch/yaw, signal strength, and voltage attributes.

Run all workspace tests:
```bash
cargo test --all
```

---

## 3. Integration Testing Layer

Integration tests are defined in the telemetry-collector testing module:
- **HTTP Endpoint Authentication**: Validates JWT role authorizations and token exclusions.
- **Ingress Limit Enforcement**: Verifies that payloads exceeding the 10KB limit are immediately rejected with `413 Payload Too Large`.
- **Ingestion Rate Limiting**: Verifies that requests exceeding 100 requests per second are throttled with `429 Too Many Requests`.
- **CORS Constraints**: Asserts origin headers align with `CORS_ALLOWED_ORIGIN` variables.

Run integration tests:
```bash
cargo test -p telemetry-collector
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
