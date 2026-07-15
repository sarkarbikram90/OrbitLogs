# Changelog 📖

All notable changes to the **OrbitLogs** platform are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> This changelog is generated from the full git commit history. Every meaningful commit is captured, including feature additions, refactors, infrastructure changes, documentation, bug fixes, CI improvements, and security hardening.

---

## [4.5.0] - 2026-07-04

### Added
- **Space Traffic Management (STM) & Conjunction Assessment (CA)**: Implemented full end-to-end Space Traffic Management capabilities across the backend and frontend microservices.
- **Conjunction (Collision) Alerts**: Defined canonical `ConjunctionEvent` and `AvoidanceManeuverRecommendation` models alongside `conjunction.alerts` and `conjunction.maneuvers` Kafka topics in `orbitlogs-models` (`lib.rs`, `kafka.rs`). Created database migration `20260704000000_space_traffic_management.sql` for TimescaleDB indexing.
- **Orbital Debris 3D Visualization**: Integrated a 250-particle orbital debris cloud into the 3D Three.js Earth globe visualizer in `mission-control-ui` (`OrbitViewer.tsx`).
- **Debris Risk Scoring & Close Approach Screener**: Extended `orbit-ops-service` (`ssa.rs`, `main.rs`) with background close approach evaluation computing Time of Closest Approach (TCA), spatial covariance, and Probability of Collision ($P_c$). Exposed `/api/ssa/conjunctions` and `/api/ssa/debris` API endpoints.
- **Autonomous Collision Recommendations & Mission Impact**: Added `CONJUNCTION_RUNBOOK` (DOC-SSA-201) to `ai-incident-commander` (`runbooks.rs`) to evaluate collision risk and auto-generate optimal Delta-V ($\Delta v$) maneuver burn vectors, propellant mass cost, payload standby windows, and post-burn miss distances.
- **Space Traffic Management Dashboard**: Upgraded `SsaDashboard.tsx` in `mission-control-ui` with a Space Risk Scorecard, live Conjunction Alert log, and an interactive 1-click **Authorize Collision Avoidance Burn** action panel.

### Changed
- **Telemetry Query in AI Incident Commander**: Updated `get_recent_events` query in `ai-incident-commander` (`db.rs`) to explicitly select telemetry fields (`satellite_id`, `timestamp`, `latitude`, `longitude`, `altitude`, `velocity`, `payload`) from `telemetry` table.

---

## [Unreleased]

### Added
- **orbit-ops-service Core Logic Unit Tests**: Implemented comprehensive unit test suites for `DirectorManager` (`director.rs`), `HealthScoringManager` (`health.rs`), and `TwinManager` (`twin.rs`) in `orbit-ops-service` to cover telemetry evaluation rules, composite health scoring algorithms, command application, and telemetry failure cascades.

### Changed
- **Folder De-duplication**: Consolidated database migration files into the root `migrations/` directory and deleted the duplicate migrations directory under `deployments/postgres/migrations/`. Pointed TimescaleDB docker-compose mounts and CI script workflows to the single unified root directory.
- **Service Relocation**: Moved the standalone OpenTelemetry metrics collector service (`space-otel-collector`) from `crates/space-otel-collector` (which is reserved for shared libraries) to `apps/space-otel-collector` (where standalone binary services reside) to adhere to standard monorepo packaging conventions.
- **Workspace Dependency Management**: Centralized third-party dependency version definitions (e.g. `axum`, `proptest`, `criterion`, `tower`, `tower-http`, `lru`) in the root `Cargo.toml` workspace block, enabling sub-crates to inherit versions via `{ workspace = true }`.
- **Warning & Dead Code Elimination**: Resolved all compiler warnings across the codebase, including macro-generated unused functions in `orbitlogs-models`, unused imports in `orbit-ops-service`, and unused enum variants.
- **Workspace Root Declutter**: Cleaned up the repository root by archiving and moving outdated assessment, evaluation, and review markdown files into `docs/reviews/` and `docs/archive/`.
- **evaluate_telemetry Signature Formatting**: Formatted the `evaluate_telemetry` function signature in `orbit-ops-service` (`director.rs`) across multiple lines to improve code readability.
- **Capitalize Error Messages**: Capitalized the panic and error messages in the shutdown handlers within `mission-command-service` (`main.rs`).
- **Ground Station Metric Descriptions**: Clarified Prometheus metric descriptions for antenna health and latency in `ground-station-simulator` (`main.rs`).
- **Guardrail & Anomaly Log Wording in Incident Commander**: Updated LLM input guardrail warning logs and rule response phrasing for critical anomalies in `ai-incident-commander` (`main.rs`).
- **Telemetry Packet & Frame Error Phrasing**: Improved error message phrasing across standard packet and frame validation paths in `orbitlogs-packets` (`lib.rs`).
- **Constellation Simulator Log & Comments**: Refined startup logs and updated fleet size support comments in `constellation-simulator` (`main.rs`).
- **Audit Logging Code Style**: Formatted the trace ID extraction logic and database error mapping in `mission-command-service` (`audit.rs`).

### Fixed
- **CORS Preflight Header typo in Timeline Service**: Fixed a bug in `apps/mission-timeline/src/main.rs` where the incorrect CORS header key `"access-control-origin"` was used in the preflight OPTIONS handler instead of `"access-control-allow-origin"`.
- **Input Guardrail Error Case Casing**: Fixed error message casing to be lowercase in the database injection detection checks in `ai-incident-commander` (`main.rs`).

---

## [4.4.0] - 2026-06-23

### Changed
- **Duration Constructor Standardizations**: Standardized the Kafka send timeout constructor to use double-precision float constructors (`Duration::from_secs_f64`) in `orbit-ops-service` (`ssa.rs`).
- **Code Formatting & Style Compliance**: Formatted and cleaned up `/health`, `/readiness` checks, flight dynamics trends functions, and close approach risk calculations in `orbit-ops-service` (`main.rs`, `flight_dynamics.rs`, and `ssa.rs`) to improve code readability.
- **Incident Commander Logging & Runbooks**: Refined log strings, error messages, and telemetry service names to use grammatically complete phrases (e.g. `ai-driven-incident-commander`), and structured embedded thermal and battery runbook templates in `ai-incident-commander` (`main.rs`).

---

## [4.3.0] - 2026-06-21

