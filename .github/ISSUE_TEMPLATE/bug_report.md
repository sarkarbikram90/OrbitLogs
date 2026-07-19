---
name: Bug report 🐛
about: Report a bug or telemetry anomaly in OrbitLogs
title: '[BUG] '
labels: bug
assignees: ''

---

**Describe the bug**
A clear and concise description of what the bug is.

**Impacted Microservice or Library Crate**
Check all components that exhibit the issue:
- [ ] Telemetry Collector (`apps/telemetry-collector`)
- [ ] Telemetry Simulator (`apps/telemetry-simulator`)
- [ ] Ground Station Simulator (`apps/ground-station-simulator`)
- [ ] Mission Health Engine (`apps/mission-health-engine`)
- [ ] Mission Replay Engine (`apps/mission-replay-engine`)
- [ ] Mission Command Service (`apps/mission-command-service`)
- [ ] Digital Twin Engine (`apps/digital-twin-engine`)
- [ ] AI Incident Commander (`apps/ai-incident-commander`)
- [ ] Orbit Operations & STM Service (`apps/orbit-ops-service`)
- [ ] Astrodynamics Library (`crates/orbitlogs-astrodynamics`)
- [ ] Mission Control UI (`apps/mission-control-ui`)
- [ ] Other / Infrastructure

**Steps to Reproduce**
1. Run command '...'
2. Perform action '...'
3. Observe error payload or behavior '...'

**Expected Behavior**
A clear description of what you expected to happen.

**Telemetry Logs / Screenshots**
If applicable, attach log outputs, raw Kafka record headers, or console screenshots.

**Environment**
- OS: [e.g. Linux, macOS, Windows]
- Rust version: [e.g. 1.82.0]
- Node.js version: [e.g. 20.x]
- Docker / Compose version:
