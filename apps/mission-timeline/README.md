# Mission Timeline Service 📅

The **Mission Timeline Service** is an events-aggregator engine that gathers operational state shifts, automated anomalies, and telemetry errors into a centralized relational view.

## Architecture Role

- Consumes timeline updates (`timeline.events`) and system state alarms (`mission.health`) from Apache Kafka.
- Translates unstructured messages into standard event structures.
- Persists all event trails into TimescaleDB tables for operational queries.
- Exposes REST APIs (Port `8087`) to query, filter (by source, satellite, severity), and append new timeline updates.

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Port** | `--port` | `PORT` | `8087` | Port the Axum HTTP server listens on |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **OTel Endpoint** | `--otel-exporter-otlp-endpoint` | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OpenTelemetry OTLP collector endpoint |
| **Timeline Topic** | `--timeline-events-topic` | `TIMELINE_EVENTS_TOPIC` | `timeline.events` | Kafka topic to consume events from |
| **Health Topic** | `--health-topic` | `HEALTH_TOPIC` | `mission.health` | Kafka topic to consume health status changes from |
| **Group ID** | `--group-id` | `GROUP_ID` | `mission-timeline-group` | Kafka consumer group ID |
| **API Key** | `--api-key` | `API_KEY` | None (Required) | Security authentication bearer token |
| **CORS Origin** | `--cors-allowed-origin` | `CORS_ALLOWED_ORIGIN` | `http://localhost:3000` | Allowed CORS request origin |

## API Endpoints

- **`GET /api/timeline`**
  - **Description**: Query historical events.
  - **Query Parameters**:
    - `satellite_id`: Filter by spacecraft identifier.
    - `source`: Filter by source type (`TELEMETRY`, `COMMAND`, `ALERT`, `INCIDENT`, `GROUND_STATION`, `DIGITAL_TWIN`).
    - `severity`: Filter by severity (`INFO`, `WARNING`, `CRITICAL`).
    - `limit`: Number of events to return (default 100).
- **`POST /api/timeline`**
  - **Description**: Inject a custom timeline event.
  - **Authentication**: Bearer Token required.
  - **Request Body**:
    ```json
    {
      "timestamp": "2026-07-11T20:00:00Z",
      "event_id": "00000000-0000-0000-0000-000000000000",
      "source": "COMMAND",
      "satellite_id": "SAT-0001",
      "event_type": "ORBIT_ADJUSTMENT",
      "severity": "INFO",
      "message": "Operator triggered propellant burn",
      "details": null
    }
    ```
