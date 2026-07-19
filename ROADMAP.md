# OrbitLogs Project Roadmap 🗺️

This document outlines the development roadmap and technical milestones for **OrbitLogs**, aligning with CNCF Sandbox criteria and space industry observability requirements.

---

## 🟢 Completed Milestones (v1.0.0 – v4.5.0)

- [x] **Subsystem Telemetry Engine**: 7 core spacecraft subsystems (EPS, TCS, TTC, PROP, ADCS, Payload, OBC) with physics-based side-effect modeling.
- [x] **Astrodynamics Library**: SGP4 propagation engine, coordinate rotations (ECI/ECEF/ENU), and ground station access predictor.
- [x] **Operations & Planning**: HTN-based command planner, Extended Kalman Filter (EKF) orbit determination, and satellite conjunction risk scoring.
- [x] **OpenTelemetry & Kafka Compliance**: W3C trace context propagation over Kafka headers, OTLP exporters, and Prometheus scraper integration (`/metrics`).
- [x] **Production Security Hardening**: Non-root container builds, constant-time token checks, TimescaleDB hypertable compression, and at-least-once Kafka offset commits.

---

## 🟡 Short-Term Goals (Next 1 – 3 Months)

- [ ] **CNCF Sandbox Application**: Formal submission of OrbitLogs to the CNCF Sandbox under the Linux Foundation.
- [ ] **Automated License Header Verification**: Integrate a CI action to verify Apache 2.0 license header notices across all workspace files.
- [ ] **Zero-Copy Parser Refactoring**: Optimize telemetry ingestion throughput by implementing lifetime references (`TelemetryView<'a>`).
- [ ] **Unified Error Payload Schema**: Standardize REST/WebSocket error responses with application error codes and OTel trace context identifiers.

---

## 🔵 Medium-Term Goals (Next 3 – 6 Months)

- [ ] **Production Helm Charts**: Author official production Helm charts to deploy all 17 microservices on Kubernetes with Prometheus/Grafana integration.
- [ ] **WebAssembly (WASM) Astrodynamics**: Compile `orbitlogs-astrodynamics` to WASM for client-side orbit propagation directly inside the web console.
- [ ] **Ingress Deduplication Buffer**: Implement time-windowed packet deduplication in `telemetry-collector` for overlapping ground station passes.
- [ ] **OpenTelemetry Space Payload Semantic Conventions**: Author a draft specification for spacecraft telemetry semantic conventions within the OTel community.

---

## 🔴 Long-Term Goals (6 Months+)

- [ ] **Multi-Region TimescaleDB Replication**: Geo-distributed database replication across polar ground stations.
- [ ] **KubeEdge & Edge Deployment**: Deploy `telemetry-collector` as a KubeEdge application on low-bandwidth ground terminal gateways.
- [ ] **Autonomous Orbital Collision Avoidance Maneuver (CAM) Automation**: End-to-end integration between `orbit-ops-service`, `ai-incident-commander`, and `mission-command-service`.
