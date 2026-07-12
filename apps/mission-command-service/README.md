# Mission Command Service 🔌

The **Mission Command Service** is the central gateway for flight operators to submit, authorize, queue, and dispatch control commands to active satellites in the constellation.

## Architecture Role

- Exposes endpoints to manage the lifecycle of satellite commands: `CREATED` -> `APPROVED` -> `QUEUED` -> `TRANSMITTED` -> `RECEIVED` -> `VERIFIED`/`FAILED`.
- Enforces Role-Based Access Control (RBAC):
  - **Operator**: Can create and queue commands for dispatch.
  - **Flight Director**: Gated approval role required to authorize critical actions (e.g. maneuvers, safety state changes).
- Prevents brute-force access via IP-based login rate limiters.
- Signs and verifies cryptographic tokens (JWT).
- Publishes command updates to Kafka (`command.events`) and registers audit trail events (`timeline.events`).

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Port** | `--port` | `PORT` | `8085` | Port the Axum HTTP server listens on |
| **Kafka Brokers** | `--kafka-brokers` | `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker endpoints |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **OTel Endpoint** | `--otel-exporter-otlp-endpoint` | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OpenTelemetry OTLP collector endpoint |
| **Command Topic** | `--command-events-topic` | `COMMAND_EVENTS_TOPIC` | `command.events` | Target Kafka topic for command events |
| **Timeline Topic** | `--timeline-events-topic` | `TIMELINE_EVENTS_TOPIC` | `timeline.events` | Target Kafka topic for audit/timeline entries |
| **API Key** | `--api-key` | `API_KEY` | None (Required) | Security authentication bearer token |
| **JWT Secret** | `--jwt-secret` | `JWT_SECRET` | Ephemeral UUID | Secret key used for signing JWTs |
| **CORS Origin** | `--cors-allowed-origin` | `CORS_ALLOWED_ORIGIN` | `http://localhost:3000` | Allowed CORS request origin |

## API Endpoints

- **`POST /api/auth/login`**
  - **Description**: Authenticate operator credentials and obtain a JWT bearer token.
- **`GET /api/commands`**
  - **Description**: List historical command records with pagination, filtering, and ordering.
- **`POST /api/commands`**
  - **Description**: Submit a new command into the pipeline (`Operator` role required).
- **`POST /api/commands/:id/approve`**
  - **Description**: Approve a command, transitioning status to `APPROVED` (`Flight Director` role required).
- **`POST /api/commands/:id/queue`**
  - **Description**: Transition status to `QUEUED` and stage it for signal propagation.
- **`POST /api/commands/:id/execute`**
  - **Description**: Dispatches command over simulated ground links and publishes events.
