# OrbitLogs: Telemetry Ingestion Design & Architecture
### Version: v4.5.0 (Latest)

This document details the design specifications, ingestion routing, schema validation, and quarantine handling logic of the **Telemetry Ingestion API** (`telemetry-collector`).

---

## 1. System Responsibilities

The Telemetry Ingestion tier is the gateway for all space-to-ground downlink feeds. Its goals include:
*   **Low-Latency Reception**: Handle bursty telemetry transmissions during satellite passes, maintaining response times **under 5ms**.
*   **Serialization Integrity**: Ingest standard JSON payloads or CCSDS binary frames, verifying binary integrity using CRC-16 checks.
*   **Schema Safety**: Verify data schema version compatibility using `space-schema-registry` rules.
*   **Defensive Quarantine**: Instantly route bad data to a Dead-Letter Queue (DLQ) to protect downstream database consumers.

---

## 2. Ingestion Routing & Protocol Handling

The server listens on `:8080` and exposes the following endpoints:

### `POST /telemetry`
Accepts downlink telemetry streams.
*   **JSON Ingestion**: Enforces strict parsing checks using Serde's `#[serde(deny_unknown_fields)]` to block unmapped parameters.
*   **CCSDS Binary Ingestion**: When `Content-Type: application/octet-stream` is set, the server decodes binary packets using `orbitlogs-packets` and validates the **CRC-16-CCITT** checksum.
*   **CORS & Auth**: Enforces JWT Bearer authentication and locks origins to the `CORS_ALLOWED_ORIGIN` env configuration.
*   **Rate Limiting**: Employs an IP-partitioned token-bucket rate limiter via `DashMap` to prevent request starvation under load.
*   **Returns**: HTTP `202 Accepted` with a unique transaction UUID.

### `POST /telemetry/tles/ingest`
Accepts batches of standard Two-Line Element (TLE) sets.
*   Validates TLE integrity at the edge via SGP4 propagation sanity checks.
*   Archives valid TLE records to the TimescaleDB `tle_records` table.
*   Returns HTTP `200 OK` with a count of successfully ingested records.

### `GET /health`
Validates connection pools:
*   Performs real-time checks on TimescaleDB pools and Kafka broker connectivity.
*   Exposes liveness and readiness states to Kubernetes probes.

### `GET /liveness`
Kubernetes liveness probe endpoint. Returns `200 OK` if the process is alive.

### `GET /readiness`
Kubernetes readiness probe endpoint. Returns `200 OK` when database and Kafka connections are established and verified.

### `GET /metrics`
Exposes Prometheus-formatted counters and histograms for request validation speeds and database insertion latencies.

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
1.  An error log is recorded.
2.  The payload is enveloped in a DLQ JSON wrapper detailing the specific reason for validation rejection and the receipt timestamp.
3.  The enveloped reject is published to the `telemetry.invalid` dead-letter topic.
4.  The client receives an HTTP `400 Bad Request` with a JSON list of detailed validation failure reasons.
