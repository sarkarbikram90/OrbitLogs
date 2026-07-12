# Mission Control UI 🛰️ RAG Dashboard

The **Mission Control UI** is a React, TypeScript, and Vite-based real-time dashboard operations panel for monitoring, troubleshooting, and analyzing the OrbitLogs satellite constellation.

## Core Features

- **3D Earth Globe Visualizer**: Custom WebGL Three.js projection mapping satellite orbits and a 250-particle space debris cloud in real-time.
- **AI incident Commander Chat**: Chat panel leveraging LLM RAG to search manuals, retrieve telemetry details, and auto-diagnose issues.
- **Space Traffic Management Panel**: Risk Scorecard, conjunction events log, and 1-click **Authorize Collision Avoidance Burn** burn commands.
- **Real-Time WebSockets**: Live telemetry streaming directly from `telemetry-collector`.
- **Historical Timeline scrubbing**: SSE playback controls (seeks, play, pause, up to 100x playback speed).

## Configuration

The frontend references the following backend environment endpoints during build-time (in `.env` or defined environment):

- `VITE_API_URL`: Base HTTP address of backend microservices.
- `VITE_WS_URL`: WebSocket endpoint for telemetry ingestion.

## Build and Run

To start the UI locally:
```bash
# Install dependencies
npm install

# Start local dev server
npm run dev
```

To build for production distribution (uses Vite code splitting to segment node modules):
```bash
npm run build
```
