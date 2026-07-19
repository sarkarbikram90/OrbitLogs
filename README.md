# OrbitLogs 🚀

[![OrbitLogs CI/CD](https://github.com/sarkarbikram90/OrbitLogs/actions/workflows/ci.yml/badge.svg)](https://github.com/sarkarbikram90/OrbitLogs/actions/workflows/ci.yml)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Language: TypeScript](https://img.shields.io/badge/Language-TypeScript-blue.svg)](https://www.typescriptlang.org/)
[![Database: TimescaleDB](https://img.shields.io/badge/Database-TimescaleDB-blue.svg)](https://www.timescale.com/)
[![Message Queue: Kafka (KRaft)](https://img.shields.io/badge/Queue-Kafka%20(KRaft)-green.svg)](https://kafka.apache.org/)
[![Observability: OpenTelemetry](https://img.shields.io/badge/Observability-OpenTelemetry-blueviolet.svg)](https://opentelemetry.io/)
[![Deployment: Enterprise Ready](https://img.shields.io/badge/Deployment-Enterprise%20Ready-brightgreen.svg)](#-enterprise--commercial-features)
[![Security: SOC 2 Compliant Architecture](https://img.shields.io/badge/Security-SOC%202%20Compliant-blue.svg)](SECURITY.md)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green.svg)](LICENSE)

![OrbitLogs Realistic Satellite Orbit Visualization](docs/images/satellite_orbit.png)

> **Commercial Spacecraft Telemetry, Digital Twin & Mission Control Observability Platform**

OrbitLogs is a production-grade, high-performance commercial real-time satellite telemetry ingestion, stream processing, digital twin simulation, historical replay, and 3D constellation visualization platform. Engineered with Rust, React, and TypeScript microservices, OrbitLogs enables commercial satellite operators, constellation managers, and defense space agencies to monitor, troubleshoot, and optimize satellite fleets orbiting Earth.

---

## 💼 Enterprise Product Tiers

OrbitLogs is packaged for commercial satellite operators of all scales, from early-stage CubeSat startups to mega-constellations:

| Feature / Capability | Developer (Free) | Professional | Enterprise & Defense |
| :--- | :---: | :---: | :---: |
| **Monitored Spacecraft Assets** | Up to 5 Satellites | Up to 100 Satellites | Unlimited Constellations |
| **Telemetry History Retention** | 7 Days | 90 Days | Multi-Year / Custom Retention |
| **Multi-Tenancy Data Isolation** | Community Shared | Isolated Tenant Workspace | Dedicated Private VPC / Air-Gapped |
| **AI Incident Commander (RAG Copilot)** | ❌ | ✅ Included | ✅ Fine-Tuned Custom Models |
| **Conjunction Avoidance & STM** | Manual Review | Automated Collision Alerts | Automated Burn Vector Generation |
| **Extended Kalman Filter (EKF) Orbit Determination** | Basic | ✅ Included | ✅ Hardware In The Loop (HITL) |
| **Authentication & SSO** | API Token Header | OAuth2 / OIDC | Enterprise SAML 2.0 (Okta, Azure AD) |
| **Support SLA** | Community | 24/7 Email & Slack (4h SLA) | Dedicated Technical Account Manager (1h SLA) |

---

## 🌌 Core Value Proposition for Space Operators

Traditional ground segment monitoring software relies on legacy monolithic tools that lack real-time scalability, modern observability, or automated anomaly diagnosis. OrbitLogs transforms satellite fleet operations:

*   **Subsystem-Level Observability**: Track state of health across individual spacecraft subsystems (EPS, TCS, TTC, PROP, ADCS, Payload, OBC) with sub-second latency.
*   **Synchronized Digital Twin Engine**: Simulate spacecraft thermal stress, battery capacity fade, and power budgets in real-time, allowing operators to run "what-if" failure injections before commanding physical assets.
*   **Time-Series Timeline Replay**: Scrub back through mission history at up to 100x speed to analyze complex failure cascades and automate incident postmortems.
*   **Automated Space Traffic Management (STM)**: Compute Probability of Collision ($P_c$) every 10 seconds against 250+ orbital debris objects and synthesize optimal Collision Avoidance Maneuvers (CAM).
*   **Zero-Loss Ingestion Pipeline**: Memory-safe Rust binary decoders combined with manual Kafka offset commit validation guarantee at-least-once delivery of critical telemetry records.

---

## 🛠️ Microservice Architecture

OrbitLogs operates as a decoupled, event-driven microservices architecture:

```mermaid
graph TD
    classDef simulator fill:#E05A47,stroke:#FFF,stroke-width:1px,color:#FFF;
    classDef api fill:#2B6CB0,stroke:#FFF,stroke-width:1px,color:#FFF;
    classDef broker fill:#2F855A,stroke:#FFF,stroke-width:1px,color:#FFF;
    classDef consumer fill:#6B46C1,stroke:#FFF,stroke-width:1px,color:#FFF;
    classDef db fill:#319795,stroke:#FFF,stroke-width:1px,color:#FFF;
    classDef visual fill:#D53F8C,stroke:#FFF,stroke-width:1px,color:#FFF;
    classDef trace fill:#DD6B20,stroke:#FFF,stroke-width:1px,color:#FFF;

    SIM["🚀 Telemetry Simulator<br/>(7 Subsystems Loop + GS Modulator)"]:::simulator
    GS_SIM["📡 Ground Station Simulator<br/>(Svalbard, Troll, Kiruna, McMurdo)"]:::simulator
    API["🔌 Ingestion API Gateway<br/>(Rust Axum Server on Port 8080)"]:::api
    K_RAW["📥 Kafka Topic: telemetry.raw"]:::broker
    K_PROC["📤 Kafka Topic: telemetry.processed"]:::broker
    K_HLTH["💚 Kafka Topic: mission.health"]:::broker
    K_GS_HLTH["📡 Kafka Topic: groundstation.health"]:::broker
    K_DLQ["⚠️ Kafka Topic: telemetry.invalid<br/>(Dead Letter Queue)"]:::broker
    COL["⚙️ Telemetry Processor & Merger<br/>(Tokio Consumer & State Cache)"]:::consumer
    HLTH["💚 Mission Health Engine<br/>(Rules Evaluator & Alert Webhooks)"]:::consumer
    TSDB[("🗄️ TimescaleDB Hypertables<br/>(Telemetry, Health, Events, Replays)")]:::db
    REPLAY["🔄 Mission Replay Engine<br/>(SSE Playback Stream on Port 8084)"]:::api
    UI["🛰️ Mission Control UI<br/>(React + 3D Three.js Globe on Port 3000)"]:::visual
    PROM["📈 Prometheus Scraper"]:::visual
    GRA["📊 Grafana Dashboards<br/>(Fleet, Infrastructure on Port 3005)"]:::visual
    OTEL["🔌 OTel Collector"]:::trace
    JAEG["🔍 Jaeger UI (OTLP Tracing)"]:::trace

    SIM -->|CCSDS Binary Frames| API
    API -->|Schema/CRC Failures| K_DLQ
    API -->|Validated Payload| K_RAW
    GS_SIM -->|Publish Ground Health| K_GS_HLTH
    K_GS_HLTH -->|Modulate Comms Latency/Loss| SIM
    K_GS_HLTH -->|Ground State Context| HLTH
    K_RAW -->|Stream Consume| COL
    COL -->|Perform Anomaly Verification| COL
    COL -->|Batch Write stateful merges| TSDB
    COL -->|Publish Processed Event| K_PROC
    K_PROC -->|Consume processed telemetry| HLTH
    HLTH -->|Evaluate & Write Health| TSDB
    HLTH -->|Publish Health State| K_HLTH
    
    API -.->|Real-time WS Telemetry| UI
    REPLAY -.->|SSE Replay Stream| UI
    TSDB -->|Query ranges & seek| REPLAY
    UI -->|Interactive timeline seek/speed| REPLAY
    
    API -.->|Metrics /metrics| PROM
    HLTH -.->|Metrics /metrics| PROM
    PROM -.->|Query Metrics| GRA
    TSDB -.->|Query Hypertable Data| GRA
    API -.->|Traces via OTLP| OTEL
    COL -.->|Traces via OTLP| OTEL
    OTEL -.->|Forward Traces| JAEG
    GRA -.->|Integrated Trace Navigation| JAEG

    subgraph Ground Segment & Simulation
        SIM
        GS_SIM
    end

    subgraph Core Ingestion & Stream Processing
        API
        K_RAW
        K_DLQ
        COL
        K_PROC
        HLTH
        K_HLTH
        K_GS_HLTH
    end

    subgraph Data Archiving & Playback
        TSDB
        REPLAY
    end

    subgraph Console & Operations
        UI
        GRA
        JAEG
    end
```

---

## 🔌 Port Mapping & Developer Interfaces

| Service / Interface | Port | Protocol | Description |
| :--- | :--- | :--- | :--- |
| **Mission Control Console** | `3000` | HTTP | Operations UI & 3D Orbit Visualization |
| **Telemetry Ingestion Gateway** | `8080` | HTTP/WS | Binary/JSON Ingestion Gateway & WS Live Streams |
| **Telemetry Simulator** | `8081` | HTTP | Subsystem Telemetry Generator |
| **Mission Health Engine** | `8082` | HTTP | Health Rules & Alert Webhook Dispatcher |
| **Ground Station Simulator** | `8083` | HTTP | Ground Link Budget & Weather Modulator |
| **Mission Replay Engine** | `8084` | HTTP/SSE | Time-Series Historical Playback Server |
| **Mission Command Service** | `8085` | HTTP | RBAC Command Lifecycle Control |
| **Digital Twin Engine** | `8086` | HTTP | Virtual Satellite State & Failure Injector |
| **Mission Timeline Engine** | `8087` | HTTP | Event Aggregator & Historical Search API |
| **AI Incident Commander** | `8088` | HTTP | Operational Copilot powered by Gemini/OpenAI RAG |
| **Constellation Simulator** | `8089` | HTTP | Mass Fleet State Loop (5,000+ Spacecraft Assets) |
| **Space OTel Collector** | `8091` | HTTP | OpenTelemetry Metrics Mapper (`/metrics`) |
| **Orbit Operations & STM** | `8095` | HTTP | Space Traffic Management & Conjunction Assessment |
| **Contact Window Predictor** | `8096` | HTTP | Ground Access Window Predictor |
| **Constellation Scheduler** | `8097` | HTTP | Task Allocation & Antenna Slew Optimizer |
| **Orbit Determination Service** | `8098` | HTTP | Extended Kalman Filter (EKF) Orbit Estimation |
| **Autonomous Mission Planning** | `8099` | HTTP | HTN Command Sequence Planner (`/healthz`) |
| **TimescaleDB (Postgres)** | `5432` | TCP | Telemetry, Command Audits, and Event Store |
| **Prometheus** | `9090` | HTTP | Performance Metrics Database |
| **Grafana** | `3005` | HTTP | Operational Fleet Dashboards |
| **Jaeger UI** | `16686` | HTTP | OpenTelemetry Distributed Tracing UI |

---

## 🚀 Quick Start (Local Evaluation)

### Prerequisites
* **Docker** & **Docker Compose**
* **Rust (v1.82+)**
* **Node.js (v20+)**

### Launch Local Infrastructure
```bash
chmod +x scripts/*.sh
./scripts/bootstrap.sh
```

Access the **Mission Control Console** at [http://localhost:3000](http://localhost:3000).

---

## 🔒 Enterprise Security & Compliance

- **SOC 2 Type II Architecture**: Dedicated audit trail hypertables track every command submission, parameter edit, and user session.
- **W3C OpenTelemetry Tracing**: Trace context (`traceparent`) is propagated across async Kafka boundaries to Jaeger for end-to-end observability.
- **Non-Root Container Hardening**: Containers run under dedicated unprivileged system user `orbitlogs` (UID/GID `10001`).

---

## 📬 Commercial Contact & Sales

Interested in deploying OrbitLogs for your constellation operations or evaluating our Enterprise / On-Premise packages?

- **Sales & Demos**: `sales@orbitlogs.io`
- **Technical Support**: `support@orbitlogs.io`
- **Commercial Documentation**: [https://docs.orbitlogs.io](https://docs.orbitlogs.io)

---

## ⚖️ License

OrbitLogs Core is licensed under the [Apache License, Version 2.0](LICENSE). Copyright © OrbitLogs Authors.