### Added
- **Production Readiness Audit Report**: Created `OrbitLogs-Project-Review-Report.md`, a comprehensive staff-level engineering assessment evaluating the system's production readiness, identifying architectural/security gaps, and defining the mitigation plan.
- **Rust Architecture & Code Review Audit**: Created `Project-Rust-Code-Audit-Review.md` evaluating the codebase against official Rust API Guidelines, covering enums for states, memory layout safety, error suppression patterns, and tokio-mutex and CRC table lookup optimizations.
- **Service Health Check APIs**: Implemented `/health`, `/liveness`, and `/readiness` HTTP endpoints in `orbit-ops-service` (`apps/orbit-ops-service/src/main.rs`).
- **Telemetry Collector Health API**: Integrated health router and endpoints (`/health`, `/liveness`, `/readiness`) into the OpenTelemetry collector's metrics server (`crates/space-otel-collector/src/main.rs`), listening on port `8091`.
- **Hardened Kubernetes Security Contexts**: Configured pod and container-level `securityContext` settings (non-root execution `runAsNonRoot: true`, `allowPrivilegeEscalation: false`, and read-only root filesystems where appropriate) across manifests for `orbit-ops-service`, `space-otel-collector`, and `telemetry-collector`.
- **Kubernetes Liveness & Readiness Probes**: Added HTTP liveness/readiness probes to the Kubernetes deployment specifications for `orbit-ops-service`, `space-otel-collector`, and `telemetry-collector`.
- **Collector Containerization**: Created a multi-stage, alpine-based production `Dockerfile` for `space-otel-collector` (`crates/space-otel-collector/Dockerfile`) executing under an unprivileged `orbitlogs` user context.
- **Kubernetes space-otel-collector Integration**: Created deployment manifests for `space-otel-collector` and registered the new service under the global `kustomization.yaml`.
- **Docker Compose space-otel-collector Integration**: Integrated `space-otel-collector` container service inside `docker-compose.yml` for unified local stack orchestration.
- **Vite Rollup Code Splitting**: Implemented rollup `manualChunks` in `vite.config.ts` for `mission-control-ui` to split third-party `node_modules` dependencies into a dedicated `vendor` chunk, reducing build warning sizes from 826KB to 104KB.
- **Kubernetes Secrets Safety Warnings**: Prepended security guidelines to `secrets.yaml` detailing production external secrets provider integration patterns.

### Changed
- **Prometheus Monitoring Scrapes**: Expanded `prometheus.yml` scrape configurations to track targets `orbit-ops-service` (port `8095`) and `space-otel-collector` (port `8091`).
- **DateTime Imports**: Consolidated `chrono::DateTime` and `Utc` imports in `orbit-ops-service/src/main.rs` to resolve cargo build E0425 compiler identifier errors.

---


## [4.2.0] - 2026-06-21

### Added
- **Playwright E2E Visual Canvas Tests**: Deployed a complete Playwright end-to-end visual test suite (`playwright.config.ts` and `visual-canvas.spec.ts`) in `apps/mission-control-ui` to validate 3D Three.js canvas dimensions, responsiveness, WebGL compatibility, and real-time WebSocket communication under load.
- **Automated TLE Scraping Daemon**: Implemented a background TLE scraping loop in `apps/orbit-ops-service` that automatically polls CelesTrak's live Starlink orbit database, parses standard three-line elements, and keeps satellite coordinates continuously synchronized.
- **LLM Input/Output Guardrails**: Integrated robust prompt/SQL injection sanitization at the input boundary of `apps/ai-incident-commander` along with sensitive leak scrubbers in the output path to protect internal configurations and database schemas.

### Removed
- **Obsolete Review Artifacts**: Deleted deprecated review and readiness markdown files from the workspace root to clean up production noise:
  - `Latest-review.md`
  - `SpaceX-Starlink-Level-OrbitLogs-Review.md`
  - `SpaceX-Starlink-Review.md`
  - `Orbitlogs-Production-Deployment-Readiness.md`
  - `Orbitlogs-review-report.md`

---

## [4.1.0] - 2026-06-21

### Added
- **Global Context Propagation**: Registered `TraceContextPropagator` in the shared `orbitlogs-models` telemetry initialization layer, activating W3C trace context extraction/injection across all asynchronous microservice boundaries.
- **OTel Resource Merging**: Refactored OpenTelemetry resource generation to merge user-specified metadata with `Resource::default()` to comply with standard specs (language, SDK name, and version).
- **Graceful Shutdown & Span Flushing**: Integrated SIGINT/SIGTERM handlers and explicit `shutdown_telemetry()` hooks in all Axum servers and processing workers to ensure zero-loss tracing flushes before container termination.
- **DashMap Metrics Cache**: Introduced global concurrent static caches (`SATELLITE_GAUGES_CACHE`, `GS_GAUGES_CACHE`) in `space-otel-collector` to avoid lock contention on high-cardinality label lookups and eliminate redundant heap allocations.

### Changed
- **Incident Commander Rebrand**: Refactored `ai-incident-commander` internals to update application name branding to `AI Driven Incident Commander`.
- **Duration Constructor Standardizations**: Refactored database connection retry loops, simulation frequencies, and network delay sleeps across all crates to leverage double-precision float constructors (`Duration::from_secs_f64`).
- **Telemetry Ingestion Cleanup**: Deleted the duplicate `telemetry.rs` file in `apps/telemetry-collector/src/` and standardized imports on the workspace models library.
- **Numeric Precision & Calculations**: Refactored flight dynamics base decay formulas and health scoring algorithms in `orbit-ops-service` to use standard floating point arithmetic.

### Fixed
- **Integration Test Grammar**: Refined error assertion text and log formatting inside `telemetry-collector` integration tests and the packets crate.

---


## [4.0.0] - 2026-06-18

### Added
- **Production Hardening & Scale Readiness**:
  - Implemented **TimescaleDB Continuous Aggregates** (`20260101000011_continuous_aggregates.sql`) to pre-calculate hourly averages of telemetry metrics (battery, solar output, CPU, temperature, altitude) to prevent CPU exhaustion.
  - Added a dedicated Kubernetes **Job** (`orbitlogs-db-migrations`) to execute database migrations sequentially on startup, preventing horizontal scaling pods from triggering concurrent schema lock contentions.
  - Hardened Horizontal Pod Autoscaling (HPAs) for telemetry ingestion proxies (`telemetry-collector`) and analytics consumers (`mission-health-engine`) to scale dynamically using external metric queues (`consumer_lag_messages` > 10,000) rather than generic CPU usage thresholds.
  - Resolved command verification lifecycle race conditions by removing legacy mock sleep timers in `mission-command-service` and relying strictly on closed-loop telemetry updates from `orbit-ops-service`.
  - Added a new `run-migrations` subcommand in the `telemetry-collector` CLI to support clean, isolated run-to-completion database bootstrap.
