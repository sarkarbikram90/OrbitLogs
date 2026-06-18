# OrbitLogs Platform Roadmap
### Version: v4.0.0 (Latest)

This roadmap outlines the evolution of the **OrbitLogs** spacecraft ground segment operations platform, detailing the completed development phases (1 through 18) and detailing upcoming initiatives.

---

## 📅 Roadmap Status

### 🟢 Completed Phases (v1.0.0 through v4.0.0)

*   **Phase 1: Spacecraft Subsystem Architecture**
    *   Models 7 independent subsystems per spacecraft.
    *   Implements stateful caching and telemetry merges.
*   **Phase 2: Mission Health Engine**
    *   Evaluates raw telemetry metrics into discrete GREEN/YELLOW/RED health states.
*   **Phase 3: Ground Station Simulator**
    *   Integrates polar pass geometric properties with signal strength and loss modulations.
*   **Phase 4: Telemetry Packet Layer**
    *   Enforces CCSDS binary formats with CRC-16 checksum verification.
*   **Phase 5: Mission Replay Engine**
    *   Enables SSE-based historical telemetry replays at speeds up to 100x.
*   **Phase 6: Mission Control User Interface**
    *   Provides React-based telemetry dashboards.
*   **Phase 7: Orbital Visualization**
    *   Renders 3D Earth orbit tracks, pass coordinates, and link beams.
*   **Phase 8: AI Incident Commander**
    *   Integrates Gemini API and operational runbooks for anomaly diagnoses.
*   **Phase 9: Space Schema Registry**
    *   Manages runtime compatibility for V1, V2, and V3 formats.
*   **Phase 10: Fleet Health Metrics**
    *   Computes availability, readiness, and risk ratings across the fleet.
*   **Phase 11: Space OTel Collector**
    *   Transforms Kafka streaming events into OpenTelemetry span metrics.
*   **Phase 12: Axum Mission Command Service & RBAC**
    *   Establishes multi-tier command approvals and audits.
*   **Phase 13: Digital Twin Engine**
    *   Maintains synchronized virtual states for constellation models.
*   **Phase 14: Mission Timeline**
    *   Consolidates database events, alerts, and logs into a single feed.
*   **Phase 15: Constellation Simulator**
    *   Supports high-efficiency state simulations for up to 5,000 active nodes.
*   **Phase 16: ISS Orbit and TLE Propagator Adapters**
    *   Integrates SGP4 orbital mechanics algorithms.
*   **Phase 17: Ground Station weather & Link Attenuation**
    *   Models signal attenuation caused by weather events and network delays.
*   **Phase 18: Advanced sidebar navigation layout**
    *   Provides sub-dashboards for incident co-piloting and command audits.

---

## 🚀 Future Roadmap & Remediations (v5.0.0 Planning)

1.  **Kubernetes Manifest Consolidation**: Define unified Helm charts deploying all 11 microservices under customized values files.
2.  **Autoscaling via Consumer Group Queue Lag**: Set Horizontal Pod Autoscalers to scale consumer worker pools based on Kafka queue partition depths rather than generic CPU usage.
3.  **TimescaleDB Continuous Aggregates**: Write materialized roll-ups on the telemetry hypertable to speed up historical metrics queries at massive scale.
4.  **Reconcile Command Verification Loops**: Deprecate the legacy mock sleep loop in the command service to prevent database race conditions, using only closed-loop telemetry validations.
5.  **Automated Visual E2E UI Tests**: Integrate Playwright or Cypress suites validating Three.js and WebSocket connections.
