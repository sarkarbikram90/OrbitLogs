# Open SpaceOps — Staff-Level Engineering Review

> **Reviewer Role**: Staff Engineer (SpaceX Avionics / Ground Software)
> **Date**: 2026-06-06
> **Scope**: Full codebase, architecture, system design, CI/CD, deployment, security, testing

---

## Overall Rating: 7.4 / 10

This is a genuinely impressive project for an open-source portfolio-grade system. It demonstrates strong systems thinking, real aerospace domain knowledge (CCSDS framing, subsystem cross-effects, ground station link budgets), and production-oriented engineering patterns (DLQ routing, at-least-once delivery, non-root containers). The multi-service event-driven architecture is well-decomposed, and the 3D Mission Control UI adds real presentation polish.

However, a Staff-level review at SpaceX must look beyond "does it work" toward "can I trust this system during a critical mission pass." There are meaningful gaps in observability depth, error domain modeling, replay engine scalability, telemetry state management correctness, and test coverage breadth that would need to be addressed before any production flight software team would sign off on this stack.

---

## Dimension Scores

| Dimension | Score | Notes |
|:---|:---:|:---|
| **Architecture & Design** | 8.0 | Clean decoupled event-driven topology. Proper separation of concerns across 6 services + 2 shared crates. |
| **Code Quality & Rust Idioms** | 7.5 | Solid Rust code overall. Some anti-patterns (duplicated code, `Mutex` poisoning risk, `unwrap()` in prod paths). |
| **Domain Fidelity** | 8.5 | Excellent spacecraft subsystem modeling with physics-based cross-effects. CCSDS binary framing is well done. |
| **Testing & Verification** | 6.5 | Good unit tests for models/packets with proptest. Integration tests exist. Missing chaos, contract, and replay engine tests. |
| **Observability & SRE** | 7.0 | Prometheus metrics, Grafana dashboards, Jaeger tracing, structured logging. But no alerting rules tied to business SLOs. |
| **Deployment & Operations** | 7.5 | Multi-stage Dockerfiles, non-root execution, Docker Compose, health checks. Missing K8s manifests, HPA, PDB. |
| **Security** | 7.0 | Bearer auth, SASL/SSL Kafka, Docker secrets, `deny_unknown_fields`. API key in default values and rate limiter not confirmed. |
| **Documentation** | 8.0 | Excellent README, OpenAPI spec, architecture docs, runbooks, changelog. Missing ADRs and data flow diagrams. |
| **UI/UX** | 7.0 | Premium dark theme, 3D globe, SSE replay. Some state management issues and hardcoded URLs. |
| **CI/CD Pipeline** | 7.5 | Multi-stage pipeline with lint, test, container build, non-root verification, and e2e smoke test. Missing coverage gates. |

---

## Architecture Analysis

### Strengths

- **Event-driven decoupling** — The Kafka-based topology (raw → processed → health) creates clean boundaries. Each service can be scaled, deployed, and tested independently.
- **CCSDS binary protocol layer** — The [spaceops-packets](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/crates/spaceops-packets/src/lib.rs) crate demonstrates serious aerospace domain knowledge. CRC-16-CCITT, proper header packing, and APID-based subsystem routing are textbook CCSDS TM.
- **Subsystem cross-effects** — The [generator.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-simulator/src/generator.rs) models realistic cascading failures (thermal spike → CPU throttle → ADCS drift, battery depletion → payload shutdown).
- **Telemetry state merging** — The [processor.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-collector/src/consumer/processor.rs) merges sparse subsystem updates into a unified satellite view using a cache + DB fallback pattern.

### Weaknesses

