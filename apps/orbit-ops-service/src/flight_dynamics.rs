use serde::{Serialize, Deserialize};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use dashmap::DashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightDynamicsState {
    pub satellite_id: String,
    pub altitude_decay_rate_m_day: f64,
    pub raan_drift_deg_day: f64,
    pub fuel_burn_rate_pct_day: f64,
    pub maneuver_effectiveness_index: f64, // 0.0 to 1.0 scale
    pub recommendation: Option<String>,
    pub days_until_maneuver: i32,
}

pub struct FlightDynamicsManager {
    pub states: DashMap<String, FlightDynamicsState>,
}

impl FlightDynamicsManager {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }

    pub fn update_dynamics(&self, sat_id: &str, current_altitude: f64, current_fuel: f32) {
        // Calculate dynamic properties
        let idx = sat_id.strip_prefix("SAT-")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        
        // Stars decay faster at lower altitudes
        let base_decay = if current_altitude < 480.0 { 180.0 } else if current_altitude < 500.0 { 85.0 } else { 35.0 };
        let decay = base_decay + (idx as f64 * 3.1) % 15.0;

        let fuel_burn = 0.01 + (idx as f64 * 0.005) % 0.08;
        let drift = 0.05 + (idx as f64 * 0.007) % 0.04;
        let effectiveness = 0.88 + (idx as f64 * 0.01) % 0.10;

        let (recommendation, days) = if current_altitude < 490.0 {
            (Some(format!("Satellite {} altitude has decayed to {:.2}km. Station keeping orbit raising burn recommended.", sat_id, current_altitude)), 2)
        } else if current_fuel < 15.0 {
            (Some(format!("Warning: propellant fuel levels low ({:.1}%). Station-keeping maneuvers should use sparse cold gas bursts.", current_fuel)), 15)
        } else if (idx + 3) % 7 == 0 {
            (Some("Routine RAAN alignment maneuver recommended to maintain constellation plane spacing.".to_string()), 5)
        } else {
            (None, 45)
        };

        self.states.insert(sat_id.to_string(), FlightDynamicsState {
            satellite_id: sat_id.to_string(),
            altitude_decay_rate_m_day: decay,
            raan_drift_deg_day: drift,
            fuel_burn_rate_pct_day: fuel_burn,
            maneuver_effectiveness_index: effectiveness,
            recommendation,
            days_until_maneuver: days,
        });
    }
}

pub async fn get_flight_dynamics(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<FlightDynamicsState>, StatusCode> {
    if let Some(fd) = state.flight_dynamics.states.get(&satellite) {
        Ok(Json(fd.clone()))
    } else {
        // Generate a default state if not found
        Ok(Json(FlightDynamicsState {
            satellite_id: satellite,
            altitude_decay_rate_m_day: 50.0,
            raan_drift_deg_day: 0.08,
            fuel_burn_rate_pct_day: 0.02,
            maneuver_effectiveness_index: 0.95,
            recommendation: None,
            days_until_maneuver: 30,
        }))
    }
}
