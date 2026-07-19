# Autonomous Mission Planner 🤖🚀

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green.svg)](../../LICENSE)
[![CNCF Sandbox Status](https://img.shields.io/badge/CNCF-Sandbox%20Applicant-purple.svg)](../../GOVERNANCE.md)

The **Autonomous Mission Planner** microservice synthesizes, validates, and sequences spacecraft command timelines for orbiting assets within the OrbitLogs ecosystem.

---

## 🏛️ Architecture & Module Structure

The microservice is organized into domain-focused Rust modules:

* **`config.rs`**: Environment configuration and CLI argument parsing (`Cli`).
* **`command.rs`**: Spacecraft `Command` definitions and battery SoC impact calculations (`battery_delta`).
* **`digital_twin.rs`**: Spacecraft telemetry snapshot model (`DigitalTwinState`).
* **`error.rs`**: Flight safety constraint violation types (`SafetyViolation`).
* **`timeline.rs`**: `OnboardTimeline` schedule management & safety threshold verification algorithms.
* **`planner.rs`**: Evaluation cycle execution (`run_planner_cycle`).
* **`main.rs`**: Application entrypoint, tracing initialization, and tokio loop with graceful shutdown signal handling.

---

## 🚀 Core Capabilities

- **Hierarchical Task Network (HTN) Planning**: Generates conditional schedules of spacecraft commands (`ShedSecondaryLoad`, `AdjustRadiatorYaw`, `OrientSolarPanels`, `ExecuteBurn`).
- **Digital Twin Co-Validation**: Before uploading a generated timeline, the planner simulates command impact against virtual representations in the `digital-twin-engine`.
- **Flight Safety Constraint Verification**: Verifies physical energy conservation constraints (e.g., maintaining battery state of charge above the minimum 20.0% safety threshold under degradation models).

---

## ⚙️ Configuration

| Parameter | CLI Flag | Environment Variable | Default Value | Description |
|:---|:---|:---|:---|:---|
| **Planner Interval** | `--interval-secs` | `PLANNER_INTERVAL_SECS` | `60.0` | Sleep interval (seconds) between evaluation cycles |

---

## 🧪 Running Locally

```bash
cargo run -p autonomous-mission-planning
```
