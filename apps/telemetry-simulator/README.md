# Telemetry Simulator 🚀

The **Telemetry Simulator** models realistic physical behaviours, subsystem variables, and environmental conditions of a spacecraft fleet, sending active telemetry packets to the ingestion pipeline.

## Architecture Role

- Models orbital decay, battery aging, thermal heat sinks, and telemetry formats.
- Supports scenario simulations:
  - `normal`: Standard operations.
  - `battery-depletion`: EPS failure cascades.
  - `thermal-spike`: Radiator blockages causing ADCS degradation.
  - `fuel-leakage`: Propulsion capacity warning tests.
  - `gs-outage` / `weather-modulation`: Polar communications interruptions.
- Dispatches telemetry records via HTTP POST requests to `telemetry-collector`.
- Consumes ground link attributes from Kafka to dynamically adjust signal attributes.
- Serves Prometheus performance tracking metrics on port `8081`.

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Collector URL** | `--collector-url` | `COLLECTOR_URL` | `http://localhost:8080/telemetry` | Destination HTTP telemetry ingestion endpoint |
| **Fleet Size** | `--fleet-size` | `SIM_FLEET_SIZE` | `5` | Number of concurrent satellite simulator models |
| **Frequency** | `--frequency-ms` | `SIM_FREQUENCY_MS` | `1000` | Telemetry packet emission rate in milliseconds |
| **Scenario** | `--scenario` | `SIM_SCENARIO` | `normal` | Simulation scenario selection |
| **Deterministic** | `--deterministic` | `SIM_DETERMINISTIC` | `false` | Enable deterministic runs for debugging |
| **Seed** | `--seed` | `SIM_SEED` | `42` | RNG seed for deterministic runs |
| **Metrics Port** | `--metrics-port` | `METRICS_PORT` | `8081` | Port for the Prometheus metrics scrape target |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Health Topic** | `--health-topic` | `HEALTH_TOPIC` | `groundstation.health` | Kafka topic containing ground station link states |
| **API Key** | `--api-key` | `API_KEY` | None (Required) | Security authentication bearer token |

## Metrics API

- **`GET /metrics`** (Port `8081`)
  - **Description**: Exposes Prometheus metrics tracking telemetry generation rates and collector success ratios.
