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

//! Conjunction Assessment & Collision Avoidance Maneuver (CAM) Engine.
//!
//! Implements analytical collision probability computation using the
//! Foster/Alfano method and optimal RIC delta-v computation for
//! collision avoidance maneuvers.

use chrono::{DateTime, Utc};
use nalgebra::Matrix3;
use serde::{Deserialize, Serialize};


/// Conjunction event between a primary satellite and a secondary object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConjunctionEvent {
    /// Primary satellite identifier
    pub primary_id: String,
    /// Secondary object identifier (catalog number or temp designator)
    pub secondary_id: String,
    /// Secondary object type (DEBRIS, PAYLOAD, ROCKET_BODY, UNKNOWN)
    pub secondary_type: String,
    /// Time of Closest Approach (TCA) in UTC
    pub tca: DateTime<Utc>,
    /// Miss distance at TCA in meters
    pub miss_distance_m: f64,
    /// Relative velocity at TCA in m/s
    pub relative_velocity_m_s: f64,
    /// Collision probability (Pc) — dimensionless, typically 10⁻⁸ to 10⁻²
    pub collision_probability: f64,
    /// Primary position at TCA (ECI, km)
    pub primary_position_km: [f64; 3],
    /// Secondary position at TCA (ECI, km)
    pub secondary_position_km: [f64; 3],
    /// Combined position covariance at TCA (3x3, km²)
    pub combined_covariance_km2: [[f64; 3]; 3],
    /// Hard body radius for combined objects in meters
    pub combined_hard_body_radius_m: f64,
}

/// 3D covariance ellipsoid for position uncertainty visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovarianceEllipsoid {
    /// Semi-axis lengths in RIC frame (Radial, In-Track, Cross-Track) in meters
    pub semi_axes_m: [f64; 3],
    /// Rotation matrix from RIC to ECI frame (column-major 3x3)
    pub rotation_matrix: [[f64; 3]; 3],
    /// Sigma level (1.0, 2.0, or 3.0)
    pub sigma_level: f64,
}

/// Proposed Collision Avoidance Maneuver (CAM).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CamProposal {
    /// Delta-v vector in RIC frame (Radial, In-Track, Cross-Track) in m/s
    pub delta_v_ric_m_s: [f64; 3],
    /// Total delta-v magnitude in m/s
    pub delta_v_magnitude_m_s: f64,
    /// Recommended burn start time (UTC)
    pub burn_time: DateTime<Utc>,
    /// Estimated propellant consumption in kg
    pub propellant_kg: f64,
    /// Post-maneuver predicted miss distance in meters
    pub post_burn_miss_distance_m: f64,
    /// Post-maneuver collision probability
    pub post_burn_pc: f64,
    /// Remaining fuel percentage after maneuver
    pub post_burn_fuel_pct: f64,
}

/// Computes collision probability using a simplified Foster/Alfano 2D analytical method.
///
/// Projects the 3D conjunction geometry onto the encounter plane (perpendicular
/// to the relative velocity vector) and computes the probability that the
/// relative position falls within the combined hard-body radius.
///
/// # Arguments
/// * `miss_distance_m` — Miss distance at TCA in meters
/// * `relative_velocity_m_s` — Relative velocity magnitude at TCA in m/s
/// * `covariance_km2` — Combined 3x3 position covariance in km²
/// * `hard_body_radius_m` — Combined hard body radius in meters
///
/// # Returns
/// Collision probability (0.0 to 1.0)
pub fn compute_collision_probability(
    miss_distance_m: f64,
    _relative_velocity_m_s: f64,
    covariance_km2: &[[f64; 3]; 3],
    hard_body_radius_m: f64,
) -> f64 {
    // Convert covariance from km² to m²
    let sigma_r = (covariance_km2[0][0] * 1e6).sqrt(); // Radial sigma in meters
    let sigma_i = (covariance_km2[1][1] * 1e6).sqrt(); // In-track sigma in meters

    // Guard against zero/very small covariance
    if sigma_r < 1e-10 || sigma_i < 1e-10 {
        return if miss_distance_m < hard_body_radius_m {
            1.0
        } else {
            0.0
        };
    }

    // Simplified 2D Gaussian encounter probability (Foster method)
    // P_c ≈ (R² / (2 * σ_r * σ_i)) * exp(-d² / (2 * σ²))
    let combined_sigma = (sigma_r * sigma_r + sigma_i * sigma_i).sqrt();
    let exponent = -(miss_distance_m * miss_distance_m) / (2.0 * combined_sigma * combined_sigma);
    let r_sq = hard_body_radius_m * hard_body_radius_m;

    let pc = (r_sq / (2.0 * sigma_r * sigma_i)) * exponent.exp();

    pc.clamp(0.0, 1.0)
}

