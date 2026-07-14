# OrbitLogs Platform Roadmap
### Version: v4.5.0 (Latest)

This roadmap outlines the evolution of the **OrbitLogs** spacecraft ground segment operations platform, detailing all completed development phases (1 through 25) and upcoming initiatives.

---

## 📅 Roadmap Status

### 🟢 Completed Phases (v1.0.0 through v4.5.0)

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
*   **Phase 17: Ground Station Weather & Link Attenuation**
    *   Models signal attenuation caused by weather events and network delays.
*   **Phase 18: Advanced Sidebar Navigation Layout**
    *   Provides sub-dashboards for incident co-piloting and command audits.
*   **Phase 19: Space Traffic Management (STM) & Conjunction Assessment**
    *   Automated close approach screening, collision probability ($P_c$) calculations, 3D orbital debris visualization, and autonomous collision avoidance maneuver (CAM) recommendations.
*   **Phase 20: SGP4 Astrodynamics Library & Real-Time TLE Ingestion**
    *   Core coordinate frame conversions (ECI/ECEF/ENU), GMST rotation, geodetic/Cartesian transforms, and secure TLE ingestion API.
*   **Phase 21: Contact Window Prediction**
    *   Background service computing ground station access windows using line-of-sight elevation geometry and SGP4 propagation (`apps/contact-window-predictor`).
*   **Phase 22: Constellation Scheduling & Optimization**
    *   Task scheduling, communication slot allocation, and ground station optimizer with look-ahead heuristics (`apps/constellation-scheduler`).
*   **Phase 23: Orbit Determination (Extended Kalman Filter)**
    *   Precise state estimation using EKF with J2 oblateness perturbations and GPS pseudorange observations (`apps/orbit-determination-service`).
*   **Phase 24: Autonomous Mission Planning**
    *   Hierarchical Task Network (HTN) planner for synthesizing safe command sequences with digital twin pre-validation (`apps/autonomous-mission-planning`).
*   **Phase 25: Propulsion Optimization & Test Utilities**
    *   Tsiolkovsky rocket equation fuel optimization, delta-v burn calculations (`crates/orbitlogs-propulsion`), and centralized test utility crate (`crates/orbitlogs-test-utils`).

---

## 🚀 Future Roadmap & Remediations (v6.0.0 Planning)

1.  **Unified Helm Chart Deployment**: Define a single parameterized Helm chart deploying all 17 microservices with environment-specific values files.
2.  **Multi-Region Geo-Replication**: Transition TimescaleDB to a geo-distributed setup for sub-millisecond local write paths across global ground stations.
3.  **Zero-Copy Telemetry Parsing**: Refactor the binary parser to yield telemetry data views (`TelemetryView<'buf>`) using references instead of allocating owned structures on every packet.
4.  **WASM Client-Side Propagation**: Compile `orbitlogs-astrodynamics` to WebAssembly for client-side SGP4 propagation, reducing server load for 10,000+ satellite visualization.
5.  **Structured API Error Protocol**: Evolve REST APIs from free-text error payloads to machine-readable error responses with codes (e.g., `ERR_AUTH_EXPIRED`, `ERR_COMMAND_INVALID_STATE`).
6.  **Ingress Deduplication Layer**: Add a telemetry deduplication buffer at the ingestion tier to handle duplicate packets received by multiple ground stations simultaneously.

