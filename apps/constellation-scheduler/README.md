# Constellation Scheduler Service 📅🤖

The **Constellation Scheduler** is an optimization service that determines the best times for ground station antennas to establish downlinks and uplinks with passing satellites.

## Core Capabilities

- **Constraint-Based Scheduling**: Solves the NP-hard communication slot allocation problem using look-ahead heuristic solvers.
- **Link Budget Calculations**: Computes path loss dynamically using free-space loss models modulated by simulated atmospheric conditions (such as the ITU-R rain fade approximation).
- **Physical Antenna Modeling**: Simulates antenna constraints including minimum elevation limits (e.g. $15.0^\circ$) and max slew rates.
- **Outcomes**: Populates the `contact_schedules` hypertable with scheduled contact times and link priorities.

## Configuration

| Parameter | CLI Flag | Env Var | Default | Description |
|:---|:---|:---|:---|:---|
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres connection string |
| **Scheduler Interval** | `--interval-secs` | `SCHEDULER_INTERVAL_SECS` | `60` | Loop sleep interval between runs |

## Running Locally

```bash
cargo run -p constellation-scheduler
```
