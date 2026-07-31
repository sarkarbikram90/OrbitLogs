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

//! Monte Carlo 48-Hour Predictive Digital Twin Fast-Forward Engine.
//!
//! Simulates thousands of stochastic future trajectories for a spacecraft digital twin
//! under simulated solar flare events, thruster firings, battery capacity fading, and
//! thermal degradation to project health confidence intervals (p5, p25, p50, p75, p95).

use rand::Rng;
use serde::{Deserialize, Serialize};

/// Simulated environmental or operational perturbation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScenarioPerturbation {
    /// Geomagnetic storm / solar flare increasing thermal load and atmospheric drag
    SolarFlare { kp_index: f32, duration_hours: f32 },
    /// Thruster firing maneuver consuming fuel and generating thermal pulse
    ThrusterBurn { delta_v_m_s: f32, duration_sec: f32 },
    /// High-power payload activity pass
    PayloadBurst { duration_hours: f32, power_w: f32 },
    /// Nominal operational orbit trajectory
    Nominal,
}

/// Request parameters for running a Monte Carlo simulation fast-forward.
#[derive(Debug, Clone, Deserialize)]
pub struct MonteCarloRequest {
    /// Number of trial iterations (default 500, max 5000)
    pub trials: Option<usize>,
    /// Time horizon in hours (default 48.0)
    pub horizon_hours: Option<f32>,
    /// Selected environmental scenario perturbation
    pub scenario: Option<ScenarioPerturbation>,
}

/// Statistical summary across all Monte Carlo trial trajectories.
#[derive(Debug, Clone, Serialize)]
pub struct MetricPercentiles {
    pub p5: f64,
    pub p25: f64,
    pub p50: f64, // Median
    pub p75: f64,
    pub p95: f64,
}

/// Result envelope of a 48-hour Monte Carlo digital twin projection.
#[derive(Debug, Clone, Serialize)]
pub struct MonteCarloResult {
    pub satellite_id: String,
    pub trials_run: usize,
    pub horizon_hours: f32,
    pub scenario_name: String,
    pub battery_soc_percentiles: MetricPercentiles,
    pub temp_c_percentiles: MetricPercentiles,
    pub fuel_remaining_percentiles: MetricPercentiles,
    pub overall_health_percentiles: MetricPercentiles,
    pub predicted_anomalies_count: usize,
    pub anomaly_probability_percent: f64,
}

/// Executes a Monte Carlo stochastic fast-forward simulation on a satellite's digital twin state.
pub fn run_monte_carlo_projection(
    satellite_id: String,
    base_record: &orbitlogs_models::TelemetryRecord,
    request: MonteCarloRequest,
) -> MonteCarloResult {
    let trials = request.trials.unwrap_or(500).clamp(50, 5000);
    let horizon = request.horizon_hours.unwrap_or(48.0).clamp(1.0, 168.0);
    let scenario = request.scenario.unwrap_or(ScenarioPerturbation::Nominal);

    let scenario_name = match &scenario {
        ScenarioPerturbation::SolarFlare { kp_index, .. } => format!("Solar Flare (Kp = {:.1})", kp_index),
        ScenarioPerturbation::ThrusterBurn { delta_v_m_s, .. } => format!("Thruster Burn ({:.2} m/s Δv)", delta_v_m_s),
        ScenarioPerturbation::PayloadBurst { power_w, .. } => format!("High Payload Activity ({:.0} W)", power_w),
        ScenarioPerturbation::Nominal => "Nominal Orbit Operations".to_string(),
    };

    let base_v = base_record.battery_voltage.unwrap_or(28.0);
    let base_temp = base_record.temperature.unwrap_or(25.0);
    let base_fuel = base_record.fuel_remaining.unwrap_or(90.0) as f64;

    let mut final_socs = Vec::with_capacity(trials);
    let mut final_temps = Vec::with_capacity(trials);
    let mut final_fuels = Vec::with_capacity(trials);
    let mut final_healths = Vec::with_capacity(trials);
    let mut anomaly_count = 0;

    let mut rng = rand::thread_rng();

    for _ in 0..trials {
        // Perturb baseline values with stochastic Gaussian-like noise
        let noise_v: f64 = rng.gen_range(-0.5..0.5);
        let noise_t: f64 = rng.gen_range(-2.0..2.0);
        let noise_f: f64 = rng.gen_range(-0.2..0.2);

        let mut sim_soc = ((base_v - 22.0) / (32.0 - 22.0) * 100.0) + noise_v * 5.0;
        let mut sim_temp = base_temp + noise_t;
        let mut sim_fuel = base_fuel + noise_f;

        // Apply scenario effects over horizon
        match scenario {
            ScenarioPerturbation::SolarFlare { kp_index, duration_hours } => {
                let factor = (kp_index / 9.0) as f64 * (duration_hours / horizon) as f64;
                sim_temp += 15.0 * factor + rng.gen_range(-3.0..5.0);
                sim_soc -= 8.0 * factor;
            }
            ScenarioPerturbation::ThrusterBurn { delta_v_m_s, .. } => {
                let fuel_used = delta_v_m_s as f64 * 0.15 + rng.gen_range(0.01..0.05);
                sim_fuel = (sim_fuel - fuel_used).max(0.0);
                sim_temp += 4.0;
            }
            ScenarioPerturbation::PayloadBurst { power_w, duration_hours } => {
                let drain = (power_w as f64 / 100.0) * (duration_hours / horizon) as f64 * 10.0;
                sim_soc -= drain;
                sim_temp += (power_w as f64 / 50.0) * 2.0;
            }
            ScenarioPerturbation::Nominal => {
                sim_soc += rng.gen_range(-2.0..2.0);
                sim_temp += rng.gen_range(-1.0..1.0);
            }
        }

        sim_soc = sim_soc.clamp(0.0, 100.0);
        sim_fuel = sim_fuel.clamp(0.0, 100.0);

        let health_score = (sim_soc * 0.4 + (100.0 - (sim_temp - 20.0).abs() * 2.0).clamp(0.0, 100.0) * 0.3 + sim_fuel * 0.3).clamp(0.0, 100.0);

        if sim_soc < 20.0 || sim_temp > 60.0 || health_score < 40.0 {
            anomaly_count += 1;
        }

        final_socs.push(sim_soc);
        final_temps.push(sim_temp);
        final_fuels.push(sim_fuel);
        final_healths.push(health_score);
    }

    final_socs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    final_temps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    final_fuels.sort_by(|a, b| a.partial_cmp(b).unwrap());
    final_healths.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let calc_pct = |arr: &[f64]| -> MetricPercentiles {
        let n = arr.len();
        MetricPercentiles {
            p5: arr[(n as f64 * 0.05) as usize],
            p25: arr[(n as f64 * 0.25) as usize],
            p50: arr[(n as f64 * 0.50) as usize],
            p75: arr[(n as f64 * 0.75) as usize],
            p95: arr[(n as f64 * 0.95) as usize],
        }
    };

    let anomaly_probability_percent = (anomaly_count as f64 / trials as f64) * 100.0;

    MonteCarloResult {
        satellite_id,
        trials_run: trials,
        horizon_hours: horizon,
        scenario_name,
        battery_soc_percentiles: calc_pct(&final_socs),
        temp_c_percentiles: calc_pct(&final_temps),
        fuel_remaining_percentiles: calc_pct(&final_fuels),
        overall_health_percentiles: calc_pct(&final_healths),
        predicted_anomalies_count: anomaly_count,
        anomaly_probability_percent,
    }
}
