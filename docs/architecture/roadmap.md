# Open SpaceOps Platform Roadmap

This roadmap outlines the multi-phase evolution of the Open SpaceOps Mission Operations Platform.

## Roadmap Status

- `[x]` **Phase 1: Spacecraft Subsystem Architecture** (Completed)
  - [x] Design spacecraft subsystem modeling
  - [x] Implement database migrations for subsystem fields
  - [x] Implement subsystem telemetry generation in simulator
  - [x] Implement concurrent loops for independent streams
  - [x] Implement stateful merging & caching in collector
  - [x] Add unit and integration tests
  - [x] Generate sequence & ER diagrams
- `[x]` **Phase 2: Mission Health Engine** (Completed)
  - [x] Convert raw telemetry streams to health statuses (Green/Yellow/Red)
  - [x] Publish results to `mission.health` topic
  - [x] Create Grafana status dashboards
- `[ ]` **Phase 3: Ground Station Simulator** (Planned)
  - [ ] Model ground station networks (weather, signal strength, latency, outages)
  - [ ] Integrate link budget factors with the telemetry streams
- `[ ]` **Phase 4: Telemetry Packet Layer** (Planned)
  - [ ] Design binary serialization (CCSDS-inspired packet/frame layout)
- `[ ]` **Phase 5: Mission Replay Engine** (Planned)
  - [ ] Store historical telemetry and support playback speed control (1x to 100x)
- `[ ]` **Phase 6: Mission Control User Interface** (Planned)
  - [ ] Build React/TypeScript dashboard with WebSocket streaming and Three.js visualization
- `[ ]` **Phase 7: Orbital Visualization** (Planned)
  - [ ] Implement 3D Earth tracking for 1000+ satellites
- `[ ]` **Phase 8: AI Incident Commander** (Planned)
  - [ ] Incorporate LLM agent support for incident correlation, root cause analysis, and remediation guidance