- **OrbitLogs Review Report**: Added [Orbitlogs-review-report.md](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/Orbitlogs-review-report.md) — a detailed, production-grade platform review report detailing the system architecture, microservices/crates breakdown, space tech domain applicability, security/architectural hardening remediations, shortcomings, and future improvement scope.
- **SpaceX Starlink Staff Engineer Review (v2.3.0)**: Added [SpaceX-Starlink-Engineer-Review.md](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/SpaceX-Starlink-Engineer-Review.md) — a comprehensive staff-level production readiness review scoring the platform across 10 engineering dimensions based on the legacy codebase. Published ratings: 6.5/10 production deployment readiness, 8.5/10 hiring signal.
- **SpaceX Starlink Audit Report (v4.0.0)**: Added [SpaceX-Starlink-Review.md](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/SpaceX-Starlink-Review.md) — a comprehensive staff-level production readiness review evaluating the hardened v4.0.0 platform architecture, mTLS networking, SHA-256 secure comparison, and geodetic TLE pass prediction calculations. Published ratings: 8.5/10 production deployment readiness, 9.5/10 hiring signal.
- **SpaceX Starlink Level Audit Report (v4.0.0)**: Created [SpaceX-Starlink-Level-OrbitLogs-Review.md](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/SpaceX-Starlink-Level-OrbitLogs-Review.md) — a complete, detailed end-to-end systems engineering audit report identifying race conditions in the command verification lifecycle, database schema migration execution contentions, and deployment scale blockers.
- **Production Deployment Readiness Report (v4.0.0)**: Created [Orbitlogs-Production-Deployment-Readiness.md](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/Orbitlogs-Production-Deployment-Readiness.md) — a final production deployment readiness assessment scoring the hardened platform 10/10 and confirming resolutions for all blockers.
- **Docker Compose DB Init Sync**: Synchronized database initialization scripts by copying missing v3 and v4 migrations (`09` and `10`) to the Docker Compose Postgres init directory `deployments/postgres/migrations/`.
- **SGP4 Orbit Propagation Service**: Integrated the pure Rust `sgp4` library to model physically realistic orbit state vectors (latitude, longitude, altitude, velocity, period, inclination, eccentricity) for the constellation, published as `orbit.position.updated` events on the `orbit.state` Kafka topic, and persisted to the `orbit_positions` Timescale DB hypertable.
- **TLE Ingestion Pipeline**: Implemented a complete pipeline for importing, validating, versioning, and checking the expiration of Two-Line Element (TLE) datasets, with event warnings `TLE_UPDATED`, `TLE_EXPIRED`, and `TLE_VALIDATION_FAILED` published to Kafka.
- **Ground Station Pass Prediction**: Implemented a pass prediction engine in `orbit.rs` mapping satellite orbits against ground stations using real line-of-sight elevation geometry (elevation > 5.0°) with event triggers `PASS_PREDICTED`, `PASS_STARTED`, and `PASS_ENDED`.
- **Ground Station Contact Scheduler**: Added calendar scheduling reservation checks in `ground_ops.rs` that prioritize emergency/critical operations and log execution to the `contact_schedules` database.
- **Space Situational Awareness (SSA) & Conjunctions**: Integrated space debris tracking and orbital close approach simulations with miss distance, relative velocity, and probability calculations to trigger `COLLISION_RISK_ALERT` events across low/medium/high/critical severities.
- **Flight Dynamics Engine**: Added calculation updates for altitude decay, drift, fuel burn rate, and Delta-V requirements, generating automatic `MANEUVER_RECOMMENDED`, `ORBIT_DECAY_ALERT`, and `FUEL_WARNING` events.
- **Closed-Loop Command Verification**: Implemented a background listener on `telemetry.processed` that intercepts live telemetry to verify command execution state transitions (e.g., verifying `ENABLE_PAYLOAD` against `payload_active == true` before marking command as `CONFIRMED` and publishing `COMMAND_VERIFIED`).
- **Operations Analytics Handler**: Created a REST API route `/api/analytics` calculating real pass success rate, contact success rate, command success rate, and average contact duration from Timescale database tables.
- **Dynamic WebSocket Stream**: Wired `/api/orbit/stream` to push real-time SGP4 coordinate updates to clients on 3-second intervals.
- **V4 Timescale Database Migration**: Added migration schema `20260101000010_v4_expansion.sql` defining relational tables and hypertables for `orbit_positions`, `tle_records`, `pass_predictions`, `contact_schedules`, `ssa_objects`, `conjunction_events`, `mission_plans`, `orbit_maneuvers`, and `command_verifications`.

### Changed
- **System Documentation Refactoring**: Rewrote all 12 system architecture, design specifications, operations runbooks, and testing strategy markdown documents in `docs/` and `design/` to align with the rebranding to **OrbitLogs** and the v4.0.0 feature baseline (SGP4 propagation, closed-loop command verification, mTLS cluster config, and token-bucket rate limits).
- **CHANGELOG Rebuilt from Full Git History**: Completely replaced the previous CHANGELOG.md with a comprehensive version traced from every git commit since the initial commit (`909c3f5`) through v3.0.0 (`595100b`). All versions (`v0.1.0` through `v3.0.0`) now have detailed Added / Changed / Fixed / Removed entries derived from actual `git show --stat` output rather than manual summaries.
- **Dynamic TLE Handlers**: Refactored the background propagation loops, REST endpoints, and WebSocket handlers in `main.rs` to fetch and use the latest database-imported TLE, falling back to generated defaults only when no records exist.
- **Resource Management Refactoring**: Cleaned up compilation warnings by updating `PlannerState`, `TrainingManager`, and `PostmortemManager` database actions to use their dedicated sub-component connection pools instead of the shared master state pool.

### Removed
- **`SpaceX-level-review.md`**: Deleted outdated interim review document (127 lines) that was superseded by the new comprehensive `SpaceX-Starlink-Engineer-Review.md`. Prior review artifacts (`SpaceX Engineer level review.md`, `SpaceX-Staff-review.md`, `SpaceX-Engineer-Review-Gemini.md`, etc.) had already been removed in v1.2.0.
- **Outdated Review Documents**: Deleted outdated `Orbitlogs-review-report.md` and legacy `SpaceX-Starlink-Engineer-Review.md` files, which have been consolidated and superseded by the updated `SpaceX-Starlink-Review.md` evaluating the v4.0.0 codebase.

---

## [3.1.0] - 2026-06-17

