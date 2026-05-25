# Open SpaceOps: Telemetry Collector Design

This document details the design specifications, ingestion routes, schema enforcement strategies, and validation layers of the **Telemetry Collector & Consumer** service.

---

## 1. System Goals & Responsibilities

The Telemetry Collector is a dual-purpose high-performance server and daemon designed to:
- Ingest real-time telemetry packets over high-throughput HTTP endpoints.
- Enforce strict JSON payload limits, data structures, and range boundaries.
- Quarantine malformed, invalid, or dangerous payloads immediately to a Dead Letter Topic.
- Decouple database persistence via Kafka to keep average API response latencies **under 5ms**.

---

## 2. Ingestion Routing & Endpoints

The server exposes three highly optimized Axum endpoints:

### `POST /telemetry`
Accepts JSON telemetry streams.
- **Processing Stage 1: Struct Deserialization**: Ensures types strictly match the expected telemetry model schema.
- **Processing Stage 2: Bounds Checking**: Evaluates value limits (e.g. CPU $\in [0, 100]\%$, battery voltage $\ge 0.0\text{V}$).
- **Processing Stage 3: Kafka Produce**: Publishes valid payloads to `telemetry.raw` partitioned by `satellite_id` for in-order downstream execution.
- **Returns**: HTTP `202 Accepted` with a unique transaction `event_id`.

### `GET /health`
Deep system health evaluation:
- Performs real-time pings to TimescaleDB and checks active Kafka broker connections.
- Returns `200 OK` if all services are online; returns `503 Service Unavailable` on database or broker failures.

### `GET /metrics`
Exposes system runtime and database latency statistics formatted for Prometheus scraping.

---

## 3. Validation & In-Flight Quarantine (DLQ)

```
        [ POST /telemetry ]
                 │
                 ▼
     [ Multi-Stage Validator ]
                 │
        ┌────────┴────────┐
     (Valid)           (Invalid)
        │                 │
        ▼                 ▼
   [ Kafka raw ]     [ JSON DLQ Envelope ]
                          │
                          ▼
                 [ Kafka invalid DLQ ]
```

When a payload fails validation:
1. An error log is recorded.
2. The payload is enveloped in a DLQ JSON wrapper detailing the specific reason for validation rejection and the receipt timestamp.
3. The enveloped reject is published to the `telemetry.invalid` dead-letter topic.
4. The client receives an HTTP `400 Bad Request` with a JSON list of detailed validation failure reasons.
