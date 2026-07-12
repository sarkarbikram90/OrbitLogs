# Space OTel Collector 📡

The **Space OTel Collector** is an observability mapping daemon that bridges system-wide events and time-series telemetry streams from Kafka topics into structured Prometheus metrics and OpenTelemetry traces.

## Architecture Role

- Consumes telemetry and status streams from multiple Kafka topics:
  - `telemetry.processed`
  - `command.events`
  - `groundstation.health`
  - `mission.health`
- Extracts trace context parameters propagated in message headers to build end-to-end telemetry/command traces.
- Exports application traces to an external OTLP endpoint (e.g. Jaeger).
- Exposes Prometheus metrics via `/metrics` (Port `8091`), leveraging static concurrent maps (`DashMap`) to track spacecraft and ground station metrics without lock contention.

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **OTel Endpoint** | `--otel-exporter-otlp-endpoint` | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OpenTelemetry OTLP collector endpoint |
| **Processed Topic** | `--processed-topic` | `PROCESSED_TOPIC` | `telemetry.processed` | Kafka topic containing processed telemetry |
| **Command Topic** | `--command-events-topic` | `COMMAND_EVENTS_TOPIC` | `command.events` | Kafka topic containing command actions |
| **GS Health Topic** | `--gs-health-topic` | `GS_HEALTH_TOPIC` | `groundstation.health` | Kafka topic containing ground station health |
| **Health Topic** | `--health-topic` | `HEALTH_TOPIC` | `mission.health` | Kafka topic containing mission health metrics |
| **Metrics Port** | `--metrics-port` | `METRICS_PORT` | `8091` | Port for the Prometheus metrics scrape target |
| **Group ID** | `--group-id` | `GROUP_ID` | `space-otel-collector-group` | Kafka consumer group ID |

## Metrics API

- **`GET /metrics`** (Port `8091`)
  - **Description**: Exposes OpenTelemetry metrics for the satellite constellation status and command executions.