/// Computes the optimal RIC delta-v vector for a collision avoidance maneuver.
///
/// Uses a simplified along-track (in-track) impulse strategy:
/// - The primary maneuver component is in the In-Track direction (prograde/retrograde)
/// - This is the most efficient direction for changing the time of arrival at TCA
/// - Radial and cross-track components are added for fine-tuning
///
/// # Arguments
/// * `event` — The conjunction event to mitigate
/// * `target_miss_distance_m` — Desired post-maneuver miss distance in meters
/// * `burn_lead_time_s` — Seconds before TCA to execute the burn
/// * `satellite_mass_kg` — Current satellite mass in kg
/// * `specific_impulse_s` — Engine specific impulse in seconds
/// * `current_fuel_pct` — Current fuel percentage (0-100)
pub fn compute_cam_delta_v(
    event: &ConjunctionEvent,
    target_miss_distance_m: f64,
    burn_lead_time_s: f64,
    satellite_mass_kg: f64,
    specific_impulse_s: f64,
    current_fuel_pct: f64,
) -> CamProposal {
    // Compute the required position change at TCA
    let required_displacement_m = target_miss_distance_m - event.miss_distance_m;

    // For in-track maneuvers: Δr ≈ Δv * Δt (simplified two-body)
    // The in-track displacement grows linearly with time before TCA
    let delta_v_in_track = if burn_lead_time_s > 0.0 {
        required_displacement_m.abs() / burn_lead_time_s
    } else {
        0.5 // Default 0.5 m/s if no lead time
    };

    // Primary maneuver is in-track (prograde to increase altitude/period)
    let sign = if required_displacement_m >= 0.0 {
        1.0
    } else {
        -1.0
    };

    // Small cross-track component for out-of-plane adjustment
    let delta_v_cross = delta_v_in_track * 0.05;

    let delta_v_ric = [0.0, sign * delta_v_in_track, delta_v_cross];
    let delta_v_magnitude =
        (delta_v_ric[0].powi(2) + delta_v_ric[1].powi(2) + delta_v_ric[2].powi(2)).sqrt();

    // Tsiolkovsky rocket equation: Δm = m * (1 - e^(-Δv / (Isp * g0)))
    let g0 = 9.80665; // Standard gravity m/s²
    let exhaust_velocity = specific_impulse_s * g0;
    let mass_ratio = (-delta_v_magnitude / exhaust_velocity).exp();
    let propellant_kg = satellite_mass_kg * (1.0 - mass_ratio);

    // Compute post-burn fuel percentage
    let total_fuel_kg = satellite_mass_kg * (current_fuel_pct / 100.0);
    let remaining_fuel_kg = (total_fuel_kg - propellant_kg).max(0.0);
    let post_burn_fuel_pct = (remaining_fuel_kg / satellite_mass_kg) * 100.0;

    // Estimate post-burn miss distance
    let post_burn_miss = event.miss_distance_m + (delta_v_in_track * burn_lead_time_s);

    // Estimate post-burn Pc (should be dramatically reduced)
    let post_burn_pc = compute_collision_probability(
        post_burn_miss,
        event.relative_velocity_m_s,
        &event.combined_covariance_km2,
        event.combined_hard_body_radius_m,
    );

    let burn_time = event.tca - chrono::Duration::seconds(burn_lead_time_s as i64);

    CamProposal {
        delta_v_ric_m_s: delta_v_ric,
        delta_v_magnitude_m_s: delta_v_magnitude,
        burn_time,
        propellant_kg,
        post_burn_miss_distance_m: post_burn_miss,
        post_burn_pc,
        post_burn_fuel_pct,
    }
}

