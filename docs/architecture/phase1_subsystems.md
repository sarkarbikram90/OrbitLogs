# Phase 1: Spacecraft Subsystem Telemetry Architecture & Design

This document details the architecture, design, failure modes, operational runbooks, and troubleshooting guides for the spacecraft subsystem telemetry system implemented in Phase 1.

---

## 1. System Architecture

The Open SpaceOps system has been refactored to support concurrent, independent telemetry streams from seven spacecraft subsystems per satellite: Power, Thermal, Communications, Propulsion, ADCS, Payload, and Flight Computer. 

### Component Diagram

```mermaid
graph TD
    subgraph Spacecraft ["Spacecraft Simulator"]
        PWR[Power Subsystem]
        THM[Thermal Subsystem]
        COM[Communications Subsystem]
        PRP[Propulsion Subsystem]
        ADC[ADCS Subsystem]
        PAY[Payload Subsystem]
        FC[Flight Computer]
        SIM_STATE[(Shared physical state)]
    end

    subgraph GroundSegment ["Ground Segment Collector"]
        COLL_SRV[HTTP Ingestion Server]
        KAFKA[(Kafka Event Bus)]
        CONS_PROC[Consumer Processor]
        CONS_CACHE[(In-Memory Cache)]
    end

    subgraph DataPlatform ["Data Platform"]
        DB[(TimescaleDB Hypertable)]
        GF[Grafana Dashboard]
    end

    PWR -- read/write --> SIM_STATE
    THM -- read/write --> SIM_STATE
    COM -- read/write --> SIM_STATE
    PRP -- read/write --> SIM_STATE
    ADC -- read/write --> SIM_STATE
    PAY -- read/write --> SIM_STATE
    FC -- read/write --> SIM_STATE

    PWR -- JSON Payload --> COLL_SRV
    THM -- JSON Payload --> COLL_SRV
    COM -- JSON Payload --> COLL_SRV
    PRP -- JSON Payload --> COLL_SRV
    ADC -- JSON Payload --> COLL_SRV
    PAY -- JSON Payload --> COLL_SRV
    FC -- JSON Payload --> COLL_SRV

    COLL_SRV -- telemetry.raw --> KAFKA
    KAFKA -- read partition by sat_id --> CONS_PROC
    CONS_PROC -- read/write --> CONS_CACHE
    CONS_PROC -- SELECT last record (fallback) --> DB
    CONS_PROC -- INSERT merged record --> DB
    GF -- query view/table --> DB
```

---

## 2. Telemetry Flow & Merging Logic

To support backward compatibility with existing dashboards that query a single flat table (`telemetry`), the collector's consumer runs a stateful merge logic. Since Kafka partitioning guarantees that all events for a specific `satellite_id` are processed sequentially by the same consumer instance, the consumer uses a fast in-memory cache of the latest satellite state, falling back to a database lookup on a cache miss.

### Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    participant Simulator as Subsystem Loop (Simulator)
    participant Collector as HTTP Ingester (Collector)
    participant Kafka as Kafka (telemetry.raw)
    participant Consumer as Consumer Processor
    participant Cache as In-Memory Cache
    participant DB as TimescaleDB

    Simulator->>Simulator: MutexLock & Update physical state
    Simulator->>Simulator: Generate sparse record (Option<T>)
    Simulator->>Collector: POST /telemetry
    Collector->>Collector: Validate (satellite_id, present fields)
    Collector->>Kafka: Publish payload (Key: satellite_id)
    Collector-->>Simulator: 202 Accepted
    Kafka->>Consumer: Consume record
    Consumer->>Cache: Lookup last-known record for satellite_id
    alt Cache Hit
        Cache-->>Consumer: Return last-known state
    else Cache Miss
        Consumer->>DB: Query latest record from telemetry table
        alt Record Exists
            DB-->>Consumer: Return record
        else No Record
            Consumer-->>Consumer: Instantiate default template
        end
    end
    Consumer->>Consumer: Overwrite fields present in incoming stream
    Consumer->>Cache: Write merged record to cache
    Consumer->>DB: INSERT fully populated merged record
    Consumer->>Consumer: Trigger anomaly check (on updated fields only)
