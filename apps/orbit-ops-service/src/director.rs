use serde::{Serialize, Deserialize};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use dashmap::DashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorRecommendation {
    pub category: String, // 'CRITICAL', 'WARNING', 'OPTIMIZATION'
    pub title: String,
    pub description: String,
    pub reference_runbook: String,
    pub proposed_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorStatus {
    pub satellite_id: String,
    pub operational_status: String, // 'NOMINAL', 'DEGRADED', 'SAFE_MODE'
    pub recommendations: Vec<DirectorRecommendation>,
}

pub struct DirectorManager {
    pub active_recommendations: DashMap<String, Vec<DirectorRecommendation>>,
}

impl DirectorManager {
    pub fn new() -> Self {
        Self {
            active_recommendations: DashMap::new(),
        }
    }

    pub fn evaluate_telemetry(&self, sat_id: &str, temp: f64, battery_v: f64, packet_loss: f64, fuel: f32) {
        let mut recs = Vec::new();

        if temp > 60.0 {
            recs.push(DirectorRecommendation {
                category: "CRITICAL".to_string(),
                title: "TCS Thermal Overload Detected".to_string(),
                description: format!("Cpu/Thermal core temperature ({:.1}°C) exceeds normal operating range. Offloading secondary payload systems recommended to prevent chip degradation.", temp),
                reference_runbook: "TCS-RUNBOOK-042 (Thermal Runaway Mitigation)".to_string(),
                proposed_commands: vec![
                    "CMD_SHED_LOAD_SECONDARY".to_string(),
                    "CMD_RADIATOR_YAW_ADJUST".to_string()
                ],
            });
        }

        if battery_v < 11.2 {
            recs.push(DirectorRecommendation {
                category: "CRITICAL".to_string(),
                title: "EPS Low Battery Voltage".to_string(),
                description: format!("Battery cell voltage ({:.2}V) has dropped below safety limits. Immediately disable active payload sensors to optimize charge accumulation.", battery_v),
                reference_runbook: "EPS-RUNBOOK-011 (Battery Under-Voltage Recovery)".to_string(),
                proposed_commands: vec![
                    "CMD_PAYLOAD_DEACTIVATE".to_string(),
                    "CMD_ENTER_SAFE_MODE".to_string()
                ],
            });
        }

        if packet_loss > 15.0 {
            recs.push(DirectorRecommendation {
                category: "WARNING".to_string(),
                title: "TTC Link Degraded".to_string(),
                description: format!("High packet loss ({:.1}%) detected over active ground station link. Re-calibrating high-gain transmitter array or rescheduling passes recommended.", packet_loss),
                reference_runbook: "TTC-RUNBOOK-078 (RF Attenuation & Interference)".to_string(),
                proposed_commands: vec![
                    "CMD_TTC_POWER_BOOST".to_string(),
                    "CMD_SCHEDULE_PASS_BANGALORE".to_string()
                ],
            });
        }

        if fuel < 20.0 && fuel > 5.0 {
            recs.push(DirectorRecommendation {
                category: "OPTIMIZATION".to_string(),
                title: "PROP Fuel Management".to_string(),
                description: format!("Satellite fuel remaining is low ({:.1}%). Station-keeping maneuvers should be shifted to sparse cold-gas pulses to prolong spacecraft lifecycle.", fuel),
                reference_runbook: "PROP-RUNBOOK-009 (Low-Thrust Sparse Maneuvers)".to_string(),
                proposed_commands: vec![
                    "CMD_PROP_SPARSE_CYCLE_ENABLE".to_string()
                ],
            });
        }

        self.active_recommendations.insert(sat_id.to_string(), recs);
    }
}

pub async fn get_director_status(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<DirectorStatus>, StatusCode> {
    let recs = state.director.active_recommendations.get(&satellite)
        .map(|entry| entry.value().clone())
        .unwrap_or_else(Vec::new);

    let status = if recs.iter().any(|r| r.category == "CRITICAL") {
        "DEGRADED"
    } else {
        "NOMINAL"
    };

    Ok(Json(DirectorStatus {
        satellite_id: satellite,
        operational_status: status.to_string(),
        recommendations: recs,
    }))
}
