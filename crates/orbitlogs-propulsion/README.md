# OrbitLogs Propulsion Crate 🚀🔥

The **OrbitLogs Propulsion** library provides math operations for spacecraft trajectory modification calculations.

## Core Features

- **Tsiolkovsky Rocket Equation Solver**: Calculates propellant mass consumption for impulsive orbital maneuvers:
  $$m_{\text{fuel}} = m_{\text{start}} \left(1 - e^{-\frac{\Delta V}{I_{sp} g_0}}\right)$$
- **Maneuver Planning**: Generates detailed `ManeuverPlan` structures tracking execution epochs, 3-axis Cartesian ECI velocity adjustments ($\Delta V$), and fuel costs in kilograms.

## Usage

Include in `Cargo.toml`:
```toml
orbitlogs-propulsion = { workspace = true }
```

Sample code:
```rust
use chrono::Utc;
use orbitlogs_propulsion::ManeuverPlan;

let plan = ManeuverPlan::calculate_maneuver(
    "SAT-0001",
    250.0, // start mass (kg)
    300.0, // Isp (seconds)
    [0.45, 0.0, 0.0], // Delta-V vector (m/s)
    Utc::now(), // burn epoch
);
println!("Propellant cost: {} kg", plan.fuel_consumed_kg);
```
