# Changelog 📖

All notable changes to the **Open SpaceOps** platform will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

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
