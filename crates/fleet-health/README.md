# Fleet Health Crate 💚

The **Fleet Health** library provides core availability, fleet readiness, and cumulative degradation scoring algorithms used across multiple microservices (e.g. `mission-health-engine`, `orbit-ops-service`).

## Core Calculations

- **Fleet Readiness**: Calculates operational status rates based on scheduled vs active payload execution ratios.
- **Subsystem Availability**: Evaluates redundancies for attitude control reaction wheels and battery modules.
- **Cumulative Degradation / Risk Score**: Tracks risk escalation from prolonged battery thermal stress, CPU throttles, or fuel constraints.
- **Capacity Scoring**: Assesses fuel/propellant reserves and remaining storage lifetimes.

## Usage

Include in `Cargo.toml`:
```toml
fleet-health = { workspace = true }
```

Reference in Rust code:
```rust
use fleet_health::{calculate_fleet_readiness, assess_subsystem_risk};
```