- **Single Kafka partition per topic** — The `KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1` means zero fault tolerance and no parallelism.
- **No schema registry** — Kafka payloads are raw JSON strings. Schema evolution will break consumers silently.
- **Replay engine polls DB every 200ms** — The [stream_replay](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-replay-engine/src/main.rs#L460-L696) function queries the DB on every tick to check session state. At scale with 50+ concurrent replays, this becomes a Postgres bottleneck.
- **No backpressure mechanism** — The telemetry simulator fires HTTP POST requests in `tokio::spawn` fire-and-forget loops with no circuit breaker or backpressure signal.

---

## Categorized Improvements

---

### 🔴 HIGH Priority (13 items)

> Items that could cause data loss, outages, security incidents, or fundamentally limit the system's reliability.

---

#### H-1: Kafka Configuration — Single Partition / Zero Replication

**Files**: [docker-compose.yml](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/deployments/docker-compose/docker-compose.yml#L54)

The Kafka broker runs with `KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1` and no explicit topic partition configuration. In production, a single Kafka broker crash results in total data loss. Topics should have `--partitions 3 --replication-factor 3` at minimum, and consumers should be designed for partition-aware processing.

---

#### H-2: Default API Key Hardcoded in CLI Arguments

**Files**: [main.rs (collector)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-collector/src/main.rs#L47), [main.rs (replay)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-replay-engine/src/main.rs#L43)

```rust
#[arg(long, env = "API_KEY", default_value = "secret-spaceops-key-2026")]
api_key: String,
```

The default API key `secret-spaceops-key-2026` is in source control. Even for a demo project, this trains bad habits. The default should be empty or require explicit configuration, failing fast with a clear error if not set.

---

#### H-3: `std::sync::Mutex` in Async Context — Potential Deadlock

**Files**: [main.rs (simulator)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-simulator/src/main.rs#L162), [main.rs (health engine)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-health-engine/src/main.rs#L74)

Using `std::sync::Mutex` inside `tokio::spawn` blocks risks deadlocking the async runtime if a panic occurs while the lock is held (mutex poisoning). The simulator wraps `SatelliteSimulator` in `Arc<std::sync::Mutex<_>>` and locks it inside async tasks. Replace with `tokio::sync::Mutex` or redesign to use message-passing channels.

---

#### H-4: Missing Rate Limiter Implementation

**Files**: [server/mod.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-collector/src/server/mod.rs)

The README claims "Axis endpoints apply a strict token-bucket rate limiter," but no rate limiting middleware is visible in the Axum router configuration. The OpenAPI spec also specifies a `429` response code. This is a security gap — any client can saturate the ingestion pipeline.

---

#### H-5: Replay Engine — DB Polling on Every SSE Tick

**Files**: [main.rs (replay)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-replay-engine/src/main.rs#L460-L475)

Every 200ms, the SSE stream loop queries the database for session state:
```rust
let current_session = match db::get_replay_session(&state_clone.db_pool, session_id).await {
```
For N concurrent replays, this generates `5*N` queries/second against Postgres. This should use an in-memory session store (`DashMap` or `tokio::sync::watch`) with periodic DB persistence.

---

#### H-6: No Schema Versioning or Migration Runner

**Files**: [deployments/postgres/migrations/](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/deployments/postgres/migrations)

Database migrations are applied via raw `.sql` files piped through `psql` in bash scripts. There's no schema version tracking, rollback capability, or idempotency guarantee. In production, schema changes risk silent data corruption. Adopt a migration framework like `sqlx migrate` (which is already in the dependency tree), `refinery`, or `flyway`.

---

#### H-7: Consumer Offset Commit After `tokio::spawn` — Race Condition

**Files**: [main.rs (health engine)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-health-engine/src/main.rs#L386-L397)

```rust
tokio::spawn(async move {
    if let Err(e) = process_telemetry_health(&state_ref, &record).await { ... }
});
// Commit offset IMMEDIATELY after spawn, not after processing completes
if let Err(e) = consumer.commit_message(&msg, CommitMode::Async) { ... }
```

The offset is committed *before* the spawned processing task completes. If the task fails or the process crashes, the message is lost — violating the at-least-once delivery guarantee claimed in the README. The offset commit must happen inside the spawned task, or processing must be awaited before committing.

---

#### H-8: `GroundStationState` Struct Duplicated Across 3 Crates

**Files**: [generator.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-simulator/src/generator.rs#L15-L27), [health.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-health-engine/src/health.rs#L12-L24), [db.rs (ground station)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/ground-station-simulator/src/db.rs)

The `GroundStationState`/`GroundStationHealth` struct is defined identically in 3+ locations. Any field addition or type change must be synchronized across all definitions manually. This should be centralized in the `spaceops-models` crate alongside `TelemetryRecord`.

---

#### H-9: `load_secret()` Function Duplicated in Every Service

**Files**: All `main.rs` files across 5 services

The identical `load_secret()` function is copy-pasted across every service binary. This should be a utility function in a shared crate (e.g., `spaceops-models` or a new `spaceops-runtime` crate).

---

#### H-10: No Payload Size Limit Enforcement

**Files**: [server/mod.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-collector/src/server/mod.rs)

The README claims "Axis endpoints reject payloads exceeding 10KB," but no `DefaultBodyLimit` or content-length check is visible in the Axum router configuration. A large payload attack could exhaust memory.

---

#### H-11: Metrics Server Uses Raw TCP Socket — Not HTTP Compliant

**Files**: [main.rs (simulator)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-simulator/src/main.rs#L65-L108), [main.rs (health engine)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-health-engine/src/main.rs#L78-L121)

Multiple services implement a hand-rolled TCP metrics server that doesn't properly parse HTTP requests — it reads 1024 bytes and ignores the content. This is fragile and won't handle HTTP chunking, pipelining, or malformed requests. Use `axum` or `hyper` for metrics endpoints (as the replay engine already does).

---

#### H-12: Telemetry Cache Has Unbounded Memory Growth

**Files**: [processor.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-collector/src/consumer/processor.rs#L44)

```rust
cache: Mutex<HashMap<String, TelemetryRecord>>,
```

The in-memory telemetry cache grows unboundedly with each unique `satellite_id`. With `SIM_FLEET_SIZE=10`, this is fine. At 10,000+ satellites, this becomes a memory leak. Implement an LRU cache with TTL eviction.

---

#### H-13: No Graceful Shutdown for Most Services

**Files**: [main.rs (simulator)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-simulator/src/main.rs), [main.rs (health engine)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-health-engine/src/main.rs), [main.rs (ground station)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/ground-station-simulator/src/main.rs)

Only the collector and replay engine implement graceful shutdown (`tokio::select!`, `shutdown_signal`). The simulator, health engine, and ground station simulator all run infinite loops with no signal handling. On `SIGTERM` (Docker stop), in-flight Kafka messages and DB writes will be dropped.

---

### 🟡 MODERATE Priority (15 items)

> Items that affect developer productivity, system observability, or represent tech debt that will compound.

---

#### M-1: No Integration Tests for the Replay Engine

The replay engine has zero integration tests for its core SSE streaming, seek, speed change, and pause/resume logic. Only unit tests for speed validation and health mapping exist. This is a complex stateful system that needs end-to-end tests with a real database.

---

#### M-2: Missing Kafka Consumer Lag Metrics

None of the Kafka consumers export consumer lag metrics. In production, consumer lag is the single most important metric for detecting processing bottlenecks. Add `rdkafka`'s statistics callback or use `librdkafka`'s built-in `statistics.interval.ms`.

---

#### M-3: Health Assessment Uses Simple Threshold Scoring (0/50/100)

**Files**: [health.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-health-engine/src/health.rs#L82-L330)

Every subsystem scores exactly 0, 50, or 100. This is too coarse for meaningful fleet health trending. A linear interpolation score (e.g., battery at 11.0V = 25, battery at 12.0V = 75) would produce much more useful Grafana dashboards and alert rules.

---

#### M-4: UI Hardcodes Backend URLs

**Files**: [App.tsx](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-control-ui/src/App.tsx#L10-L11)

```typescript
const BACKEND_REPLAY_URL = 'http://localhost:8084';
```

Backend URLs are hardcoded to `localhost`. When deployed in Docker Compose, the UI container can't reach `localhost:8084`. These should be injected via Vite env variables (which `VITE_API_KEY` already demonstrates).

---

#### M-5: No Database Connection Pooling Tuning

All services use the same hardcoded pool sizes (`max_connections = 5-20`) with no connection idle timeout, min connections, or pool metrics. In production, connection pool exhaustion is a common failure mode. Add `min_connections`, `idle_timeout`, and expose pool metrics to Prometheus.

---

#### M-6: Anomaly Detection Has No Deduplication or Cooldown

**Files**: [processor.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-collector/src/consumer/processor.rs#L371-L471)

Every telemetry record that exceeds a threshold generates a new anomaly event row. For a satellite in a thermal spike scenario emitting 1 record/second, the events table accumulates 1 anomaly row/second indefinitely. Implement a cooldown period (e.g., "don't re-alert for the same anomaly type on the same satellite within 60 seconds").

---

#### M-7: No OpenTelemetry Tracing in Most Services

Only the telemetry collector initializes OpenTelemetry. The simulator, health engine, ground station simulator, and replay engine have no distributed tracing. Cross-service trace propagation through Kafka headers is also missing, making end-to-end request tracing impossible.

---

#### M-8: Kafka Config Boilerplate Duplicated Everywhere

Every service that touches Kafka has 15-20 lines of identical `ClientConfig` setup for SASL/SSL. Extract this into a shared `KafkaConfigBuilder` in a common crate.

---

#### M-9: Database Retry Logic Duplicated Across Services

The DB connection retry loop (5 retries, 3s sleep) is copy-pasted identically across the collector, health engine, ground station simulator, and replay engine. Extract to a shared utility.

---

#### M-10: No CI Test Coverage Gating

**Files**: [ci.yml](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/.github/workflows/ci.yml)

The CI pipeline runs tests but doesn't measure or gate on code coverage. Add `cargo-llvm-cov` or `tarpaulin` with a minimum threshold (e.g., 70%) to prevent coverage regression.

---

#### M-11: Frontend Has No Unit/Component Tests

The React/TypeScript UI has zero test files. No jest, vitest, or React Testing Library configuration. The `ReplayControls`, `OrbitViewer`, and `SatelliteDetail` components contain complex state logic that should be unit tested.

---

#### M-12: `HealthState` Enum Uses `SCREAMING_CASE` Variants

**Files**: [health.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-health-engine/src/health.rs#L37-L41)

```rust
pub enum HealthState {
    GREEN,
    YELLOW,
    RED,
}
```

Rust convention is `PascalCase` for enum variants (`Green`, `Yellow`, `Red`). The `#[serde(rename = "GREEN")]` attribute should be used to maintain wire compatibility.

---

#### M-13: UTC Clock Time Display in UI Doesn't Update

**Files**: [App.tsx](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-control-ui/src/App.tsx#L384)

```tsx
<span>UTC: {new Date().toISOString().replace('T', ' ').substring(0, 19)}</span>
```

This renders the time at mount and never updates. It should be in a `setInterval` with `useState` to tick every second.

---

#### M-14: No Prometheus Alert Rules Tied to Business SLOs

**Files**: [deployments/prometheus/](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/deployments/prometheus)

Prometheus is deployed with an alertmanager, but there's no evidence of alert rules for critical business metrics like:
- Telemetry ingestion rate drops to zero
- Consumer lag exceeds N seconds
- Fleet health score drops below threshold
- Ground station enters OUTAGE state

---

#### M-15: Replay Sessions Are Never Cleaned Up

**Files**: [db.rs (replay)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-replay-engine/src/db.rs)

Replay sessions are created in the `replay_sessions` table but never deleted. Over time, this table grows unboundedly. Implement a TTL-based cleanup job or a `STOPPED` → delete policy.

---

### 🟢 LOW Priority (12 items)

> Nice-to-haves, polish items, and minor improvements.

---

#### L-1: No Kubernetes Manifests

The project is Docker Compose-only. For production deployment at SpaceX scale, K8s manifests (Deployments, Services, HPA, PDB, NetworkPolicy) would be expected.

---

#### L-2: Zookeeper-Based Kafka

The Kafka deployment uses Zookeeper (deprecated). Migrate to KRaft mode for simpler operations.

---

#### L-3: Missing Architecture Decision Records (ADRs)

The docs directory has architecture and design documents but no ADRs explaining *why* specific technology choices were made (e.g., why Kafka over NATS, why TimescaleDB over InfluxDB).

---

#### L-4: `proptest` Used But Not Listed as Dev Dependency

**Files**: [Cargo.toml (spaceops-packets)](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/crates/spaceops-packets/Cargo.toml)

The `spaceops-packets` crate uses `proptest::proptest!` in tests but may need explicit `[dev-dependencies]` declaration.

---

#### L-5: No Data Retention Policy for TimescaleDB

No compression or retention policies are configured for the hypertables. For a production satellite fleet, telemetry data at 1 record/sec/subsystem/satellite would accumulate ~2.5GB/day for 50 satellites. Add `add_retention_policy` and `add_compression_policy`.

---

#### L-6: Load Test Doesn't Include Binary Payload Mode

**Files**: [load_test.js](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/benchmarks/load_test.js)

The k6 load test only exercises the JSON ingestion path. The binary CCSDS path (which is the primary production path) is not load tested.

---

#### L-7: No CHANGELOG Automation

The [CHANGELOG.md](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/CHANGELOG.md) is manually maintained. Consider `git-cliff` or `conventional-commits` for automated changelog generation.

---

#### L-8: Dockerfile Uses `rust:alpine` — Slow Builds

**Files**: All Dockerfiles

Alpine-based Rust builds use `musl` which is slower to compile and can cause subtle runtime differences with `glibc`. Consider `rust:bookworm` with a `debian:bookworm-slim` runner.

---

#### L-9: WebSocket Has No Authentication

**Files**: [handlers.rs](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/telemetry-collector/src/server/handlers.rs#L346-L375)

The `/ws` WebSocket endpoint has no authentication. Any client can subscribe to the real-time telemetry broadcast. Add token validation on the WebSocket upgrade request.

---

#### L-10: `security-audit` Job Uses `continue-on-error: true`

**Files**: [ci.yml](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/.github/workflows/ci.yml#L86)

The security audit and cargo-deny jobs have `continue-on-error: true`, meaning known vulnerabilities won't block merges. Remove this for the main branch.

---

#### L-11: No Browser/E2E Tests for the Mission Control UI

The e2e smoke test in CI tests the backend APIs but never validates that the React UI renders or that WebSocket/SSE connections work from the browser. Add Playwright or Cypress tests.

---

#### L-12: Three.js Globe Renders Fixed Satellite Positions

**Files**: [OrbitViewer.tsx](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/apps/mission-control-ui/src/components/OrbitViewer.tsx)

The 3D globe uses hardcoded orbital positions rather than computing positions from actual telemetry data (altitude, velocity, orbital elements). Connecting this to real telemetry would add significant visual fidelity.

---

## Summary

| Priority | Count | Key Themes |
|:---|:---:|:---|
| 🔴 **HIGH** | 13 | Data loss risks, security defaults, code duplication, at-least-once violation, unbounded resource growth |
| 🟡 **MODERATE** | 15 | Missing observability, coarse health scoring, frontend gaps, replay scalability, test coverage |
| 🟢 **LOW** | 12 | K8s, ADRs, data retention, polish items, build optimization |

The project demonstrates a strong foundation with genuine aerospace domain depth. Addressing the HIGH-priority items would elevate this from a "very good portfolio project" to a "production-candidate observability platform." The architecture is fundamentally sound — the issues are primarily in implementation details, hardening, and operational maturity.

> **Bottom line**: I'd want this engineer on my team. The architectural taste is right, the domain modeling is real, and the code is clean. The gaps are exactly the kind of things a Staff engineer would mentor through — they're not design flaws, they're shipping-maturity gaps.