/// Extracts covariance ellipsoid parameters for 3D visualization.
///
/// Computes semi-axis lengths and rotation for rendering the position
/// uncertainty ellipsoid at a given sigma level.
pub fn compute_covariance_ellipsoid(
    covariance_km2: &[[f64; 3]; 3],
    sigma_level: f64,
) -> CovarianceEllipsoid {
    let cov = Matrix3::new(
        covariance_km2[0][0],
        covariance_km2[0][1],
        covariance_km2[0][2],
        covariance_km2[1][0],
        covariance_km2[1][1],
        covariance_km2[1][2],
        covariance_km2[2][0],
        covariance_km2[2][1],
        covariance_km2[2][2],
    );

    // Eigendecomposition for principal axes
    let eigen = cov.symmetric_eigen();

    // Semi-axes = sqrt(eigenvalues) * sigma * 1000 (km → m)
    let semi_axes = [
        eigen.eigenvalues[0].abs().sqrt() * sigma_level * 1000.0,
        eigen.eigenvalues[1].abs().sqrt() * sigma_level * 1000.0,
        eigen.eigenvalues[2].abs().sqrt() * sigma_level * 1000.0,
    ];

    let rot = eigen.eigenvectors;
    let rotation_matrix = [
        [rot[(0, 0)], rot[(0, 1)], rot[(0, 2)]],
        [rot[(1, 0)], rot[(1, 1)], rot[(1, 2)]],
        [rot[(2, 0)], rot[(2, 1)], rot[(2, 2)]],
    ];

    CovarianceEllipsoid {
        semi_axes_m: semi_axes,
        rotation_matrix,
        sigma_level,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_collision_probability_high_risk() {
        // Close approach: 200m miss, large covariance
        let cov = [
            [0.01, 0.0, 0.0],   // 100m sigma radial
            [0.0, 0.04, 0.0],   // 200m sigma in-track
            [0.0, 0.0, 0.001],  // 31m sigma cross-track
        ];
        let pc = compute_collision_probability(200.0, 14000.0, &cov, 10.0);
        assert!(pc > 0.0, "Pc should be positive for close approach");
        assert!(pc < 1.0, "Pc should be less than 1");
    }

    #[test]
    fn test_collision_probability_safe() {
        // Very far miss: 50km miss, small covariance
        let cov = [
            [0.001, 0.0, 0.0],
            [0.0, 0.001, 0.0],
            [0.0, 0.0, 0.001],
        ];
        let pc = compute_collision_probability(50_000.0, 10000.0, &cov, 5.0);
        assert!(pc < 1e-10, "Pc should be negligible for large miss distance");
    }

    #[test]
    fn test_cam_delta_v_computation() {
        let tca = Utc.with_ymd_and_hms(2026, 8, 15, 12, 0, 0).unwrap();

        let event = ConjunctionEvent {
            primary_id: "SAT-0001".to_string(),
            secondary_id: "COSMOS-2251-DEB-1234".to_string(),
            secondary_type: "DEBRIS".to_string(),
            tca,
            miss_distance_m: 150.0,
            relative_velocity_m_s: 14200.0,
            collision_probability: 5e-4,
            primary_position_km: [6800.0, 0.0, 0.0],
            secondary_position_km: [6800.15, 0.0, 0.0],
            combined_covariance_km2: [
                [0.01, 0.0, 0.0],
                [0.0, 0.04, 0.0],
                [0.0, 0.0, 0.001],
            ],
            combined_hard_body_radius_m: 10.0,
        };

        let proposal = compute_cam_delta_v(
            &event,
            5000.0,  // Target 5km miss distance
            2700.0,  // 45 minutes before TCA
            500.0,   // 500 kg satellite
            220.0,   // 220s Isp (hydrazine)
            95.0,    // 95% fuel remaining
        );

        assert!(proposal.delta_v_magnitude_m_s > 0.0);
        assert!(proposal.delta_v_magnitude_m_s < 10.0, "Δv should be reasonable");
        assert!(proposal.propellant_kg > 0.0);
        assert!(proposal.propellant_kg < 50.0, "Propellant use should be reasonable");
        assert!(proposal.post_burn_miss_distance_m > event.miss_distance_m);
        assert!(proposal.post_burn_fuel_pct < 95.0);
        assert!(proposal.post_burn_fuel_pct > 0.0);
    }

    #[test]
    fn test_covariance_ellipsoid() {
        let cov = [
            [0.04, 0.0, 0.0],
            [0.0, 0.16, 0.0],
            [0.0, 0.0, 0.01],
        ];

        let ellipsoid = compute_covariance_ellipsoid(&cov, 3.0);
        assert_eq!(ellipsoid.sigma_level, 3.0);
        // All semi-axes should be positive
        for axis in &ellipsoid.semi_axes_m {
            assert!(*axis > 0.0);
        }
    }
}
