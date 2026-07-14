# Telemetry Ingestion API & Consumer 🔌

The **Telemetry Ingestion API & Consumer** (also known as `telemetry-collector`) is the main gateway for raw satellite telemetry streams entering the OrbitLogs platform.

## Architecture Role

- Exposes high-throughput HTTP endpoints (Port `8080`) to receive spacecraft data.
- Supports both **JSON payloads** and **CCSDS binary space packet frames**.
- Enforces strict schema validations and validates CRC-16 checksums on binary frames.
- Rejects corrupt or oversized payloads (Max 10KB), routing anomalies to the dead-letter queue (`telemetry.invalid`).
- Publishes validated streams to Apache Kafka (`telemetry.raw`).
- Features a **Tokio background consumer loop** that reads from `telemetry.raw`, updates anomaly parameters, and persists telemetry to TimescaleDB.
- Relies on W3C tracing contexts and manual async offset commits to guarantee at-least-once database storage.

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **HTTP Port** | `--http-port` | `HTTP_PORT` | `8080` | Port the HTTP ingestion server listens on |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **OTel Endpoint** | `--otel-exporter-otlp-endpoint` | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OpenTelemetry OTLP collector endpoint |
| **Raw Topic** | `--raw-topic` | `RAW_TOPIC` | `telemetry.raw` | Target topic for validated raw telemetry |
| **Processed Topic** | `--processed-topic` | `PROCESSED_TOPIC` | `telemetry.processed` | Target topic for fully processed/persisted events |
| **Invalid Topic** | `--invalid-topic` | `INVALID_TOPIC` | `telemetry.invalid` | Dead-letter queue (DLQ) Kafka topic |
| **Group ID** | `--group-id` | `GROUP_ID` | `telemetry-consumer-group` | Kafka consumer group ID |
| **API Key** | `--api-key` | `API_KEY` | None (Required) | Security authentication bearer token |
| **CORS Origin** | `--cors-allowed-origin` | `CORS_ALLOWED_ORIGIN` | `http://localhost:3000` | Allowed CORS request origin |

## Subcommands

Run the executable with specific roles:
- **`run-server`**: Run only the HTTP Axum API server.
- **`run-consumer`**: Run only the background Kafka processor.
- **`run-all`**: Run both server and consumer concurrently (default).
- **`run-migrations`**: Run TimescaleDB database schema migrations and exit.

## API Endpoints

- **`POST /telemetry`**
  - **Description**: Ingest a new telemetry payload (accepts JSON or binary stream).
- **`POST /telemetry/tles/ingest`**
  - **Description**: Ingest a batch of standard Two-Line Element (TLE) sets. Validates TLE integrity via SGP4 propagation sanity checks before archiving to TimescaleDB.
- **`GET /ws`**
  - **Description**: WebSocket endpoint for real-time telemetry streaming to dashboards.
- **`GET /health`**
  - **Description**: Returns combined health status of TimescaleDB and Kafka connections.
- **`GET /liveness`**
  - **Description**: Kubernetes liveness probe. Returns `200 OK` if the process is alive.
- **`GET /readiness`**
  - **Description**: Kubernetes readiness probe. Returns `200 OK` when database and Kafka connections are established.
- **`GET /metrics`**
  - **Description**: Exposes Prometheus metrics tracking ingestion rates and validation counts.

