# Digital Twin Engine 👥

The **Digital Twin Engine** maintains synchronized in-memory virtual states (digital twins) for all active spacecraft within the OrbitLogs constellation.

## Architecture Role

- Consumes processed telemetry records from Kafka (`telemetry.processed`) to sync in-memory representations.
- Maintains **recursive degradation model** (solar efficiency, battery capacity fade, radiator thermal resistance) via Kalman-like low-pass filters that converge over thousands of samples.
- Performs predictive diagnostic calculations to forecast remaining life expectancy and recommend operational changes.
- Records simulated incidents and routes anomaly updates back to the mission timeline.
- Features a Failure Simulator API allowing operators to inject and clear artificial subsystem errors (e.g. ADCS drift, thermal overload, solar panel blockages) for operator training.
- Exposes **Prometheus metrics** for constellation-scale observability.
- Serves **Kubernetes health probes** (`/healthz`, `/readyz`) on a dedicated port.

## Module Structure

| Module | Description |
| :--- | :--- |
| `main.rs` | Bootstrap, wiring, graceful lifecycle |
| `config.rs` | Typed CLI/env configuration |
| `error.rs` | Structured `AppError` with `IntoResponse` |
| `metrics.rs` | Prometheus counters, gauges, histograms |
| `consumer.rs` | Kafka consumer loop with exponential backoff |
| `handlers.rs` | HTTP request handlers |
| `simulation.rs` | Health projection engine & `DigitalTwinState` model |
| `db.rs` | Typed database queries |

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Port** | `--port` | `PORT` | `8086` | Port the Axum HTTP server listens on |
| **Health Port** | `--health-port` | `HEALTH_PORT` | `8087` | Port for Kubernetes health probes |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **OTel Endpoint** | `--otel-endpoint` | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OpenTelemetry OTLP collector endpoint |
| **Processed Topic** | `--processed-topic` | `PROCESSED_TOPIC` | `telemetry.processed` | Kafka topic to consume processed telemetry from |
| **Timeline Topic** | `--timeline-topic` | `TIMELINE_EVENTS_TOPIC` | `timeline.events` | Target Kafka topic for timeline event notifications |
| **Group ID** | `--group-id` | `GROUP_ID` | `digital-twin-group` | Kafka consumer group ID |
| **API Key** | `--api-key` | `API_KEY` | None (Required) | Security authentication bearer token |
| **CORS Origin** | `--cors-allowed-origin` | `CORS_ALLOWED_ORIGIN` | `http://localhost:3000` | Allowed CORS request origin |

## API Endpoints

### Public Endpoints

- **`GET /api/twin/:satellite`**
  - Returns the current synchronized virtual state of the specified satellite.

- **`GET /api/twin/:satellite/health`**
  - Evaluates subsystem-specific health scores (power, thermal, ADCS, communications, propulsion, payload, flight computer), calculates expectancy durations, and yields recovery recommendations.

- **`GET /api/twin/:satellite/timeline`**
  - Fetches the most recent events registered for the specified satellite.
  - Query parameter: `?limit=N` (default: 50, max: 500)

- **`GET /api/twin/:satellite/simulated-failures`**
  - Lists all currently active simulated failures for a satellite.

- **`GET /metrics`**
  - Prometheus metrics endpoint.

### Authenticated Endpoints (Bearer Token Required)

- **`POST /api/twin/:satellite/simulate-failure`**
  - Injects an artificial failure mode.
  - Valid subsystems: `power`, `thermal`, `adcs`, `communications`, `propulsion`.
  - Request Body:
    ```json
    {
      "subsystem": "thermal"
    }
    ```

- **`DELETE /api/twin/:satellite/simulated-failures`**
  - Clears all active simulated failures for a satellite.

### Health Probes (Port 8087)

- **`GET /healthz`** — Liveness probe (always returns 200 when process is running).
- **`GET /readyz`** — Readiness probe (returns 200 when service is ready to accept traffic).

## Observability

### Prometheus Metrics

| Metric | Type | Labels | Description |
| :--- | :--- | :--- | :--- |
| `digital_twin_active_twins` | Gauge | — | Active satellite twin count |
| `digital_twin_telemetry_processed_total` | Counter | `satellite_id` | Messages consumed |
| `digital_twin_telemetry_deserialization_errors_total` | Gauge | — | Bad messages |
| `digital_twin_simulated_failures_active` | Gauge | `satellite_id`, `subsystem` | Active failure injections |
| `digital_twin_health_projection_duration_seconds` | Histogram | — | Projection compute latency |
| `digital_twin_api_request_duration_seconds` | Histogram | `method`, `path`, `status` | HTTP latency |
