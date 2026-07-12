# Ground Station Simulator 📡

The **Ground Station Simulator** models the physical, environmental, and network characteristics of the low-altitude satellite link communications layer.

## Architecture Role

- Simulates status, backhaul network metrics, and antenna parameters for four polar ground facilities:
  1. Svalbard (Norway) — `GS-0001`
  2. Troll (Antarctica) — `GS-0002`
  3. Kiruna (Sweden) — `GS-0003`
  4. McMurdo (Antarctica) — `GS-0004`
- Modulates signal conditions using weather configurations (storms, clear sky), antenna servo mechanical faults, atmospheric noise, and network packet loss.
- Stores simulated ground segment status histories directly in the database.
- Publishes Ground Station Health updates to the `groundstation.health` topic in Apache Kafka.
- Serves Prometheus metrics exposing antenna performance ratios and local outage statistics.

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **Health Topic** | `--health-topic` | `HEALTH_TOPIC` | `groundstation.health` | Target Kafka topic for ground station status updates |
| **Metrics Port** | `--metrics-port` | `METRICS_PORT` | `8083` | Port for the Prometheus metrics scrape target |
| **Sim Frequency** | `--sim-frequency-sec` | `SIM_FREQUENCY_SEC` | `2` | State updates interval in seconds |

## Metrics API

- **`GET /metrics`** (Port `8083`)
  - **Description**: Exposes Prometheus metrics tracking antenna capabilities and station status checks.