```

---

## 3. Database Schema Design (ER Diagram)

The database schema is updated to add correlation keys (`subsystem_id`, `mission_id`), introduce Payload subsystem metrics, and drop `NOT NULL` constraints on metrics columns to support both unified and sparse telemetry.

```mermaid
erDiagram
    satellites {
        VARCHAR(50) id PK
        VARCHAR(100) name
        VARCHAR(50) model
        TIMESTAMP launch_date
        VARCHAR(20) status
        TIMESTAMP created_at
    }

    telemetry {
        TIMESTAMP timestamp PK
        VARCHAR(50) satellite_id FK
        VARCHAR(50) subsystem_id
        VARCHAR(50) mission_id
        DOUBLE_PRECISION battery_voltage
        DOUBLE_PRECISION battery_current
        DOUBLE_PRECISION solar_panel_output
        REAL cpu_usage
        REAL memory_usage
        DOUBLE_PRECISION temperature
        REAL orientation_pitch
        REAL orientation_roll
        REAL orientation_yaw
        DOUBLE_PRECISION velocity
        DOUBLE_PRECISION altitude
        REAL fuel_remaining
        REAL communication_latency
        REAL packet_loss
        REAL signal_strength
        DOUBLE_PRECISION payload_power_draw
        BOOLEAN payload_active
        UUID event_id
    }

    events {
        TIMESTAMP timestamp PK
        VARCHAR(50) satellite_id FK
        VARCHAR(50) event_type
        VARCHAR(20) severity
        TEXT message
        JSONB details
    }

    satellites ||--o{ telemetry : "emits"
    satellites ||--o{ events : "logs"
```

---

## 4. Failure Modes & Side-Effect Propagation

Real spacecraft subsystems are physically coupled. The simulator models these dependencies dynamically:

| Trigger Scenario | Cause (Subsystem) | Immediate Metric Shift | Propagated Side Effect |
| :--- | :--- | :--- | :--- |
| **ThermalSpike** | Thermal | Temperature spikes to >80.0°C | **Flight Computer:** CPU throttles (forces high usage >90%). |
| | | Temperature spikes to >90.0°C | **Communications:** Latency increases (+500ms) and packet loss rises (+10%). |
| | | Temperature spikes to >100.0°C | **Power:** Solar panel output drops by 50% due to thermal stress. |
| **BatteryDegradation** | Power | Battery voltage drops below 11.5V | **Payload:** Payload shuts down (active = false, draw = 0W). |
| | | Battery voltage drops below 11.0V | **ADCS:** Attitude control loops degrade, causing pitch/roll/yaw drift (+/- 15°). |
| | | Battery voltage drops below 10.5V | **Communications:** Signal strength drops (-15 dBm), packet loss rises. |
| **FuelLeakage** | Propulsion | Escaping gas thrust | **ADCS:** Attitude spins out of control (+/- 20° pitch/roll/yaw fluctuations). |
| | | Fuel reaches 0.0% | **Propulsion:** Satellite suffers orbital decay (velocity drops and altitude drops by 0.5 km/tick). |

---

## 5. Operational Runbook

### Deployment Diagram (Docker Compose)

```mermaid
graph TD
    timescaledb[timescaledb:5432]
    kafka[kafka:9092]
    collector[telemetry-collector:8080]
    simulator[telemetry-simulator:8081]
    grafana[grafana:3000]

    simulator --> collector
    collector --> kafka
    collector --> timescaledb
    grafana --> timescaledb
```

### Launching the Stack

To build and run the services in a clean, stateful local environment:

```powershell
# Navigate to deployment folder
cd deployments/docker-compose

# Re-build and run containers
docker-compose down -v
docker-compose up --build -d

# Verify containers are running
docker-compose ps
```

---

## 6. Troubleshooting Guide

### Issue 1: High Latency or Database Bottlenecks
* **Symptoms**: Kafka consumer lag increases on `telemetry.raw` or database insert latency metrics spike.
* **Diagnosis**: Check the database connection pool settings and execution times of `insert_telemetry` via Prometheus metrics (`database_insert_latency` histogram).
* **Mitigation**: Ensure the composite index `idx_telemetry_subsystem_mission` and hypertable compression policies are successfully applied. Confirm TimescaleDB has adequate memory.

### Issue 2: Flapping or Duplicate Anomalies in Events Log
* **Symptoms**: Multiple identical `ANOMALY_BATTERY_DEGRADATION` or `ANOMALY_THERMAL_SPIKE` events are logged within a few seconds.
* **Diagnosis**: Telemetry record merging carrying forward old values can trigger alerts multiple times if not filtered by incoming subsystem.
* **Mitigation**: Verify that `processor.rs` only performs anomaly checks on fields explicitly present in the incoming `TelemetryRecord` rather than checking the merged record.
