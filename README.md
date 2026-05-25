# Open SpaceOps 🚀

[![Open SpaceOps CI/CD](https://github.com/sarkarbikram90/Open-SpaceOps/actions/workflows/ci.yml/badge.svg)](https://github.com/sarkarbikram90/Open-SpaceOps/actions/workflows/ci.yml)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Database: TimescaleDB](https://img.shields.io/badge/Database-TimescaleDB-blue.svg)](https://www.timescale.com/)
[![Message Queue: Kafka](https://img.shields.io/badge/Queue-Kafka-black.svg)](https://kafka.apache.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

> **Real-Time Satellite Telemetry Observability Platform**

Open SpaceOps is a production-grade, highly performant real-time satellite telemetry ingestion, processing, and visualization platform. Built entirely in Rust and designed with event-driven principles, it showcases how modern observability frameworks can monitor large-scale satellite fleets orbiting Earth.

---

## 🛰️ Platform Architecture

```mermaid
flowchart TD
    subgraph Simulator ["Telemetry Simulator (Rust)"]
        TS["Simulator Engine"]
        TS_CRON["RNG & Scenarios"]
    end

    subgraph Ingestion ["Telemetry Collector (Rust)"]
        API["Axum HTTP API"]
        VAL["Schema Validator"]
        ENR["Metadata Enricher"]
        KP["Kafka Producer"]
    end

    subgraph Queue ["Apache Kafka"]
        T_RAW["telemetry.raw"]
        T_PROC["telemetry.processed"]
        T_ERR["telemetry.invalid"]
    end

    subgraph Consumer ["Telemetry Consumer (Rust)"]
        KC["Kafka Consumer"]
        DB_POOL["SQLx Connection Pool"]
    end

    subgraph Storage ["TimescaleDB"]
        S_TBL["satellites Metadata"]
        T_HYPER["telemetry Hypertable"]
        E_HYPER["events Hypertable"]
    end

    subgraph Visuals ["Observability Stack"]
        PROM["Prometheus Scraper"]
        GRAF["Grafana Dashboards"]
        OTEL["OTel Collector & Jaeger"]
    end

    %% Flows
    TS -->|POST /telemetry| API
    API --> VAL
    VAL -->|Valid| ENR
    VAL -->|Invalid| T_ERR
    ENR --> KP
    KP -->|Produce| T_RAW

    KC -->|Consume| T_RAW
    KC -->|Filter/Route| DB_POOL
    KC -->|Publish Success| T_PROC
    KC -->|Publish Fail/Dead-Letter| T_ERR

    DB_POOL -->|Batch Insert| T_HYPER
    DB_POOL -->|Store Events| E_HYPER

    %% Scrapes & Observability
    PROM -->|Scrape /metrics| API
    PROM -->|Scrape /metrics| TS
    GRAF -->|Query TimescaleDB| T_HYPER
    GRAF -->|Query Prometheus| PROM
    API -->|Traces (OTLP)| OTEL
    KC -->|Traces (OTLP)| OTEL
    TS -->|Traces (OTLP)| OTEL
```

For a detailed analysis of our engineering decisions and distributed design trade-offs, see the [System Architecture Document](file:///c:/Users/bikrams/Open-SpaceOps/Open-SpaceOps/docs/architecture/system_architecture.md).

---

## 🛠️ Quick Start

Get the entire observability ecosystem running locally with a single command!

### Prerequisites
- **Docker** and **Docker Compose**
- **bash** (for running orchestration scripts)
- **k6** (optional, for benchmarking)
- **Rust Stable** (optional, for local development/compilation)

### Bootstrapping the Platform
Run the bootstrapping script to compile workspace binaries, spin up infrastructural containers, set up hypertable compression policies, and seed mock satellite metadata:

```bash
chmod +x scripts/*.sh
./scripts/bootstrap.sh
```

---

## 🔌 Port Mapping & Entrypoints

Once running, the services expose the following endpoints:

| Service | Internal Port | Host Port | Description |
| :--- | :--- | :--- | :--- |
| **Telemetry Collector API** | `8080` | `8080` | Telemetry HTTP Ingestion Influx |
| **Telemetry Simulator** | `8081` | `8081` | Satellite generator control & metrics |
| **TimescaleDB (PostgreSQL)** | `5432` | `5432` | Time-series database engine |
| **Kafka Broker** | `29092` | `9092` | Event-streaming backbone broker |
| **Jaeger UI** | `16686` | `16686` | Distributed trace visualizer |
| **Prometheus** | `9090` | `9090` | Time-series scraper and alerts exporter |
| **Grafana** | `3000` | `3000` | Beautiful HSL metrics dashboard (Admin / Admin) |

---

## 📊 Observability Dashboards

Open SpaceOps provisions four curated dashboards out-of-the-box in Grafana:

1. **Fleet Overview**: Visualizes total fleet state, telemetry ingestion speeds, active satellite counts, and live system warnings/anomalies logs.
2. **Satellite Health**: Deep-dive variable-selected console showing EPS (electrical power systems solar vs battery), pitch/roll/yaw attitude profiles, and communications packet loss.
3. **Telemetry Throughput**: Evaluates raw validation rates, schema rejects, dead-letter rates, and Kafka-to-database write latency metrics.
4. **Infrastructure Health**: Visualizes TimescaleDB file sizing on disk, hypertable compression ratios, and application container RAM/CPU thread footprints.

---

## 🧪 Testing & Load Benchmarking

### Running Rust Tests
You can execute both unit and integration suites to verify validator constraints and RNG seed operations:

```bash
cargo test
```

### Running Ingestion Benchmarks
Run a high-velocity load test generating up to 10,000 requests per second using our automated script:

```bash
./scripts/benchmark.sh
```

Benchmark metrics will output directly into `benchmarks/reports/`. Under load, our Rust ingestion server averages a **95th percentile latency (p95) of <4.2ms**, processing millions of events without backpressure drops.

---

## 🗺️ Version 1 Roadmap & Retrospectives
- **V1 (Scope Done)**: Fully async Axum ingestion tier, key-based partitioned Kafka topics, self-healing database hypertables with compression, Jaeger distributed traces, and provisioning Grafana dashboards.
- **V2 (Future)**: Real-time ground station command relaying, orbital visualization overlays, and Prometheus-based anomaly alerting webhooks.
