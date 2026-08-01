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

//! Real-Time Space Weather & Solar Flare Monitoring Module.
//!
//! Models NOAA SWPC & ISRO Master Control Facility (MCF) space weather feeds
//! including solar proton flux (>10 MeV), geomagnetic storm indices (Kp, Dst),
//! atmospheric density drag multipliers, and Single Event Upset (SEU) risk.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// NOAA Space Weather Scales - Geomagnetic Storm Level (G1-G5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GeomagneticStormLevel {
    None,
    G1Minor,    // Kp = 5
    G2Moderate, // Kp = 6
    G3Strong,   // Kp = 7
    G4Severe,   // Kp = 8
    G5Extreme,  // Kp = 9
}

impl GeomagneticStormLevel {
    pub fn from_kp(kp: f64) -> Self {
        if kp >= 9.0 {
            GeomagneticStormLevel::G5Extreme
        } else if kp >= 8.0 {
            GeomagneticStormLevel::G4Severe
        } else if kp >= 7.0 {
            GeomagneticStormLevel::G3Strong
        } else if kp >= 6.0 {
            GeomagneticStormLevel::G2Moderate
        } else if kp >= 5.0 {
            GeomagneticStormLevel::G1Minor
        } else {
            GeomagneticStormLevel::None
        }
    }
}

/// NOAA Space Weather Scales - Solar Radiation Storm Level (S1-S5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RadiationStormLevel {
    None,
    S1Minor,    // Flux >= 10 pfu
    S2Moderate, // Flux >= 100 pfu
    S3Strong,   // Flux >= 1,000 pfu
    S4Severe,   // Flux >= 10,000 pfu
    S5Extreme,  // Flux >= 100,000 pfu
}

impl RadiationStormLevel {
    pub fn from_flux_pfu(flux: f64) -> Self {
        if flux >= 100_000.0 {
            RadiationStormLevel::S5Extreme
        } else if flux >= 10_000.0 {
            RadiationStormLevel::S4Severe
        } else if flux >= 1_000.0 {
            RadiationStormLevel::S3Strong
        } else if flux >= 100.0 {
            RadiationStormLevel::S2Moderate
        } else if flux >= 10.0 {
            RadiationStormLevel::S1Minor
        } else {
            RadiationStormLevel::None
        }
    }
}

/// Single Event Upset (SEU) Radiation Risk Assessment for On-Board Avionics
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SeuRiskLevel {
    Low,
    Elevated,
    High,
    Critical,
}

/// Complete Real-Time Space Weather Observation Frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceWeatherObservation {
    /// Timestamp of observation (UTC)
    pub timestamp: DateTime<Utc>,
    /// Planetary Kp Index (0.0 to 9.0)
    pub kp_index: f64,
    /// Disturbance Storm Time Index in nanotesla (Dst, typical range -400 to +50 nT)
    pub dst_index_nt: f64,
    /// Solar Proton Flux (>10 MeV) in pfu (particles / cm² s sr)
    pub solar_proton_flux_pfu: f64,
    /// Solar Radio Flux F10.7 index (sfu)
    pub f10_7_index: f64,
    /// Derived Geomagnetic Storm Level
    pub geomagnetic_storm: GeomagneticStormLevel,
    /// Derived Solar Radiation Storm Level
    pub radiation_storm: RadiationStormLevel,
    /// Upper Atmospheric Drag Density Multiplier relative to baseline (1.0 = nominal)
    pub drag_density_multiplier: f64,
    /// Single Event Upset (SEU) Risk Level for memory & star tracker electronics
    pub seu_risk: SeuRiskLevel,
}

/// Computes upper atmospheric drag density multiplier from geomagnetic and solar radio indices.
///
/// Uses an empirical thermospheric heating model where solar F10.7 flux and geomagnetic Kp index
/// drive thermospheric expansion, increasing drag density at LEO altitudes (300-600 km).
pub fn calculate_atmospheric_drag_multiplier(kp_index: f64, f10_7_index: f64) -> f64 {
    // Nominal baseline: F10.7 = 70 sfu, Kp = 2.0
    let f10_7_factor = (f10_7_index / 70.0).max(1.0);
    let kp_factor = 1.0 + 0.15 * (kp_index - 2.0).max(0.0).powf(1.4);

    (f10_7_factor * kp_factor).clamp(1.0, 10.0)
}

/// Assesses Single Event Upset (SEU) radiation risk level for spacecraft microelectronics.
pub fn assess_seu_risk(solar_proton_flux_pfu: f64, dst_index_nt: f64) -> SeuRiskLevel {
    if solar_proton_flux_pfu >= 10_000.0 || dst_index_nt <= -150.0 {
        SeuRiskLevel::Critical
    } else if solar_proton_flux_pfu >= 1_000.0 || dst_index_nt <= -80.0 {
        SeuRiskLevel::High
    } else if solar_proton_flux_pfu >= 100.0 || dst_index_nt <= -30.0 {
        SeuRiskLevel::Elevated
    } else {
        SeuRiskLevel::Low
    }
}

