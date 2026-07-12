# AI Driven Incident Commander 🤖

The **AI Driven Incident Commander** is a RAG (Retrieval-Augmented Generation) based diagnostics assistant designed to help flight operators analyze satellite telemetry anomalies, past command audits, and operational manuals to compile incident postmortems, perform root-cause analysis, and recommend recovery procedures.

## Architecture Role

- Integrates with external LLM APIs (Gemini/OpenAI) using the system's `http_client`.
- Integrates with TimescaleDB to retrieve recent telemetry and event logs to augment the LLM prompts.
- Employs local prompt/SQL injection sanitization (`guardrails.rs`) to prevent credential leakages.
- Serves as a fallback diagnostic rules engine (`runbooks.rs`) when external LLM APIs are offline.

## Configuration

The service can be configured using CLI flags or the corresponding environment variables:

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **HTTP Port** | `--port` | `PORT` | `8088` | Port the Axum HTTP server listens on |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **OTel Endpoint** | `--otel-exporter-otlp-endpoint` | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OpenTelemetry OTLP collector endpoint |
| **API Key** | `--api-key` | `API_KEY` | None (Required) | Security authentication bearer token |
| **CORS Origin** | `--cors-allowed-origin` | `CORS_ALLOWED_ORIGIN` | `http://localhost:3000` | Allowed CORS request origin |

## API Endpoints

- **`POST /api/chat`**
  - **Description**: Main interaction endpoint for conversational incident diagnostics.
  - **Authentication**: Bearer Token in `Authorization` header.
  - **Request Body**:
    ```json
    {
      "message": "What is wrong with SAT-0001?",
      "satellite_id": "SAT-0001"
    }
    ```
  - **Response**: Sanitized LLM diagnostic text and recommended runbooks.
