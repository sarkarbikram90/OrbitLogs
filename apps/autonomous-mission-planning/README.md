# Autonomous Mission Planner 🤖🚀

The **Autonomous Mission Planner** manages command validation and execution sequencing on behalf of passing satellites.

## Core Capabilities

- **Hierarchical Task Network (HTN) Planning**: Generates conditional schedules of spacecraft commands (`ShedSecondaryLoad`, `AdjustRadiatorYaw`, `OrientSolarPanels`, `ExecuteBurn`).
- **Digital Twin Co-Validation**: Before uploading a generated timeline, the planner runs the command sequence against the satellite's virtual representation in the `digital-twin-engine`.
- **Constraint Solver**: Assures that critical thresholds (e.g., keeping battery reserves above a minimum $20.0\%$ state of charge) are never breached during maneuver executions.

## Configuration

| Parameter | CLI Flag | Env Var | Default | Description |
|:---|:---|:---|:---|:---|
| **Planner Interval** | `--interval-secs` | `PLANNER_INTERVAL_SECS` | `60` | Loop sleep interval between runs |

## Running Locally

```bash
cargo run -p autonomous-mission-planning
```
