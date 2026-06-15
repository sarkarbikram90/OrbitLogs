# Changelog 📖

All notable changes to the **OrbitLogs** platform will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [2.2.0] - 2026-06-15

### Added
- **SpaceX Flight Operations Login Screen**: Created a high-fidelity access control credentials modal in [App.tsx](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/mission-control-ui/src/App.tsx) to dynamically request the console passkey and operator role context without baking credentials into compiled client-side assets.
- **Dynamic JWT Authentication Rotation**: Configured background JWT key rotation in [CommandCenter.tsx](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/mission-control-ui/src/components/CommandCenter.tsx) to dispatch token signing handshakes automatically when operators transition roles.
- **Console Disconnect Controller**: Added a "Disconnect" control button to the UI header to clear `sessionStorage` and reload access state.
- **Containerization Support**: Created 5 new Dockerfiles for `mission-command-service`, `digital-twin-engine`, `ai-incident-commander`, `constellation-simulator`, and `mission-timeline` matching production requirements.
- **Command Service Unit Tests**: Added unit tests inside [apps/mission-command-service/src/main.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/mission-command-service/src/main.rs) validating constant-time comparison, role parsing, and JWT signature verification.

### Changed
- **RBAC Cryptographic Validation**: Replaced plaintext `X-User-Role` headers with securely signed JSON Web Tokens (JWT) verifying permissions in the `mission-command-service`.
- **Bearer Token Security Headers**: Configured state-modifying POST requests across `CommandCenter`, `DigitalTwinViewer`, and `AiCopilot` to authorize requests using Bearer headers.
- **15-Member Docker Caching**: Updated all 10 Rust service Dockerfiles to copy all 15 workspace member `Cargo.toml` files and pre-compile dummy sources, caching cargo dependencies effectively.
- **CORS Restriction**: Configured CORS origin restrictions on `mission-command-service` locked to the `CORS_ALLOWED_ORIGIN` environment variable.
- **Graceful Shutdowns**: Implemented SIGINT/SIGTERM handlers for Axum servers and consumer loops across all 10 backend services to ensure clean process exits.

### Fixed
- **TimescaleDB Schema Alignment**: Synced telemetry schema columns (`DROP NOT NULL`) to align with Rust `Option<>` structures, added payload metrics columns, and synced initial migrations.
- **Compiler Warnings & Errors**: Fixed loop break types and async return parameters in `mission-timeline` and `constellation-simulator` compilation scopes.

## [2.1.0] - 2026-06-15

### Added
- **Staff Engineer Production Readiness Review**: Conducted a comprehensive end-to-end codebase review evaluating all 16 workspace members across 10 engineering dimensions (Architecture, Code Quality, Security, Testing, Observability, Data Layer, Deployment, Frontend, Documentation, Production Hardening). Published as `SpaceX-Staff-Engineer-Review.md`.
- **Per-Service Scorecard**: Documented individual production readiness scores for all 11 apps and 5 shared crates with detailed findings per component.
- **Prioritized Action Plan**: Created a 28-item tiered remediation roadmap (P0/P1/P2) covering security hardening, test coverage expansion, deployment gaps, and engineering excellence targets.

### Identified — Security (P0 Blockers)
- Frontend API key hardcoded in client-side JavaScript (`App.tsx`).
- CORS wildcard (`allow_origin(Any)`) on the Mission Command Service.
- RBAC implemented via self-declared `X-User-Role` headers with no JWT/OIDC validation.
- No authentication middleware on command lifecycle endpoints.
- Kafka SASL/TLS disabled by default; all inter-service traffic is plaintext.

### Identified — Infrastructure Gaps
- 5 of 11 services (`mission-command-service`, `digital-twin-engine`, `ai-incident-commander`, `constellation-simulator`, `mission-timeline`) have no Dockerfile and cannot be deployed.
- Existing Dockerfiles are stale and reference only the original 5 workspace members.
- Schema mismatch between `NOT NULL` database columns and `Option<>` Rust model fields.
- No migration tooling — raw SQL files applied via bash scripts with no rollback capability.

### Identified — Testing Gaps
- 8 of 11 services have zero test coverage.
- No frontend tests (no Jest, React Testing Library, or Playwright).
- No contract tests, chaos tests, or code coverage enforcement.

---

## [2.0.0] - 2026-06-15

### Added
- **Space Schema Registry**: Added schema validation and compatibility checks for telemetry format version transitions (V1, V2, V3).
- **Fleet Health Engine**: Implemented fleet-wide calculation metrics covering readiness, availability, risk score, capacity, and utilization.
- **Mission Command Service**: Created an Axum-based service with role-based access control (RBAC) to model command creation, approval, and execution states.
- **Digital Twin Engine**: Built virtual representation states that listen to telemetry and allow failure simulation injections.
- **Mission Timeline**: Consolidated events and logs querying in TimescaleDB using hypertables.
- **AI Incident Commander**: Integrated LLM prompt builders and rule-based diagnostic fallbacks to support operator remediation.
- **Constellation Simulator**: Simulates high-throughput state loops for up to 5,000 satellites.
- **Simulators Enhancements**: Enhanced simulators with ISS TLE propagation orbits and Ground Station backhaul weather faults.
- **Advanced Frontend Console**: Designed a new sidebar navigation structure with specific sections for Command Center (RBAC controls), Mission Timeline search, Digital Twin subsystems viewer, and AI Copilot.
- **Satellite Orbit Visualization**: Generated and embedded a realistic cybernetic satellite orbit graphic in `README.md` to represent active space operations.

