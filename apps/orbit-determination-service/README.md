# Orbit Determination Service 🛰️📐

The **Orbit Determination Service** is a high-accuracy navigation microservice that processes raw position measurements to generate refined, filtered orbital state vectors.

## Core Capabilities

- **Extended Kalman Filtering (EKF)**: Employs a 6-state EKF ($\mathbf{x} = [x, y, z, v_x, v_y, v_z]^T$) to track the position and velocity of each satellite in the Earth-Centered Inertial (ECI) frame.
- **Orbital Perturbation Modeling**: Integrates Keplerian gravity equations with process noise constraints to estimate drift.
- **Measurement Updates**: Fuses simulated GPS pseudorange or ranging observations to update state estimation vectors and covariance matrices.
- **Persistence**: Updates the TimescaleDB `orbit_positions` hypertable with filtered states.

## Configuration

| Parameter | CLI Flag | Env Var | Default | Description |
|:---|:---|:---|:---|:---|
| **Database URL** | `--database-url` | `DATABASE_URL` | None (Required) | Postgres connection string |
| **OD Interval** | `--interval-secs` | `OD_INTERVAL_SECS` | `60` | Loop sleep interval between runs |

## Running Locally

```bash
cargo run -p orbit-determination-service
```
