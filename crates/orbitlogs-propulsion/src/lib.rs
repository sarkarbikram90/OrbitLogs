// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManeuverPlan {
    pub sat_id: String,
    pub burn_epoch: DateTime<Utc>,
    pub delta_v_vector: [f64; 3], // ECI coordinate velocity adjustments (m/s)
    pub fuel_consumed_kg: f32,
}

impl ManeuverPlan {
    /// Leverages the Tsiolkovsky rocket equation to calculate fuel consumed for a delta-v burn.
    /// Delta-V is in m/s, sat_mass is in kg, isp_seconds is in seconds.
    pub fn calculate_maneuver(
        sat_id: &str,
        sat_mass: f32,
        isp_seconds: f32,
        required_dv: [f64; 3],
        burn_epoch: DateTime<Utc>,
    ) -> Self {
        let g0 = 9.80665;
        let dv_mag =
            (required_dv[0].powi(2) + required_dv[1].powi(2) + required_dv[2].powi(2)).sqrt();

        // Rocket equation: Delta V = Isp * g0 * ln(m_start / m_final)
        // => m_final = m_start / exp(Delta V / (Isp * g0))
        // => m_fuel = m_start - m_final = m_start * (1 - exp(-Delta V / (Isp * g0)))
        let exponent = -dv_mag / (isp_seconds as f64 * g0);
        let mass_ratio = exponent.exp();
        let fuel_mass = sat_mass * (1.0 - mass_ratio as f32);

        Self {
            sat_id: sat_id.to_string(),
            burn_epoch,
            delta_v_vector: required_dv,
            fuel_consumed_kg: fuel_mass,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_maneuver() {
        let sat_id = "SAT-0001";
        let sat_mass = 250.0; // kg
        let isp = 300.0; // seconds for cold gas or krypton thruster
        let required_dv = [10.0, 0.0, 0.0]; // 10 m/s burn in x-axis
        let epoch = Utc::now();

        let plan = ManeuverPlan::calculate_maneuver(sat_id, sat_mass, isp, required_dv, epoch);

        assert_eq!(plan.sat_id, sat_id);
        assert!(plan.fuel_consumed_kg > 0.0);
        assert!(plan.fuel_consumed_kg < sat_mass); // Fuel consumed should be less than start mass

        // Expected mass ratio: exp(-10 / (300 * 9.80665)) = exp(-0.003399) = 0.996606
        // Fuel mass: 250 * (1 - 0.996606) = 250 * 0.003394 = ~0.848 kg
        assert!((plan.fuel_consumed_kg - 0.848).abs() < 1e-2);
    }
}