### Changed
- **Branding**: Rebranded the entire project, docs, configuration, and UI consoles to **OrbitLogs**.
- **Centralized Telemetry**: Consolidated tracing and OpenTelemetry initialization code into the shared `spaceops-models` crate.
- **Documentation Overhaul**: Significantly expanded `README.md` with deep operational specifications for all microservices, port mappings, and roadmap progress.

### Fixed
- **Tokio Runtime Context in Tests**: Resolved tokio reactor panic warnings in simulator unit tests using graceful `Handle::try_current()` runtime fallbacks.
- **FutureRecord Lifetimes**: Fixed `'static` loop variable reference borrows in constellation simulator threads.

### Removed
- **Contributing Guide**: Removed outdated `CONTRIBUTING.md` from the repository.

## [1.1.0] - 2026-06-07

### Added
- **Trace Context Propagation**: Injected W3C trace context into Kafka `RecordHeaders` for end-to-end distributed tracing across service boundaries.
- **Tenant-Isolated Rate Limiting**: Implemented client-isolated token buckets to replace global rate limiters, preventing cross-tenant starvation.

### Changed
- **Architectural Centralization**: Consolidated `establish_db_connection` and `KafkaConfigBuilder` into the `spaceops-models` shared crate.
- **Replay DB Throttling**: Replaced 200ms per-tick database updates with an in-memory `dashmap::DashMap` session cache, throttling persistent writes to every 5 seconds.
- **Kafka KRaft Migration**: Eliminated Zookeeper dependency and migrated local infrastructure to native Kafka KRaft mode.
- **Telemetry Cache Eviction**: Replaced unbounded consumer cache with an eviction-backed LRU cache capped at 10,000 concurrent entries.
- **Offset Commit Safety**: Added double-failure panic exits to avoid silent offset commits when both DB and DLQ inserts fail.
- **Code Formatter Compliance**: Applied `cargo fmt --all` across the entire workspace to align with strict automated CI gates.

### Fixed
- **Timing Side-Channel Vulnerability**: Replaced standard string comparisons with `subtle::ConstantTimeEq` for secure bearer token and WebSocket authorization checks.
- **Clippy Warnings**: Resolved all codebase clippy warnings (e.g., collapsing nested `if` statements) to pass strict `-D warnings` gates.
- **Workspace Docker Builds**: Fixed Dockerfile dependency resolution for `ground-station-simulator` to correctly copy `spaceops-models` sources during multi-stage builds.

## [1.0.1] - 2026-06-05

### Added
- **API Authentication**: Implemented Bearer token and query parameter authentication middleware for telemetry ingestion and replay endpoints.
- **HTTP Integration Tests**: Added 11 new integration tests covering auth, CORS, rate limits, and binary codec validation.
- **CORS Restriction**: Configured CORS middleware locked to deployment origins.
- **Kafka Security**: Added SASL/TLS configuration hooks on all Kafka client builders.
- **Kafka Circuit Breaker**: Implemented a custom 3-state circuit breaker with a 10s recovery timeout to prevent thread exhaustion during broker outages.
- **Frontend CI/CD**: Added ESLint and TypeScript build verification to GitHub Actions pipeline.

### Changed
- **Async Mutexes**: Replaced blocking `std::sync::Mutex` with `tokio::sync::Mutex` in the Consumer Cache to prevent Tokio executor starvation.

### Fixed
- **OrbitViewer Performance**: Refactored Three.js scene initialization to run only once, updating live data via `useRef` to eliminate GPU memory leaks and frame drops.

## [1.0.0] - 2026-06-04

### Added
- **Rate Limiting Layer**: Implemented endpoint rate-limiting (100 req/sec) using Tower middleware with an in-memory buffer on the `/telemetry` ingestion endpoint, returning `429 Too Many Requests` on overflow.
- **Readiness and Liveness Probes**: Added `/readiness` (database and Kafka verification) and `/liveness` endpoints to the ingestion collector.
- **Structured Error Responses**: Replaced generic text errors with detailed JSON response payloads mapping exact validation constraints.
- **DLQ Failure Metric**: Added `dlq_publish_failures_total` Prometheus counter and integrated alerting rules.
- **Vulnerability CI Scans**: Integrated automated vulnerability auditing (`cargo-audit`) and license compliance validation (`cargo-deny`) into the CI pipeline.
- **Integration Test Suite**: Created a fully automated mock integration test suite validating HTTP routing, size rejections, rate limiting, and schema checking.

### Changed
- **At-Least-Once Delivery**: Shifted from Kafka auto-commit to manual offset commits. Offsets are committed asynchronously only after successful TimescaleDB write or DLQ fallback enqueue.
- **Non-Root Containers**: Updated container images (`Dockerfile`) to create a dedicated `spaceops` user and run with non-privileged system rights (UID/GID 10001).
- **Hardcoded Credentials Removal**: Refactored Grafana provisions, docker-compose, and collector configurations to retrieve database secrets dynamically from environment variables.
- **Shared Models Extraction**: Extracted duplicate `TelemetryRecord` structs from the collector and simulator into a dedicated, reusable workspace library `spaceops-models`.
- **Serde Strict Parsing**: Configured `#[serde(deny_unknown_fields)]` on shared telemetry structs to block unexpected JSON key schemas.
- **Telemetry Latency Buckets**: Tailored custom Prometheus histogram sub-10ms buckets optimized for high-resolution satellite updates.

### Fixed
- **Type Ambiguity**: Resolved compiler float resolution ambiguity in the telemetry simulator's orbital generator.
- **Lints & Warnings**: Removed all unused imports, unused functions, and clippy warnings across all workspace members.
- **Git Repo Hygiene**: Untracked build-output directories (`target/`) from the repository index.
