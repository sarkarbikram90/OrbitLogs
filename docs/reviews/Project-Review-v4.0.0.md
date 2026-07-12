# OrbitLogs — Staff Engineer Project Review

**Reviewer**: Staff Engineer, Starlink Ground Software  
**Date**: July 2, 2026  
**Repository**: [OrbitLogs](https://github.com/sarkarbikram90/OrbitLogs)  
**Language Stack**: Rust (backend), TypeScript/React (frontend), TimescaleDB, Kafka (KRaft), OpenTelemetry, Docker, Kubernetes  

---

## Overall Portfolio Rating: 8.2 / 10

> **Verdict**: This is an exceptionally ambitious and well-executed portfolio project that demonstrates genuine systems engineering depth — not just CRUD boilerplate. It models real satellite operations concepts (CCSDS framing, subsystem cross-coupling, ground station link budgets, CRC integrity) with a production-grade event-driven architecture. Few portfolio projects reach this level of domain sophistication and operational maturity. The areas preventing a 9+ are addressable and outlined below.

---

## I. Architecture & System Design

### Strengths

| Area | Assessment |
|---|---|
| **Event-Driven Decoupling** | Kafka-backed topics (`telemetry.raw`, `telemetry.processed`, `mission.health`, `timeline.events`, `command.events`) provide clean service boundaries. Each microservice has a single responsibility. This is textbook event sourcing done right. |
| **Workspace Monorepo** | Shared crates (`orbitlogs-models`, `orbitlogs-packets`, `fleet-health`, `space-schema-registry`, `space-otel-collector`) prevent model drift across 12 apps. The `resolver = "2"` Cargo workspace with unified dependency versions is a mature pattern. |
| **Schema Versioning** | The `space-schema-registry` crate implements forward/backward compatibility checking with V1→V2→V3 migration paths. This is exactly what production spacecraft ground systems need for long-lived missions where payload formats evolve. |
| **Binary Protocol Layer** | The `orbitlogs-packets` crate implements CCSDS-inspired TM frame and space packet structures with proper bit-level packing/unpacking of primary headers, secondary headers, APID routing, and CRC-16-CCITT integrity. This is not toy code — it demonstrates genuine understanding of space protocol stacks. |
| **Subsystem Cross-Coupling** | The telemetry simulator models realistic physics cascades: thermal spikes → CPU throttling → ADCS drift → payload shutdown. This is how real satellite failure modes propagate and it's rare to see this modeled in portfolio projects. |
| **Observability Stack** | Full W3C trace context propagation across Kafka boundaries via `TraceContextPropagator`, OTLP export with `BatchSpanProcessor`, Prometheus metrics, Grafana dashboards, Jaeger tracing. This is production-grade OTel integration. |
| **Database Design** | TimescaleDB hypertables with 1-day chunking, segment-by compression on `satellite_id`, 7-day compression policies, 90-day retention policies. This demonstrates understanding of time-series data lifecycle management. |

### Weaknesses

| Area | Issue | Impact |
|---|---|---|
| **God Files** | Several services are single-file monoliths. `mission-replay-engine/main.rs` is **1,000+ lines**, `orbit-ops-service/main.rs` is **874 lines**, `ai-incident-commander/main.rs` is **613 lines**. These need decomposition into handler, service, and repository layers. | Maintainability: **Medium** |
| **Duplicated Middleware** | `auth_middleware`, `cors_middleware`, and `shutdown_signal` are copy-pasted identically across `digital-twin-engine`, `ai-incident-commander`, `mission-command-service`, and others. This is a DRY violation that should be extracted into the shared `orbitlogs-models` crate. | Code Duplication: **High** |
| **CORS Bug** | In `ai-incident-commander/main.rs:131`, the OPTIONS preflight response uses `"access-control-origin"` instead of `"access-control-allow-origin"`. This will cause CORS preflight failures for the AI chat endpoint in browsers. | Functional Bug: **High** |
| **No Service Mesh / Health Probes** | While individual services have `/health` endpoints, there's no standardized health check contract. The Kubernetes manifests exist but don't appear to define liveness/readiness probes consistently. | Operational: **Medium** |
| **Hardcoded Satellite ID Extraction** | In `ai-incident-commander`, satellite IDs are extracted via hardcoded string matching (`if query.contains("sat-0001")`) for SAT-0001 through SAT-0010. This doesn't scale and should use regex or a proper NER approach. | Scalability: **Low** |

---

## II. Security Audit

### What's Done Well ✅

| Control | Implementation | Assessment |
|---|---|---|
| **Constant-Time Auth** | `secure_compare()` in `auth.rs` uses `sha2::Sha256` + `subtle::ConstantTimeEq` to prevent timing side-channels. Hash-then-compare also prevents length leakage. | **Excellent** |
| **Serde Strict Mode** | `#[serde(deny_unknown_fields)]` on `TelemetryRecord` prevents injection of unmapped attributes via JSON payloads. | **Good** |
| **Input Validation** | `TelemetryRecord::validate()` enforces physical bounds on all numeric fields (voltage 0-100, CPU 0-100, orientation ±360°, future timestamp rejection). | **Good** |
| **Payload Size Limits** | `DefaultBodyLimit` on Axum endpoints caps incoming payloads at 10KB. | **Good** |
| **Rate Limiting** | Token-bucket rate limiter on login endpoints with per-IP partitioning. | **Good** |
| **LLM Guardrails** | AI Incident Commander implements both input sanitization (prompt injection, SQL injection pattern detection) and output sanitization (credential leakage, system instruction leakage). | **Above Average** |
| **Non-Root Containers** | Dockerfiles run services as `orbitlogs` user (UID 10001). CI verifies this with `whoami` checks. | **Good** |
| **Docker Secrets** | Sensitive values (DB password, API key, Kafka SASL) loaded via Docker secrets file mount pattern (`_FILE` suffix convention). | **Good** |
| **Dependency Auditing** | `deny.toml` + `rustsec/audit-check` + `cargo-deny-action` in CI pipeline. Unknown registries and git sources are denied. | **Good** |

### Security Concerns ⚠️

| Issue | Severity | Location | Details |
|---|---|---|---|
| **Hardcoded Default Credentials** | 🔴 **Critical** | `Cargo.toml` default args, `ci.yml` | `postgres://postgres:postgrespassword@localhost:5432/orbitlogs` appears as default values across CLI args in every service. While `.env.example` says `CHANGE_ME_REQUIRED`, the compiled binary defaults will work with `postgrespassword`. In production, default args should be empty strings or the binary should refuse to start. |
| **JWT Secret Management** | 🟡 **Medium** | `mission-command-service/main.rs` | The JWT signing secret is loaded from env/Docker secret, but there's no key rotation mechanism. JWT tokens have a 24-hour expiry, which is reasonable, but a compromised key requires redeployment. Consider asymmetric keys (RS256). |
| **SQL Injection Guardrails are Superficial** | 🟡 **Medium** | `ai-incident-commander` | The `sanitize_chat_input()` function uses substring matching for SQL patterns (`"union select"`, `"drop table"`, `"--"`). This is easily bypassed with encoding, case alternation (`UnIoN SeLeCt`), or comment injection. All DB queries use parameterized queries via `sqlx::query().bind()`, which is the actual defense — the string matching is theater. The `"--"` pattern will false-positive on legitimate queries containing dashes. |
| **API Key Single-Factor** | 🟡 **Medium** | All services | The entire authentication model relies on a single shared API key compared via Bearer token. There's no per-user identity, no token revocation, and no audit trail of which operator performed which action (beyond JWT roles in `mission-command-service`). |
| **Gemini API Key in Query String** | 🟡 **Medium** | `ai-incident-commander` | The Gemini API key is appended to the URL as a query parameter (`?key={}`). This means the key will appear in HTTP access logs, proxy logs, and potentially browser history. Use request headers instead. |
| **No TLS Enforcement** | 🟢 **Low** | All services | Services bind to `0.0.0.0:PORT` over plain HTTP. In production, TLS termination is handled at the load balancer/ingress level, but the codebase has no option for direct TLS even for internal mTLS. |

---

## III. Test Coverage Analysis

### Test Inventory

| Component | Test Module | Test Count | Coverage Area |
|---|---|---|---|
| `orbitlogs-models` | `lib.rs` | 11 | Validation, serde roundtrip, deny unknown fields, edge cases |
| `orbitlogs-packets` | `lib.rs` | 2 + **proptest** | Encode/decode roundtrip, CRC corruption detection, **property-based testing across all subsystem types** |
| `orbitlogs-packets` | `crc.rs` | 1 | Standard CRC-16-CCITT vector |
| `space-schema-registry` | `lib.rs` | 2 | V1 validation, compatibility + migration |
| `telemetry-simulator` | `generator.rs` | 8 | Deterministic RNG, all scenarios, cross-subsystem side effects, ground station modulation |
| `telemetry-collector` | `integration_tests.rs` | 3+ | HTTP handler integration tests |
| `mission-health-engine` | `health.rs` | 6+ | Health scoring rules |
| `mission-replay-engine` | `main.rs` | Tests present | Replay logic |
| `digital-twin-engine` | `main.rs` | 6 | Twin health projection for all subsystem states |
| `mission-command-service` | `main.rs` | Tests present | Command lifecycle |
| `orbit-ops-service` | `director.rs`, `health.rs`, `twin.rs` | 16+ | Director decisions, health scoring, twin state |
| **Frontend** | `e2e/visual-canvas.spec.ts` | 1 | Playwright visual canvas test |

### Test Coverage Verdict

**Strengths**:
- **Property-based testing** with `proptest` on the binary packet layer is outstanding. This generates thousands of randomized telemetry records across all 8 subsystem types and verifies encode/decode roundtrip fidelity. This is the kind of testing that catches edge cases humans miss.
- Cross-subsystem physics side-effect tests (thermal → CPU throttling, battery → payload shutdown, fuel → ADCS drift) are well-designed.
- Integration tests in `telemetry-collector` test the HTTP→Kafka pipeline end-to-end.
- CI runs the full E2E smoke test: Docker Compose → send telemetry → verify DB propagation.

**Gaps**:

| Missing Test | Priority | Rationale |
|---|---|---|
| **Auth/RBAC unit tests** | 🔴 High | `check_role()`, `login_handler()`, JWT validation, rate limiter behavior — none of these have unit tests. This is the security perimeter. |
| **AI Incident Commander tests** | 🔴 High | `sanitize_chat_input()`, `sanitize_chat_output()`, `generate_rule_response()` — zero tests for the guardrail functions or rule-based response engine. |
| **Negative path / error handling** | 🟡 Medium | No tests for malformed Kafka messages, database connection failures, timeout scenarios, or invalid state transitions in the command lifecycle. |
| **Fleet Health Engine tests** | 🟡 Medium | `fleet-health` crate has zero tests. `FleetHealthEngine::calculate()` handles empty arrays, weighted scoring, and utilization metrics — all untested. |
| **Frontend unit tests** | 🟡 Medium | Only 1 Playwright e2e spec. No component-level tests (React Testing Library/Vitest) for the mission control UI. |
| **Constellation Simulator tests** | 🟢 Low | `constellation-simulator` has no tests. At 5000 simulated satellites, edge cases around concurrent state and memory pressure should be validated. |
| **Load test integration** | 🟢 Low | The k6 load test exists but isn't run in CI. It should be part of a nightly or release pipeline. |

---

## IV. Code Quality

### What's Excellent

- **Zero `unsafe` blocks** across the entire codebase. Rust's memory safety guarantees are fully leveraged.
- **Consistent error handling pattern**: `anyhow::Error` for binary entrypoints, `(StatusCode, Json<Value>)` tuples for HTTP handlers. This is idiomatic Axum.
- **Deterministic simulation**: `ChaCha8Rng` with satellite-specific seed derivation ensures reproducible test runs. This is exactly how SpaceX's simulation infrastructure works.
- **Graceful shutdown**: Every service implements `CancellationToken` + `shutdown_signal()` with SIGTERM/SIGINT handling. Kafka consumers flush before exit.
- **Database retry logic**: Exponential backoff with jitter (1s, 2s, 4s... + 0-500ms random) in `establish_db_connection()` handles Docker Compose boot-order races elegantly.
- **Release profile optimization**: `lto = true`, `codegen-units = 1`, `opt-level = 3`, `panic = "abort"` — this is the correct production Rust release profile.

### Issues

| Category | Issue | Location |
|---|---|---|
| **Shadowed `to_string`** | `HealthState` implements both a manual `to_string()` method and `Display` trait. The manual method shadows the auto-derived `ToString` and will cause confusion. Use `Display` only. | `orbitlogs-models/lib.rs:221` |
| **Stringly-typed enums** | Subsystem status uses raw strings `"NOMINAL"`, `"DEGRADED"`, `"FAILED"` throughout the digital twin engine. These should be proper enums with `serde` derive. | `digital-twin-engine/main.rs` |
| **Magic numbers** | Health projection thresholds (11.5V, 10.5V, 75°C, 90°C, 10° pitch, 20% fuel) are hardcoded inline. These should be configuration constants or externalized into a rules configuration file. | `digital-twin-engine/main.rs:394-469` |
| **`block_in_place` in async context** | `telemetry-simulator/generator.rs:175` uses `tokio::task::block_in_place()` to call an async function from a sync context. This can stall the Tokio runtime if the HTTP call hangs. Use `tokio::spawn` with a channel instead. | `generator.rs:174-188` |
| **`HealthState::to_string` shadowing** | `HealthState` already derives `Display` via the manual `Display` impl but ALSO has an inherent `to_string()` method. The inherent method takes priority over `Display::to_string()` creating silent correctness risk. | `lib.rs:221-228` |
| **Inconsistent error logging** | Some handlers swallow errors silently (`unwrap_or_default()` in `get_recent_events`) while others properly propagate. The AI commander's DB query errors in `get_recent_events` and `get_recent_commands` silently return empty arrays, making debugging production issues very difficult. | `ai-incident-commander/main.rs:430,447` |

---

## V. Operational Maturity

### What's Production-Ready ✅

- **11 sequential database migrations** with idempotent `IF NOT EXISTS` / `if_not_exists => TRUE` patterns
- **TimescaleDB compression + retention** policies (7-day compress, 90-day retention)
- **KRaft mode Kafka** (no Zookeeper dependency)
- **OpenAPI 3.0.3 specification** for the telemetry and replay APIs
- **k6 load testing** script with `constant-arrival-rate` executor and p95 latency thresholds
- **Prometheus metrics** with `process` feature for runtime stats
- **Docker Compose** with full infrastructure stack (TimescaleDB, Kafka, OTel Collector, Prometheus, Grafana, Jaeger)
- **Kubernetes manifests** with Kustomize overlay structure
- **CI/CD pipeline** with 5 jobs: formatting/linting/testing, security audit, container builds, frontend CI, E2E smoke test

### Operational Gaps

| Gap | Impact |
|---|---|
| **No structured error codes** | HTTP error responses use free-text `"error"` strings. Operators and automated tooling need machine-readable error codes (e.g., `ERR_COMMAND_INVALID_STATE`, `ERR_AUTH_TOKEN_EXPIRED`). |
| **No circuit breaker** | If TimescaleDB goes down, all services will retry forever with exponential backoff. There's no circuit breaker pattern to fast-fail and shed load. |
| **No health aggregation** | Each service exposes its own `/health`, but there's no single pane of glass showing aggregate system health. |
| **No configuration validation** | Services start with invalid configurations and fail at runtime (e.g., missing API key panics at first request). Configuration should be validated at startup with clear error messages. |
| **Replay Engine write throttling** | The README claims 5-second throttling with DashMap caching, but I'd want to see actual metrics proving the 96% write reduction claim. |

---

## VI. Recommended Improvements (Priority Ordered)

### 🔴 Critical (Do First)

1. **Fix the CORS bug** in `ai-incident-commander` OPTIONS handler (`"access-control-origin"` → `"access-control-allow-origin"`)
2. **Remove hardcoded default credentials** from CLI arg defaults. Use empty strings and fail fast with a clear error message if not configured.
3. **Add auth/RBAC unit tests** — test JWT validation, role hierarchy, token expiration, rate limiting, and invalid tokens.
4. **Add AI guardrail tests** — test `sanitize_chat_input` with bypass attempts and `generate_rule_response` with various telemetry states.

### 🟡 Important (Do Next)

5. **Extract shared middleware** — Move `auth_middleware`, `cors_middleware`, `shutdown_signal` into `orbitlogs-models` as reusable Axum layers.
6. **Add `fleet-health` crate tests** — The engine has no tests and is a core business logic component.
7. **Break up god files** — `mission-replay-engine/main.rs` (1000+ lines) and `orbit-ops-service/main.rs` (874 lines) need modularization into handler/service/repo layers.
8. **Replace stringly-typed status fields** with enums — `SubsystemState.status`, command lifecycle states, ground station states should all be Rust enums.
9. **Externalize health thresholds** — Move magic numbers into a TOML/YAML configuration file or environment variables.
10. **Add structured error codes** across all HTTP endpoints for machine-readable error handling.

### 🟢 Nice-to-Have (Polish)

11. **Add `cargo-tarpaulin` or `llvm-cov`** to CI to track and enforce minimum code coverage thresholds.
12. **Add Playwright component tests** for the Mission Control UI beyond the single visual canvas spec.
13. **Integrate k6 load tests** into the CI pipeline as a nightly job.
14. **Add `cargo-fuzz`** targets for the binary packet parser — this is a security-sensitive parsing boundary.
15. **Document the Kafka topic schema contracts** — Currently undocumented. Consider adding Avro/Protobuf schema definitions.
16. **Add request tracing IDs** to all error responses for correlation in Jaeger.
17. **Consider async-graphql** for the constellation/fleet query layer instead of REST — the nested satellite→subsystem→telemetry query pattern is a natural fit for GraphQL.

---

## VII. Domain Accuracy Assessment

As someone working on Starlink ground infrastructure, I want to call out the domain modeling quality:

| Domain Concept | Implementation | Accuracy |
|---|---|---|
| **CCSDS Space Packet** | Primary header (version, type, APID, sequence count), secondary header (coarse/fine timestamps), CRC-16-CCITT error control | ✅ Faithful to CCSDS 133.0-B-2 |
| **Transfer Frame** | Frame header with spacecraft ID, virtual channel, frame counters, error control | ✅ Correct structure |
| **Subsystem Cross-Coupling** | Thermal→CPU throttle, battery→payload shed, fuel leak→ADCS drift, battery→comms degrade | ✅ Realistic failure cascading |
| **Ground Station Link Budget** | Weather attenuation, antenna health, backhaul congestion, pass scheduling | ✅ Good approximation |
| **Orbital Mechanics** | ISS TLE propagation via `sgp4` crate, altitude-dependent velocity calculations | ✅ Physically reasonable |
| **Command Lifecycle** | CREATED→APPROVED→QUEUED→TRANSMITTED→RECEIVED→VERIFIED/FAILED with RBAC gates | ✅ Matches standard ground segment command flows |

The domain modeling is **significantly above average** for a portfolio project. Most satellite-themed projects treat the spacecraft as a black box. OrbitLogs models subsystem interdependencies, ground segment link physics, and operational command flows with genuine aerospace engineering understanding.

---

## VIII. Scoring Breakdown

| Category | Weight | Score | Weighted |
|---|---|---|---|
| Architecture & Design | 25% | 8.5 | 2.13 |
| Code Quality & Idioms | 20% | 8.0 | 1.60 |
| Security & Safety | 15% | 7.5 | 1.13 |
| Test Coverage | 15% | 7.0 | 1.05 |
| Operational Maturity | 10% | 8.5 | 0.85 |
| Domain Depth & Originality | 10% | 9.5 | 0.95 |
| Documentation | 5% | 9.0 | 0.45 |
| **Total** | **100%** | | **8.16 ≈ 8.2** |

---

## IX. Final Assessment

OrbitLogs is one of the stronger systems engineering portfolio projects I've reviewed. It demonstrates:

1. **Genuine domain expertise** — not surface-level space theming but actual CCSDS protocol implementation, subsystem cross-coupling physics, and operational command workflows.
2. **Production engineering discipline** — graceful shutdown, exponential backoff with jitter, TimescaleDB compression/retention, constant-time auth, non-root containers, dependency auditing.
3. **Architectural maturity** — clean event-driven decoupling, shared domain crates, schema versioning with migration paths, full observability stack.

The main areas for improvement are code organization (god files, duplicated middleware), test coverage gaps (auth, AI guardrails, fleet health), and tightening the security posture (hardcoded defaults, JWT rotation, structured error codes).

**For a Starlink/SpaceX engineering portfolio review, this project would absolutely earn an interview.** The candidate clearly understands not just how to write Rust, but how to build systems that operate under real-world constraints — which is ultimately what we need at Starlink.

---

*Review completed by Staff Engineer, Starlink Ground Software Division*
