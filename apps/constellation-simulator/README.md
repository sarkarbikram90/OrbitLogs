# Constellation Simulator 🛰️

The **Constellation Simulator** is a high-performance simulation engine capable of generating concurrent telemetry and status profiles for large fleets of satellites (up to 5,000+ nodes).

## Architecture Role

- Simulates physical spacecraft subsystem state progressions (orbital decay, thermal oscillation, battery aging, and propellant depletion).
- Formats simulated data into telemetry records.
- Encodes records into CCSDS primary/secondary packets.
- Dispatches telemetry records to the raw telemetry topic in Apache Kafka.

## Configuration

Configure the simulator via the following command-line flags or environment variables:

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Fleet Size** | `--fleet-size` | `FLEET_SIZE` | `500` | Number of concurrent satellite states to simulate |
| **Tick Interval** | `--tick-interval-ms` | `TICK_INTERVAL_MS` | `5000` | State update frequency in milliseconds |
| **Raw Telemetry Topic**| `--raw-topic` | `RAW_TOPIC` | `telemetry.raw` | Target Kafka topic for dispatching raw telemetry |

## Executable

To run the simulator locally:
```bash
cargo run --bin constellation-simulator -- --fleet-size 100 --tick-interval-ms 2000
```
