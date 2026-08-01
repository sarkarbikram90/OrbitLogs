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

    /// Synthesizes an optimal 3D Collision Avoidance Maneuver (CAM) thrust burn vector
    /// in ECI frame to achieve a target safety miss distance at Time of Closest Approach (TCA).
    pub fn calculate_cam_burn_vector(
        sat_id: &str,
        sat_mass: f32,
        isp_seconds: f32,
        pos_eci: [f64; 3],
        vel_eci: [f64; 3],
        target_miss_distance_m: f64,
        time_to_ca_secs: f64,
        burn_epoch: DateTime<Utc>,
    ) -> Self {
        // Construct RIC (Radial, In-track, Cross-track) frame unit vectors
        let r_mag = (pos_eci[0].powi(2) + pos_eci[1].powi(2) + pos_eci[2].powi(2)).sqrt();
        let r_hat = [pos_eci[0] / r_mag, pos_eci[1] / r_mag, pos_eci[2] / r_mag];

        // Cross-track: h = r x v
        let h_x = pos_eci[1] * vel_eci[2] - pos_eci[2] * vel_eci[1];
        let h_y = pos_eci[2] * vel_eci[0] - pos_eci[0] * vel_eci[2];
        let h_z = pos_eci[0] * vel_eci[1] - pos_eci[1] * vel_eci[0];
        let h_mag = (h_x.powi(2) + h_y.powi(2) + h_z.powi(2)).sqrt();
        let c_hat = [h_x / h_mag, h_y / h_mag, h_z / h_mag];

        // In-track: t = c x r
        let t_hat = [
            c_hat[1] * r_hat[2] - c_hat[2] * r_hat[1],
            c_hat[2] * r_hat[0] - c_hat[0] * r_hat[2],
            c_hat[0] * r_hat[1] - c_hat[1] * r_hat[0],
        ];

        // Transverse burn magnitude required for secular along-track drift: d = 1.5 * delta_v * t_ca
        let dv_in_track = if time_to_ca_secs > 0.0 {
            target_miss_distance_m / (1.5 * time_to_ca_secs)
        } else {
            0.1 // Default minimal nudge
        };

        // Convert RIC delta-v to ECI frame vector
        let dv_eci = [
            dv_in_track * t_hat[0],
            dv_in_track * t_hat[1],
            dv_in_track * t_hat[2],
        ];

        Self::calculate_maneuver(sat_id, sat_mass, isp_seconds, dv_eci, burn_epoch)
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

    #[test]
    fn test_calculate_cam_burn_vector() {
        let sat_id = "SAT-STARLINK-101";
        let sat_mass = 260.0;
        let isp = 230.0; // Krypton Hall-effect thruster
        let pos_eci = [6878.0, 0.0, 0.0]; // 500 km LEO orbit altitude
        let vel_eci = [0.0, 7.61, 0.0]; // Circular orbit velocity in km/s
        let target_miss_m = 500.0; // 500m safety clearance
        let time_to_ca = 3600.0; // 1 hour to TCA
        let epoch = Utc::now();

        let cam_plan = ManeuverPlan::calculate_cam_burn_vector(
            sat_id,
            sat_mass,
            isp,
            pos_eci,
            vel_eci,
            target_miss_m,
            time_to_ca,
            epoch,
        );

        assert_eq!(cam_plan.sat_id, sat_id);
        let dv_mag = (cam_plan.delta_v_vector[0].powi(2)
            + cam_plan.delta_v_vector[1].powi(2)
            + cam_plan.delta_v_vector[2].powi(2))
        .sqrt();

        // Required delta-v = 500 / (1.5 * 3600) = 0.09259 m/s
        assert!((dv_mag - 0.09259).abs() < 1e-3);
        assert!(cam_plan.fuel_consumed_kg > 0.0);
    }
}
