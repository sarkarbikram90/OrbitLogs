# Mission Health Engine 💚

The **Mission Health Engine** evaluates real-time telemetry events and ground link attributes to compute consolidated health statuses and anomaly risks for the active satellite constellation.

## Architecture Role

- Consumes processed satellite telemetry (`telemetry.processed`) and ground facility link trends (`groundstation.health`).
- Evaluates rule checks (battery voltage degradation, thermal limits, guidance sensor stability, and communications packet loss).
- Generates composite health ratings: `GREEN` (Nominal), `YELLOW` (Degraded), or `RED` (Critical/Failed).
- Writes health evaluations and anomaly logs to TimescaleDB.
- Publishes health updates to the `mission.health` Kafka topic.
- Serves Prometheus tracking metrics on port `8082`.

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **Processed Topic** | `--processed-topic` | `PROCESSED_TOPIC` | `telemetry.processed` | Kafka topic containing processed telemetry |
| **Health Topic** | `--health-topic` | `HEALTH_TOPIC` | `mission.health` | Target Kafka topic for health assessments |
| **GS Health Topic** | `--gs-health-topic` | `GS_HEALTH_TOPIC` | `groundstation.health` | Kafka topic containing ground station health |
| **DLQ Topic** | `--dlq-topic` | `DLQ_TOPIC` | `telemetry.invalid` | Kafka topic for routing error/invalid events |
| **Group ID** | `--group-id` | `GROUP_ID` | `mission-health-group` | Kafka consumer group ID |
| **Metrics Port** | `--metrics-port` | `METRICS_PORT` | `8082` | Port for the Prometheus metrics scrape target |

## Metrics API

- **`GET /metrics`** (Port `8082`)
  - **Description**: Exposes Prometheus metrics tracking satellite health ratings and active anomaly counts.