### Added
- **High Consumer Lag Prometheus Alerting**: Introduced a new alert rule `HighConsumerLag` in [rules.yml](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/deployments/prometheus/rules.yml) which triggers when the consumer lag on any Kafka topic exceeds 1000 messages for 5 minutes.
- **Digital Twin Projection Unit Tests**: Implemented comprehensive unit tests under a new `tests` module in [apps/digital-twin-engine/src/main.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/digital-twin-engine/src/main.rs) validating nominal, degraded, and failed battery, thermal, ADCS pitch drift, and propulsion propellant depletion thresholds.
- **Centralized Secure String Comparison**: Implemented constant-time secure string comparison `secure_compare` in the shared model crate [crates/orbitlogs-models/src/auth.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/crates/orbitlogs-models/src/auth.rs) utilizing SHA-256 to hash inputs to equal size and eliminate the timing/length oracle vulnerability.
- **Kafka Mutual TLS (mTLS) Hardening**: Added broker keystores, truststores, and client PEM keys/certificates via a new container-based orchestrator `certs-generator` in [docker-compose.yml](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/deployments/docker-compose/docker-compose.yml) executing [generate_kafka_certs.sh](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/deployments/docker-compose/scripts/generate_kafka_certs.sh). Upgraded internal broker listeners to SSL with mutual client authentication enforced.
- **Client Configuration mTLS Support**: Extended `KafkaConfigBuilder` in [crates/orbitlogs-models/src/kafka.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/crates/orbitlogs-models/src/kafka.rs) to support client PEM cert paths, and updated [telemetry-collector](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/telemetry-collector/src/main.rs), [telemetry-simulator](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/telemetry-simulator/src/main.rs), [ground-station-simulator](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/ground-station-simulator/src/main.rs), and [orbit-ops-service](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/orbit-ops-service/src/main.rs) to load client certifications in `docker-compose.yml`.

### Changed
- **Modularization of Mission Command Service**: Modularized the monolithic [main.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/mission-command-service/src/main.rs) by splitting its core operations into dedicated submodules:
  - [auth.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/mission-command-service/src/auth.rs) for authorization, RBAC role hierarchy, and login rate limiting.
  - [commands.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/mission-command-service/src/commands.rs) for listing, creation, and execution command lifecycles.
  - [audit.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/mission-command-service/src/audit.rs) for command event audit trailing.
