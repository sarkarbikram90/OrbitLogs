# Orbit Operations Service 🌌

The **Orbit Operations Service** is the central flight dynamics, conjunction assessment, and orbital mechanics calculations coordinator for OrbitLogs.

## Architecture Role

- Performs **SGP4 Orbit Propagation** using parsed satellite TLEs, computing latitude, longitude, altitude, and velocity.
- Runs background **Space Situational Awareness (SSA)** conjunction assessments (miss distance, radial/intrack/crosstrack margins, collision probabilities).
- Models physical **Flight Dynamics** trends, including altitude decay calculations and maneuver requirements.
- Coordinates **Ground Station Contacts** scheduling, elevations, and liveness.
- Implements the **Operator Training Manager** simulator to execute incident drills and acknowledge responses.
- Generates automated **Incident Postmortems** by querying historical logs from database hypertables.
- Maps satellite dependencies in a topology graph (propagation paths and failure cascading analyses).

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Port** | `--port` | `PORT` | `8095` | Port the Axum HTTP server listens on |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **OTel Endpoint** | `--otel-exporter-otlp-endpoint` | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OpenTelemetry OTLP collector endpoint |
| **Processed Topic** | `--processed-topic` | `PROCESSED_TOPIC` | `telemetry.processed` | Kafka topic containing processed telemetry |
| **Timeline Topic** | `--timeline-topic` | `TIMELINE_TOPIC` | `timeline.events` | Target Kafka topic for timeline entries |
| **Contacts Topic** | `--contacts-topic` | `CONTACTS_TOPIC` | `groundstation.contacts` | Target Kafka topic for contact updates |
| **Group ID** | `--group-id` | `GROUP_ID` | `orbit-ops-group` | Kafka consumer group ID |

## Core APIs

- **`GET /api/ssa/:satellite/conjunctions`**: Yield active collision alert risks for the target satellite.
- **`GET /api/ssa/debris`**: Return catalog of simulated orbital debris particles.
- **`GET /api/flight-dynamics/:satellite`**: Current physical velocity, period, and altitude decay profile.
- **`POST /api/twin/:satellite/command`**: Dispatches command sequences to the twin model.
- **`POST /api/training/start`**: Start an operator training drill incident.
- **`GET /api/postmortem/:satellite`**: Run RAG diagnostics and compile an automated markdown postmortem of recent anomalies.
- **`GET /api/analytics`**: Return telemetry, contact, and command success rates.
