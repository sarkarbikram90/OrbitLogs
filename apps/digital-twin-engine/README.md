# Digital Twin Engine 👥

The **Digital Twin Engine** maintains synchronized in-memory virtual states (digital twins) for all active spacecraft within the OrbitLogs constellation.

## Architecture Role

- Consumes processed telemetry records from Kafka (`telemetry.processed`) to sync in-memory representations.
- Performs predictive diagnostic calculations to forecast remaining life expectancy and recommend operational changes.
- Records simulated incidents and routes anomaly updates back to the mission timeline.
- Features a Failure Simulator API allowing operators to inject artificial subsystem errors (e.g. ADCS drift, thermal overload, solar panel blockages) for operator training.

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Port** | `--port` | `PORT` | `8086` | Port the Axum HTTP server listens on |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **OTel Endpoint** | `--otel-exporter-otlp-endpoint` | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OpenTelemetry OTLP collector endpoint |
| **Processed Topic** | `--processed-topic` | `PROCESSED_TOPIC` | `telemetry.processed` | Kafka topic to consume processed telemetry from |
| **Timeline Topic** | `--timeline-topic` | `TIMELINE_EVENTS_TOPIC` | `timeline.events` | Target Kafka topic for timeline event notifications |
| **Group ID** | `--group-id` | `GROUP_ID` | `digital-twin-group` | Kafka consumer group ID |
| **API Key** | `--api-key` | `API_KEY` | None (Required) | Security authentication bearer token |
| **CORS Origin** | `--cors-allowed-origin` | `CORS_ALLOWED_ORIGIN` | `http://localhost:3000` | Allowed CORS request origin |

## API Endpoints

- **`GET /api/twin/:satellite`**
  - **Description**: Returns the current synchronized virtual state of the specified satellite.
- **`GET /api/twin/:satellite/health`**
  - **Description**: Evaluates subsystem-specific health scores, calculates expectancy durations, and yields recovery recommendations.
- **`GET /api/twin/:satellite/timeline`**
  - **Description**: Fetches the last 50 events registered for the specified satellite.
- **`POST /api/twin/:satellite/simulate-failure`**
  - **Description**: Injects an artificial failure mode (`power`, `thermal`, `adcs`, `communications`, `propulsion`).
  - **Authentication**: Bearer Token required.
  - **Request Body**:
    ```json
    {
      "subsystem": "thermal"
    }
    ```