- **Digital Twin Refactoring**: Extracted pure health projection calculations from `get_twin_health` route handler to `project_twin_health` in [apps/digital-twin-engine/src/main.rs](file:///c:/Users/bikrams/orbitlogs/OrbitLOgs/apps/digital-twin-engine/src/main.rs) to enable mock-free unit testing.
- **Connection Pool Refactoring**: Upgraded `orbitlogs-models` to execute programmatic SQLx database migrations in `establish_db_connection`.
- **JWT Key Separation**: Decoupled JWT signing keys from the shared service API keys across the command service configuration.
- **Login Rate Limiter**: Added client IP-based rate limiting on the `/api/auth/login` endpoint using `DashMap` in the command service.
- **Commands API Pagination**: Added limit/offset parameters and explicit column selection to `/api/commands`.
- **Incident Commander Connection Sharing**: Refactored the `ai-incident-commander` state to initialize and share a single `reqwest::Client` to avoid socket/file descriptor exhaustion, and integrated prompt injection guardrails.

### Fixed
- **Timing Vulnerabilities**: Replaced timing side-channel oracle string comparisons with secure centralized `orbitlogs_models::auth::secure_compare` across 6 services.
- **Pinned TimescaleDB Version**: Replaced the non-deterministic `latest` tag with the fixed `timescale/timescaledb:2.15.0-pg15` container tag across Compose and CI config files.

---

## [3.0.0] - 2026-06-17

### Added
- **`orbit-ops-service`**: New consolidated mission control and space operations service (Port `8095`) implementing:
  - **`orbit.rs`**: Orbital propagation and TLE-based satellite state calculations.
  - **`flight_dynamics.rs`**: Flight dynamics modeling (velocity, altitude, orbital period).
  - **`ground_ops.rs`**: Ground operations scheduling and contact window management.
  - **`planner.rs`**: Mission planning and sequencing engine.
  - **`ssa.rs`**: Space Situational Awareness (SSA) dashboard data aggregation and collision risk assessment.
  - **`training.rs`**: Operator training simulator with configurable scenario injection.
  - **`director.rs`**: AI Mission Director orchestration engine.
  - **`postmortem.rs`**: Automated incident postmortem generation.
  - **`twin.rs`**: Digital twin state synchronization module.
  - **`health.rs`**: Unified health aggregation endpoint.
  - **`graph.rs`**: Constellation topology graph builder.
- **New UI Panels** (`apps/mission-control-ui`):
  - `AiMissionDirector.tsx`: AI-powered mission decision support interface.
  - `FlightDynamics.tsx`: Real-time flight dynamics visualization panel.
  - `GroundOperations.tsx`: Ground operations scheduling and contact window display.
  - `MissionPlanner.tsx`: Interactive mission planning timeline component.
  - `PassSchedule.tsx`: Ground station pass schedule viewer.
  - `SsaDashboard.tsx`: Space situational awareness collision risk dashboard.
  - `TrainingSimulator.tsx`: Operator training and scenario injection UI.
- **Migration `20260101000009_v3_features.sql`**: New database schema tables supporting V3 orbit-ops features (contact windows, mission plans, SSA events, training scenarios).
- **`orbit-ops-service` Dockerfile**: Multi-stage production Dockerfile for the consolidated operations service.
- Added `orbit-ops-service` to `docker-compose.yml` on Port `8095`.

### Changed
- Updated `App.tsx` to integrate all 7 new V3 UI panels into the sidebar navigation.
- Updated `Cargo.toml` workspace to include `apps/orbit-ops-service`.
- Updated `Agent.md` with revised architectural guidance for V3 services.

---

## [2.3.0] - 2026-06-16

### Added
- **Request Timeout Protection**: Added a 30-second request timeout layer via `tower_http::timeout::TimeoutLayer` to Axum routers in `mission-command-service`, `mission-replay-engine`, `digital-twin-engine`, and `ai-incident-commander` to protect handler chains from slow DB/network connections.

### Changed
- **Standardized Lazy Initialization**: Replaced all usages of the third-party `lazy_static` macro with native `std::sync::LazyLock` (stabilized in Rust 1.80) across all services, fully purging `lazy_static` from workspace dependencies.
- **Production Structured Logging**: Enabled structured JSON logging format (`tracing_subscriber::fmt().json()`) across all backend services (including those using OpenTelemetry initialization under `init_telemetry`).
- **Collector Performance Enhancement**: Upgraded the telemetry collector to use the O(1) `lru` crate for telemetry history and replaced mutex-wrapped lists with lock-free `DashMap` concurrency.
- **Credentials Sanitization**: Removed all default PostgreSQL and API key secrets from `.env.example`, substituting them with explicit placeholders.

### Fixed
- **Clippy Lints & Clean Build**: Resolved all `clippy::manual_range_contains` occurrences with `!(min..=max).contains(&val)` checks.
- **Unused Warning Sanitization**: Cleaned up all unused imports, unused variables, unnecessary mutable definitions, and redundant match arms across the entire workspace to ensure a clean compilation under strict `-- -D warnings` mode.

---

## [2.2.0] - 2026-06-15

### Added
- **SpaceX Flight Operations Login Screen**: Created a high-fidelity access control credentials modal in `App.tsx` to dynamically request the console passkey and operator role context without baking credentials into compiled client-side assets.
- **Dynamic JWT Authentication Rotation**: Configured background JWT key rotation in `CommandCenter.tsx` to dispatch token signing handshakes automatically when operators transition roles.
- **Console Disconnect Controller**: Added a "Disconnect" control button to the UI header to clear `sessionStorage` and reload access state.
- **Containerization Support**: Created 5 new Dockerfiles for `mission-command-service`, `digital-twin-engine`, `ai-incident-commander`, `constellation-simulator`, and `mission-timeline` matching production multi-stage build requirements.
- **Command Service Unit Tests**: Added unit tests inside `apps/mission-command-service/src/main.rs` validating constant-time comparison, role parsing, and JWT signature verification.
- **Migration `20260101000008_fix_telemetry_schema.sql`**: Schema fix migration aligning `NOT NULL` columns with Rust `Option<>` model fields.
- **Migration `20260101000007_commands_and_timeline.sql`**: New tables `commands` and `command_audit_trail` and `timeline_events` supporting the command lifecycle and timeline aggregation services.

### Changed
- **RBAC Cryptographic Validation**: Replaced plaintext `X-User-Role` headers with securely signed JSON Web Tokens (JWT) verifying permissions in the `mission-command-service`.
- **Bearer Token Security Headers**: Configured state-modifying POST requests across `CommandCenter`, `DigitalTwinViewer`, and `AiCopilot` to authorize requests using `Authorization: Bearer` headers.
- **15-Member Docker Caching**: Updated all 10 Rust service Dockerfiles to copy all 15 workspace member `Cargo.toml` files and pre-compile dummy sources, caching Cargo dependencies effectively during layer caching.
- **CORS Restriction**: Configured CORS origin restrictions on `mission-command-service` locked to the `CORS_ALLOWED_ORIGIN` environment variable.
- **Graceful Shutdowns**: Implemented `SIGINT`/`SIGTERM` handlers for Axum servers and consumer loops across all 10 backend services to ensure clean process exits.

### Fixed
- **TimescaleDB Schema Alignment**: Synced telemetry schema columns (`DROP NOT NULL`) to align with Rust `Option<>` structures, added payload metrics columns, and synced initial migrations.
- **Compiler Warnings & Errors**: Fixed loop break types and async return parameters in `mission-timeline` and `constellation-simulator` compilation scopes.

---

## [2.1.0] - 2026-06-15

### Added
- **Phase 8 — AI Incident Commander** (`apps/ai-incident-commander`, Port `8088`):
  - Integrates Gemini and OpenAI LLM APIs for real-time incident analysis.
  - Fetches live satellite telemetry, recent events, and command history from TimescaleDB as RAG context.
  - Falls back to a high-fidelity rule-based diagnostic engine when external API keys are not configured.
  - Includes embedded operational runbooks (`DOC-TH-099` thermal, `DOC-EPS-104` battery).
- **Phase 9 — Space Schema Registry** (`crates/space-schema-registry`):
  - Manages schema version compatibility (Backward, Forward, Full) for V1/V2/V3 telemetry packet formats.
  - Runtime validation without service restarts.
- **Phase 10 — Fleet Health Crate** (`crates/fleet-health`):
  - Aggregates Fleet Readiness %, Subsystem Availability, Risk Score, and Capacity metrics across the full constellation.
- **Phase 11 — Space OTel Collector** (`crates/space-otel-collector`):
  - Consumes telemetry, command, and ground station Kafka streams.
  - Converts events to unified OpenTelemetry Metrics, Spans, and Logs.
  - Exposes Prometheus scrape endpoint on Port `8091`.
- **Phase 12 — Mission Command Service** (`apps/mission-command-service`, Port `8085`):
  - Full command lifecycle: `CREATED → APPROVED → QUEUED → TRANSMITTED → RECEIVED → VERIFIED/FAILED`.
  - Role-Based Access Control: `Operator`, `FlightDirector`, `MissionManager`, `Administrator`.
  - Publishes command events to `command.events` and `timeline.events` Kafka topics.
  - OpenTelemetry trace ID wired into command audit trail.
- **Phase 13 — Digital Twin Engine** (`apps/digital-twin-engine`, Port `8086`):
  - Maintains in-memory virtual satellite state from `telemetry.processed` stream.
  - Failure injection API for testing operator response and AI diagnostics.
  - Predictive hardware lifecycle duration calculations.
- **Phase 14 — Mission Timeline** (`apps/mission-timeline`, Port `8087`):
  - Consolidates events, anomaly logs, and state transitions in a TimescaleDB hypertable.
  - Exposes filtering by severity, satellite, and time range.
- **Phase 15 — Constellation Simulator** (`apps/constellation-simulator`, Port `8089`):
  - High-throughput loop capable of generating concurrent states for up to 5,000 satellites.
  - Simulates orbit degradation, battery capacity decay, and hardware failure cascades.
- **Phase 16 — ISS TLE Orbit Propagation**: Added `adapters.rs` to `telemetry-simulator` implementing real Two-Line Element (TLE) orbital propagation adapters.
- **Phase 17 — Ground Station Weather Attenuation**: Extended ground station simulator to inject backhaul network attenuation and atmospheric weather effects (storms, heavy snow) into link budget calculations.
- **Phase 18 — Advanced Sidebar Navigation**: Refactored `App.tsx` to introduce a full sidebar navigation layout with dedicated sub-dashboard sections for Command Center (RBAC controls), Mission Timeline search, Digital Twin viewer, and AI Copilot.
- **New UI Components**: Added `AiCopilot.tsx`, `CommandCenter.tsx`, `DigitalTwinViewer.tsx`, `MissionTimelineView.tsx` to `mission-control-ui`.
- **New Shared Crate Types**: Added `telemetry.rs` module and `init_telemetry` function to `orbitlogs-models` for centralized OTel initialization.
- **Satellite Orbit Image**: Added `docs/images/satellite_orbit.png` embedded in README.

### Changed
- **Rebrand to OrbitLogs**: Renamed the project from "OrbitLogs" to **OrbitLogs** across all Rust binaries, Docker container names, README, CHANGELOG, UI console titles, and Grafana dashboard labels.
- **Centralized Telemetry Init**: Consolidated OTel/tracing initialization into the shared `orbitlogs-models` crate, eliminating duplicate initialization code across services.
- **Documentation Overhaul**: Significantly expanded `README.md` with deep operational specifications for all 11 microservices, port mappings, and 18-phase roadmap progress.
- **Removed `CONTRIBUTING.md`**: Removed outdated contributing guide from the repository.

### Fixed
- **Tokio Runtime Context in Tests**: Resolved Tokio reactor panic warnings in simulator unit tests using graceful `Handle::try_current()` runtime fallbacks.
- **`FutureRecord` Lifetimes**: Fixed `'static` loop variable reference borrows in constellation simulator Kafka producer threads.

---

## [1.2.0] - 2026-06-08

### Added
- **Kubernetes Orchestration** (`deployments/kubernetes/`):
  - Namespace manifest (`namespace.yaml`).
  - Kustomize configuration (`kustomization.yaml`).
  - Secrets template (`secrets.yaml`).
  - App deployments: `telemetry-collector`, `telemetry-simulator`, `mission-health-engine`, `mission-replay-engine`, `ground-station-simulator`, `mission-control-ui`.
  - HPA (Horizontal Pod Autoscaler) for `telemetry-collector` and `mission-health-engine` based on CPU utilization.
  - Infrastructure StatefulSets: `kafka/statefulset.yaml`, `timescaledb/statefulset.yaml`.
- **Staff Architecture Review** (`SpaceX-Staff-Level-Architecture-Review.md`): Added architecture review document evaluating system design, scalability, observability, and hiring signal with full scoring rubric.
- **`KafkaConfigBuilder::from_env`**: Added convenience builder method to `crates/orbitlogs-models/src/kafka.rs` to construct Kafka producers directly from environment variables.

### Changed
- **3-Node Kafka KRaft Cluster**: Upgraded `docker-compose.yml` from single-broker to a full 3-node Kafka KRaft cluster with `replication-factor=3` and `min.insync.replicas=2` for all topics, eliminating Zookeeper dependency entirely.
- **Telemetry Simulator Resilience**: Updated `telemetry-simulator` to handle connection backoff and retry on collector unavailability.
- **Mission Health Engine Refactor**: Refactored `mission-health-engine/src/main.rs` to use the centralized `KafkaConfigBuilder` and `establish_db_connection` from `orbitlogs-models`.
- **Review Consolidation**: Condensed 8 separate review documents (`SpaceX Engineer level review.md`, `SpaceX-Staff-review.md`, `SpaceX-Engineer-Review-Gemini.md`, `SpaceX-Engineer-Review-Gemini-2.md`, `SpaceX-Engineer-Review-Gemini-3.md`, `Gemini-suggested-improvements.md`, `SpaceX-Staff-Level-Architecture-Review.md`, `Analysis.md`) into a single `SpaceX-Level-Engineer-Review.md`.

### Fixed
- **URL Hardcoding in UI**: Resolved hardcoded backend URLs in `App.tsx` to use environment-configurable values for staging/production deployments.
- **API Key CLI Robustness**: Improved API key resolution to check `--api-key` CLI flag, `API_KEY` env var, and Docker secrets file (`API_KEY_FILE`) with proper empty-string filtering.
- **Clippy Warnings**: Resolved all clippy linting errors in `ground-station-simulator` including collapsing nested `if` statements.
- **`ground-station-simulator` Dockerfile**: Added missing `COPY` for `orbitlogs-models` workspace member sources during multi-stage builds.
- **`cargo fmt`**: Applied `cargo fmt --all` workspace-wide to achieve full formatting compliance.

---

## [1.1.0] - 2026-06-07

### Added
- **W3C Trace Context Propagation over Kafka**: Injected W3C `traceparent` and `tracestate` headers into Kafka `RecordHeaders` for full end-to-end distributed tracing from HTTP edge through Kafka topics to TimescaleDB write.
- **Tenant-Isolated Rate Limiting**: Replaced global rate limiter with per-client token bucket (partitioned by API key or client IP via `DashMap`) to prevent cross-tenant request starvation under burst conditions.
- **Migration File Renaming**: Renamed migration files from `0N_name.sql` to ISO-style `20260101000000N_name.sql` for timestamp-ordered migration tooling compatibility.
- **Migration copies in `migrations/`**: Added top-level `migrations/` directory mirroring `deployments/postgres/migrations/` for `sqlx` CLI tooling compatibility.
- **`DashMap` State Caching**: Introduced lock-free `DashMap` for in-memory session tracking in the replay engine, replacing `Mutex<HashMap>`.
- **`dashmap` workspace dependency**: Added `dashmap = "5.5"` to workspace-level `Cargo.toml`.
- **Migration `20260101000006_retention_policies.sql`**: Added standalone retention policy migration file for compression and data eviction policies.
- **OrbitViewer GPU Memory Fix**: Refactored `OrbitViewer.tsx` Three.js scene to initialize only once using `useRef` to prevent GPU memory leaks and frame drops on re-renders.

### Changed
- **Replay Engine DB Throttling**: Replaced 200ms per-tick PostgreSQL position updates with in-memory `DashMap` session cache, flushing to the database every 5 seconds. Reduces database write load by approximately 96%.
- **Kafka KRaft Migration**: Eliminated Zookeeper dependency from `docker-compose.yml` and migrated to native Kafka KRaft mode across all brokers.
- **LRU Cache Eviction for Consumer**: Replaced unbounded consumer telemetry state cache with an LRU cache capped at 10,000 concurrent satellite entries to prevent unbounded memory growth.
- **At-Least-Once Delivery**: Implemented double-failure panic exits in the Kafka consumer — if both the TimescaleDB write and DLQ publish fail, the consumer panics and halts to prevent silent offset commits and data loss.
- **Centralized `establish_db_connection` and `KafkaConfigBuilder`**: Extracted duplicate database pool and Kafka client construction into the shared `orbitlogs-models` crate.
- **`tokio::sync::Mutex` Upgrade**: Replaced blocking `std::sync::Mutex` with `tokio::sync::Mutex` in the consumer cache to prevent Tokio executor thread starvation during async await points.

### Fixed
- **Timing Side-Channel Vulnerability**: Replaced standard `==` string comparisons for bearer tokens and WebSocket authorization with `subtle::ConstantTimeEq` to prevent timing oracle attacks.
- **Clippy Warnings**: Resolved all workspace clippy warnings including collapsing nested `if let` / `if` chains to ensure clean `-D warnings` CI gate.
- **Workspace Docker Builds**: Fixed `ground-station-simulator` Dockerfile to correctly `COPY` `orbitlogs-models` sources during the multi-stage builder stage.

---

## [1.0.1] - 2026-06-05

### Added
- **API Authentication Middleware**: Implemented Bearer token and query parameter (`?token=`) authentication middleware for telemetry ingestion and replay engine endpoints.
- **HTTP Integration Tests** (11 new): Tests covering authentication acceptance/rejection, CORS preflight, rate limiting (429 responses), body size enforcement (413 rejection), binary CCSDS packet decode, schema validation, and probe endpoints.
- **CORS Restriction**: Configured CORS middleware locked to deployment origins via `CORS_ALLOWED_ORIGIN` environment variable (replacing `allow_origin(Any)` wildcard).
- **Kafka SASL/TLS Configuration Hooks**: Added `KAFKA_SECURITY_PROTOCOL`, `KAFKA_SASL_MECHANISM`, `KAFKA_SASL_USERNAME`, `KAFKA_SASL_PASSWORD`, `KAFKA_SSL_CA_LOCATION` environment variable support to all Kafka client builders across the pipeline.
- **Kafka Circuit Breaker**: Implemented a custom 3-state circuit breaker (`Closed`, `Open`, `HalfOpen`) with 10-second recovery timeout on the Kafka producer to prevent thread exhaustion during broker outages.
- **Alertmanager Configuration** (`deployments/alertmanager/alertmanager.yml`): Initial Alertmanager routing rules and receiver configuration.
- **OpenAPI Specification** (`docs/openapi.yaml`): Complete OpenAPI 3.0 specification covering all ingestion, replay, health, and metrics endpoints.
- **`orbitlogs-packets` CRC and Binary Layer**: Added `crc.rs` module with CRC-16-CCITT implementation and full binary framing encode/decode logic in `lib.rs`.
- **Docker Secrets**: Added Docker Compose secrets infrastructure with mounted secret files for `postgres_password`, `api_key`, `database_url`, `kafka_sasl_password`, and `grafana_admin_password`.

### Changed
- **At-Least-Once Delivery**: Shifted from Kafka auto-commit to manual offset commits. Offsets committed asynchronously only after successful TimescaleDB write or DLQ enqueue fallback.
- **Non-Root Containers**: Updated all container images (`Dockerfile`) to create a dedicated `orbitlogs` user and run with non-privileged system rights (UID/GID 10001).
- **Hardcoded Credentials Removal**: Refactored Grafana provisioning, `docker-compose.yml`, and collector configurations to retrieve database secrets dynamically from environment variables and Docker secrets files.
- **Shared Models Extraction**: Extracted duplicate `TelemetryRecord` structs from the collector and simulator into a dedicated, reusable workspace library `orbitlogs-models`.
- **Serde Strict Parsing**: Configured `#[serde(deny_unknown_fields)]` on shared telemetry structs to block unexpected JSON key schemas and prevent silent field injection.
- **Telemetry Latency Buckets**: Tailored custom Prometheus histogram sub-10ms buckets optimized for high-resolution satellite telemetry update rates.
- **`cargo fmt`** compliance enforced: Applied `cargo fmt --all` and validated in CI.
- **Dockerfile Base Image Upgrades**: Upgraded builder base image to `rust:alpine` and runner to `alpine:3.21` across all service Dockerfiles.
- **CI `continue-on-error`**: Added `continue-on-error: true` to security audit steps to prevent CI failures from blocking on advisory database network timeouts.

### Fixed
- **Type Ambiguity**: Resolved compiler float resolution ambiguity in the telemetry simulator's orbital coordinate generator.
- **Lints & Warnings**: Removed all unused imports, unused variables, unused functions, and clippy warnings across all workspace members.
- **Git Repo Hygiene**: Untracked build-output directories (`target/`) from the repository index.
- **CI TimescaleDB Health Loop**: Replaced fixed CI database wait with a robust retry loop (`pg_isready` polling with up to 30 attempts).

---

## [1.0.0] - 2026-06-04

### Added
- **Rate Limiting Layer**: Implemented endpoint rate-limiting (100 req/sec) using Tower middleware with an in-memory token bucket on the `/telemetry` ingestion endpoint, returning `429 Too Many Requests` on overflow.
- **Readiness and Liveness Probes**: Added `/readiness` (database and Kafka connection verification) and `/liveness` endpoints to the ingestion collector for Kubernetes health checks.
- **Structured Error Responses**: Replaced generic text errors with detailed JSON response payloads mapping exact validation constraints.
- **`dlq_publish_failures_total` Prometheus Counter**: Added counter metric for Dead Letter Queue publish failures with Alertmanager alerting rule.
- **Vulnerability CI Scans**: Integrated `cargo-audit` (dependency vulnerability auditing) and `cargo-deny` (license compliance validation) into the GitHub Actions CI pipeline.
- **Integration Test Suite**: Created a fully automated mock integration test suite validating HTTP routing, body size rejections, rate limiting, schema checking, and CCSDS binary codec.
- **Mission Replay Engine** (`apps/mission-replay-engine`, Port `8084`):
  - Queries historical telemetry, events, and health changes from TimescaleDB.
  - Replays at configurable speeds: 1x, 5x, 10x, 50x, 100x.
  - Session controls: pause, resume, timeline scrubbing.
  - REST APIs for session management.
  - Server-Sent Events (SSE) real-time replay stream.
  - Migration `05_replay.sql` for replay session metadata.
- **Mission Control UI** (`apps/mission-control-ui`, Port `3000`):
  - React + TypeScript frontend.
  - Three.js 3D Earth orbit visualization (`OrbitViewer.tsx`).
  - Components: `FleetOverview.tsx`, `GroundStationStatus.tsx`, `IncidentPanel.tsx`, `ReplayControls.tsx`, `SatelliteDetail.tsx`.
  - Real-time WebSocket telemetry feed integration.
  - ESLint and TypeScript build verification in CI.
- **CCSDS-Inspired Binary Telemetry Layer** (`crates/orbitlogs-packets`):
  - Binary frame serialization/deserialization with CRC-16-CCITT validation.
  - `application/octet-stream` content-type routing in the ingestion API.
  - Schema version byte embedded in frame header (V1/V2/V3).
- **Ground Station Simulator** (`apps/ground-station-simulator`, Port `8083`):
  - Models 4 polar ground stations: Svalbard (GS-0001), Troll (GS-0002), Kiruna (GS-0003), McMurdo (GS-0004).
  - Simulates signal strength, packet loss, antenna health, weather interference, latency, bandwidth, and outage events.
  - Publishes ground station health states to `groundstation.health` Kafka topic.
  - Records history in TimescaleDB (`04_ground_station.sql` migration).
  - Grafana dashboard `ground-station-health.json`.
- **Mission Health Engine** (`apps/mission-health-engine`, Port `8082`):
  - Consumes `telemetry.processed` Kafka stream.
  - Evaluates subsystem health state (`GREEN`, `YELLOW`, `RED`) and numeric health score (0–100).
  - Calculates overall satellite and fleet health scores.
  - Publishes explainable health assessments to `mission.health` Kafka topic.
  - Persists health history in TimescaleDB (`03_mission_health.sql` migration).
  - Grafana dashboard `fleet-health.json`.
- **Spacecraft Subsystem Telemetry Refactor**: Refactored simulator and collector to support independent 7-subsystem telemetry streams:
  - **EPS (Power)**: Battery voltage, current, solar panel output.
  - **TCS (Thermal)**: CPU temperature.
  - **TTC (Communications)**: RSSI signal strength, packet loss, transmission latency.
  - **PROP (Propulsion)**: Fuel capacity, altitude, velocity.
  - **ADCS (Attitude)**: Orientation pitch, roll, yaw.
  - **Payload**: Instrument activity state and power draw.
  - **OBC (Flight Computer)**: CPU load and memory usage.
  - Migration `02_subsystems.sql` for subsystem-specific telemetry columns.
- **Phase docs**: Added `docs/architecture/phase1_subsystems.md` and `docs/architecture/phase2_health_engine.md`.
- **Roadmap tracker**: Added `docs/architecture/roadmap.md` tracking all 18 phases.

### Changed
- **Kafka Consumer Processor**: Extended `processor.rs` to perform stateful anomaly verification with cross-subsystem cascade detection (e.g., thermal spike → CPU throttling → ADCS degradation).
- **Telemetry Simulator Physics**: Added physical side-effect propagation across spacecraft subsystems in the telemetry generator.
- **Ground Station Link Modulation**: Extended simulator to consume `groundstation.health` events from Kafka and dynamically modulate communications latency and packet loss based on ground station health state.
- **DefaultBodyLimit**: Added `DefaultBodyLimit::max(10_240)` (10KB) to all Axum ingestion routers.
- **Prometheus Alert Rules** (`deployments/prometheus/rules.yml`): Added alerting rules for DLQ failures and high telemetry error rates.

---

## [0.2.0] - 2026-06-03

### Added
- **CI/CD Pipeline** (`.github/workflows/ci.yml`):
  - TimescaleDB service container integration in GitHub Actions.
  - Database initialization step applying all migration files in sequence.
  - `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --all` gates.
  - Cargo dependency vulnerability audit (`cargo-audit`).
  - License compliance check (`cargo-deny`).
  - Docker container build and non-root execution verification.
  - Frontend ESLint + production build verification.
  - End-to-end smoke test via Docker Compose stack startup and telemetry submission.
- **`deny.toml`**: Added license allowlist configuration for `cargo-deny`, permitting CC0, Unlicense, and 0BSD licenses.
- **`scripts/`**: Added operational scripts (`bootstrap.sh`, `seed_db.sh`, `benchmark.sh`).
- **`benchmarks/load_test.js`**: k6 load testing script for telemetry ingestion endpoint throughput testing.
- **`docs/` documentation suite**: Architecture, design, runbooks, and testing strategy documents:
  - `docs/architecture/system_architecture.md`
  - `docs/design/collector_design.md`, `database_design.md`, `kafka_design.md`, `observability_design.md`, `security_practices_design.md`, `simulator_design.md`
  - `docs/runbooks/operations_runbook.md`
  - `docs/testing/testing_strategy.md`
- **`CONTRIBUTING.md`**: Initial contributing guidelines (later removed in v2.1.0 rebrand).
- **`CHANGELOG.md`**: Initial changelog file added to the repository.

### Changed
- **Rust Builder Image**: Stabilized Dockerfile base image on `rust:alpine` after iterating through `rust:1.75`, `rust:1.82`, `rust:1.85`, and `rust:latest` to resolve compilation issues with edition 2024 dependencies.
- **`tower` dev-dependency features**: Enabled required `tower` features in `dev-dependencies` for integration test router construction.
- **`Cargo.lock` tracked in CI**: Added `Cargo.lock` to `COPY` steps in all Dockerfiles for deterministic dependency caching.

### Fixed
- **CI Database Wait**: Replaced non-deterministic `sleep` with a robust `pg_isready` polling loop.
- **Container Multi-Stage Build**: Fixed `COPY` step for `orbitlogs-models` crate sources in the builder stage to resolve `cargo build` path resolution failures.
- **Simulator Compilation**: Fixed loop variable lifetime issues in the telemetry simulator's orbital generator.

---

## [0.1.0] - 2026-05-24

### Added
- **Initial Platform** (`feat: implement OrbitLogs V1 platform and documentation`):
  - **`apps/telemetry-collector`**: Axum HTTP server ingesting satellite telemetry to Kafka (`telemetry.raw`) with a concurrent Tokio consumer writing to TimescaleDB. Subcommand modes: `run-server`, `run-consumer`, `run-all`.
  - **`apps/telemetry-simulator`**: Rust telemetry generator simulating realistic satellite state (battery, thermal, orientation, fuel, communications, CPU) and posting to the collector at configurable intervals.
  - **`crates/orbitlogs-models`**: Shared workspace crate with canonical `TelemetryRecord` struct, `validate()` method with physical bounds checking, and Kafka/DB construction utilities.
  - **Kafka Integration**: KRaft-mode Kafka broker with topics `telemetry.raw`, `telemetry.processed`, `telemetry.invalid` (DLQ).
  - **TimescaleDB Schema** (`01_init.sql`): `satellites`, `telemetry`, and `events` hypertables with composite indexes, compression policies (7-day threshold), and retention policies (90-day telemetry, 180-day events).
  - **Prometheus Metrics**: Custom histogram buckets, counters, and gauges for telemetry throughput, processing latency, error rates, and DLQ failures.
  - **Grafana Dashboards**: Pre-provisioned dashboards for fleet overview, satellite health, telemetry throughput, and infrastructure health.
  - **OpenTelemetry Integration**: OTLP trace exporter to Jaeger via the OTel Collector. `tracing-opentelemetry` spans on all HTTP handlers and Kafka consumers.
  - **`deployments/docker-compose/docker-compose.yml`**: Full local stack including TimescaleDB, Kafka (KRaft), Prometheus, Grafana, Jaeger, OTel Collector, telemetry-collector, telemetry-simulator.
  - **Mermaid Architecture Diagram**: System architecture diagram added to `README.md`.
  - **Apache 2.0 License**: `LICENSE` file committed.

### Changed
- **README.md**: Expanded from placeholder to full platform documentation with architecture diagram, feature summary, quick start guide, and port mapping table.
- **Mermaid diagram syntax fixes**: Wrapped labels and subgraph names in double quotes to resolve GitHub Mermaid rendering failures.
- **License badge**: Updated badge from placeholder to Apache 2.0.

---

*OrbitLogs — Real-Time Spacecraft Telemetry & Mission Control Observability Platform*
*Built with Rust, React, TypeScript, Kafka, TimescaleDB, and OpenTelemetry*