/// Estimates LEO orbit decay rate in meters per day under given atmospheric drag conditions.
///
/// # Arguments
/// * `altitude_km` - Satellite altitude in km (e.g. 500.0 km)
/// * `drag_multiplier` - Thermospheric drag multiplier from `calculate_atmospheric_drag_multiplier`
/// * `area_to_mass_ratio` - Cross-sectional area to mass ratio in m²/kg (typical CubeSat ~0.01 m²/kg)
pub fn calculate_orbit_decay_rate_m_day(
    altitude_km: f64,
    drag_multiplier: f64,
    area_to_mass_ratio: f64,
) -> f64 {
    // Standard exponential atmospheric density model: ρ(h) = ρ0 * exp(-(h - h0)/H)
    let rho0 = 3.84e-12; // Base density at 500 km in kg/m³
    let h0 = 500.0;
    let scale_height_km = 60.0;

    let local_density = rho0 * (-(altitude_km - h0) / scale_height_km).exp() * drag_multiplier;
    let orbital_velocity_m_s = (398600.4418 / (6378.137 + altitude_km)).sqrt() * 1000.0;

    // Daily energy loss translated to orbital altitude drop rate
    let daily_drag_accel = 0.5 * local_density * area_to_mass_ratio * orbital_velocity_m_s.powi(2);
    let decay_m_day = daily_drag_accel * 86400.0 / orbital_velocity_m_s;

    decay_m_day.clamp(1.0, 10000.0)
}

/// Generates a space weather observation package from telemetry / SWPC feed inputs.
pub fn generate_space_weather_observation(
    kp_index: f64,
    dst_index_nt: f64,
    solar_proton_flux_pfu: f64,
    f10_7_index: f64,
) -> SpaceWeatherObservation {
    let geomagnetic_storm = GeomagneticStormLevel::from_kp(kp_index);
    let radiation_storm = RadiationStormLevel::from_flux_pfu(solar_proton_flux_pfu);
    let drag_density_multiplier = calculate_atmospheric_drag_multiplier(kp_index, f10_7_index);
    let seu_risk = assess_seu_risk(solar_proton_flux_pfu, dst_index_nt);

    SpaceWeatherObservation {
        timestamp: Utc::now(),
        kp_index,
        dst_index_nt,
        solar_proton_flux_pfu,
        f10_7_index,
        geomagnetic_storm,
        radiation_storm,
        drag_density_multiplier,
        seu_risk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geomagnetic_storm_level() {
        assert_eq!(GeomagneticStormLevel::from_kp(2.0), GeomagneticStormLevel::None);
        assert_eq!(GeomagneticStormLevel::from_kp(5.2), GeomagneticStormLevel::G1Minor);
        assert_eq!(GeomagneticStormLevel::from_kp(7.0), GeomagneticStormLevel::G3Strong);
        assert_eq!(GeomagneticStormLevel::from_kp(9.0), GeomagneticStormLevel::G5Extreme);
    }

    #[test]
    fn test_radiation_storm_level() {
        assert_eq!(RadiationStormLevel::from_flux_pfu(5.0), RadiationStormLevel::None);
        assert_eq!(RadiationStormLevel::from_flux_pfu(150.0), RadiationStormLevel::S2Moderate);
        assert_eq!(RadiationStormLevel::from_flux_pfu(1500.0), RadiationStormLevel::S3Strong);
    }

    #[test]
    fn test_seu_risk_assessment() {
        assert_eq!(assess_seu_risk(10.0, -10.0), SeuRiskLevel::Low);
        assert_eq!(assess_seu_risk(200.0, -40.0), SeuRiskLevel::Elevated);
        assert_eq!(assess_seu_risk(1500.0, -90.0), SeuRiskLevel::High);
        assert_eq!(assess_seu_risk(15000.0, -160.0), SeuRiskLevel::Critical);
    }

    #[test]
    fn test_drag_multiplier_and_decay() {
        let mult = calculate_atmospheric_drag_multiplier(4.5, 150.0);
        assert!(mult > 1.0 && mult <= 10.0);

        let decay = calculate_orbit_decay_rate_m_day(500.0, mult, 0.01);
        assert!(decay > 0.0);
    }

    #[test]
    fn test_space_weather_observation() {
        let obs = generate_space_weather_observation(4.2, -48.0, 1420.0, 135.0);
        assert_eq!(obs.geomagnetic_storm, GeomagneticStormLevel::None);
        assert_eq!(obs.radiation_storm, RadiationStormLevel::S3Strong);
        assert_eq!(obs.seu_risk, SeuRiskLevel::High);
    }
}
