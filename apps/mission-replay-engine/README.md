# Mission Replay Engine 🔄

The **Mission Replay Engine** provides time-series data playback capabilities, allowing flight operators to review historical telemetry passes and audit trails at up to 100x speeds for diagnostics and postmortems.

## Architecture Role

- Connects directly to TimescaleDB to run range queries on hypertable telemetry records.
- Spawns in-memory Replay Sessions to track current play status, seek coordinates, and speeds.
- Streams historical data back to UI dashboard panels using Server-Sent Events (SSE) (Port `8084`).
- Schedules background cleanup jobs to expire inactive playback sessions.

## Configuration

| Parameter | CLI Flag | Env Variable | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres/TimescaleDB connection string |
| **Port** | `--port` | `PORT` | `8084` | Port the Axum HTTP server listens on |
| **API Key** | `--api-key` | `API_KEY` | None (Required) | Security authentication bearer token |
| **CORS Origin** | `--cors-allowed-origin` | `CORS_ALLOWED_ORIGIN` | `http://localhost:3000` | Allowed CORS request origin |

## API Endpoints

- **`POST /api/replays`**
  - **Description**: Initialise a new replay session for a given timeframe and satellite.
  - **Request Body**:
    ```json
    {
      "satellite_id": "SAT-0001",
      "start_time": "2026-07-04T00:00:00Z",
      "end_time": "2026-07-04T02:00:00Z"
    }
    ```
- **`GET /api/replays/:id`**
  - **Description**: Returns session status, time windows, and speed ratio.
- **`POST /api/replays/:id/control`**
  - **Description**: Perform operations such as play, pause, seek, or change playback speed.
  - **Request Body**:
    ```json
    {
      "action": "seek",
      "speed": 10.0,
      "seek_time": "2026-07-04T01:00:00Z"
    }
    ```
- **`GET /api/replays/:id/stream`**
  - **Description**: Connects an SSE stream that yields telemetry entries dynamically.
- **`GET /api/satellites`**
  - **Description**: Lists all satellites in the database.
- **`GET /api/ground-stations`**
  - **Description**: Lists all ground stations in the database.
