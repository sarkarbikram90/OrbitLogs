# OrbitLogs Product Roadmap 🗺️

This document outlines the commercial product development roadmap and enterprise milestones for **OrbitLogs**.

---

## 🟢 Completed Core Platform (v1.0.0 – v4.5.0)

- [x] **Subsystem Telemetry Engine**: 7 core spacecraft subsystems (EPS, TCS, TTC, PROP, ADCS, Payload, OBC) with physics-based side-effect modeling.
- [x] **Astrodynamics Library**: SGP4 propagation engine, coordinate rotations (ECI/ECEF/ENU), and ground station access predictor.
- [x] **Operations & Planning**: HTN-based command planner, Extended Kalman Filter (EKF) orbit determination, and satellite conjunction risk scoring.
- [x] **OpenTelemetry & Kafka Compliance**: W3C trace context propagation over Kafka headers, OTLP exporters, and Prometheus scraper integration (`/metrics`).
- [x] **Production Security Hardening**: Non-root container builds, constant-time token checks, TimescaleDB hypertable compression, and at-least-once Kafka offset commits.

---

## 🟡 Q3 Milestones (Short-Term Startup Focus)

- [ ] **Multi-Tenant Fleet Isolation**: Enforce `tenant_id` / `org_id` context isolation across TimescaleDB hypertables, Kafka message streams, and API gateways.
- [ ] **Lightweight Flight SDKs**: Release official Python (`orbitlogs-sdk-python`) and Rust (`orbitlogs-sdk-rust`) SDKs for 2-line telemetry integration in flight software.
- [ ] **Incident Webhook Dispatchers**: Automated alert triggers to PagerDuty, Slack, Teams, and Webhooks when health scores drop or conjunction collision risks breach $P_c > 10^{-4}$.
- [ ] **Standardized Health Probes**: `/healthz` (liveness) and `/readyz` (readiness) endpoints implemented across all 17 backend microservices.

---

## 🔵 Q4 Milestones (Enterprise & SSO Focus)

- [ ] **Enterprise SSO (SAML 2.0 / OIDC)**: Native integration for Okta, Azure AD, Ping Identity, and Google Workspace SSO.
- [ ] **SOC 2 Type II Immutable Audit Logging**: High-frequency command audit logging hypertable with cryptographically hashed session signatures.
- [ ] **Managed Cloud Helm Charts**: Production Kubernetes Helm Charts and Terraform scripts for AWS EKS, Azure AKS, and GCP GKE deployments.
- [ ] **WASM Client-Side Astrodynamics**: Compile `orbitlogs-astrodynamics` to WebAssembly to offload 10,000+ satellite orbit propagation calculations to client browsers.

---

## 🔴 Enterprise & Defense Expansion

- [ ] **Air-Gapped On-Premise Deployment Package**: Offline Docker/K8s installer for government, defense, and high-security ground control centers.
- [ ] **Autonomous Collision Avoidance Maneuver (CAM) Execution**: Automated end-to-end burn vector synthesis, approval, and execution loop between `orbit-ops-service` and `mission-command-service`.
- [ ] **Multi-Region Geo-Replicated Ground Gateway**: Low-latency ingestion across global polar ground station nodes with active-active TimescaleDB replication.
