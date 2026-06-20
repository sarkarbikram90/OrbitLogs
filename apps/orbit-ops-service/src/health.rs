use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use dashmap::DashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeHealthScore {
    pub satellite_id: String,
    pub timestamp: DateTime<Utc>,
    pub composite_score: i32,           // 0 to 100
    pub subsystems: SubsystemScores,
    pub health_trend: String,           // 'STABLE', 'IMPROVING', 'DEGRADED'
    pub remaining_useful_life_days: i32,
    pub failure_prediction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemScores {
    pub power: i32,
    pub thermal: i32,
    pub adcs: i32,
    pub comms: i32,
    pub payload: i32,
    pub flight_computer: i32,
    pub propulsion: i32,
}

pub struct HealthScoringManager {
    // satellite_id -> current health score
    pub scores: DashMap<String, CompositeHealthScore>,
}

impl HealthScoringManager {
    pub fn new() -> Self {
        Self {
            scores: DashMap::new(),
        }
    }

    pub fn calculate_health(&self, sat_id: &str, temp: f64, battery_v: f64, pitch: f64, loss: f64, fuel: f32) {
        // EPS (Power)
        let power_score = if battery_v > 12.5 { 
            100 
        } else if battery_v > 11.0 { 
            60 + ((battery_v - 11.0) / 1.5 * 40.0) as i32 
        } else { 
            20 
        };

        // TCS (Thermal)
        let thermal_score = if temp < 45.0 { 
            100 
        } else if temp < 75.0 { 
            100 - ((temp - 45.0) / 30.0 * 40.0) as i32 
        } else { 
            20 
        };

        // ADCS (Attitude)
        let adcs_score = if pitch.abs() < 5.0 { 
            100 
        } else if pitch.abs() < 15.0 { 
            100 - ((pitch.abs() - 5.0) / 10.0 * 50.0) as i32 
        } else { 
            30 
        };

        // Comms
        let comms_score = if loss < 1.0 { 
            100 
        } else if loss < 15.0 { 100 - (loss * 3.0) as i32 
        } else { 
            40 
        };

        // Propulsion
        let prop_score = if fuel > 20.0 { 
            100 
        } else { 30 + (fuel * 3.0) as i32 
        };

        // Payload & OBC
        let payload_score = 100;
        let fc_score = 100;

        let composite = (power_score + thermal_score + adcs_score + comms_score + prop_score + payload_score + fc_score) / 7;

        let trend = if composite < 60 { "DEGRADED".to_string() } 
        else if composite < 85 { 
            "WARNING".to_string() } 
        else 
        { "STABLE".to_string() 
        };

        let useful_life = if composite < 50 { 
            90 
        } else if composite < 80 { 
            730 
        } else { 
            1825 
        }; // 5 years max

        let failure_prediction = if composite < 60 {
            Some("Critical battery aging or thermal overload cascade predicted within 7 days.".to_string())
        } else if composite < 85 {
            Some("Attitude control gyroscopic momentum dump alignment required within 14 days.".to_string())
        } else {
            None
        };

        self.scores.insert(sat_id.to_string(), CompositeHealthScore {
            satellite_id: sat_id.to_string(),
            timestamp: Utc::now(),
            composite_score: composite,
            subsystems: SubsystemScores {
                power: power_score,
                thermal: thermal_score,
                adcs: adcs_score,
                comms: comms_score,
                payload: payload_score,
                flight_computer: fc_score,
                propulsion: prop_score,
            },
            health_trend: trend,
            remaining_useful_life_days: useful_life,
            failure_prediction,
        });
    }
}

pub async fn get_health_score(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<CompositeHealthScore>, StatusCode> {
    if let Some(h) = state.health.scores.get(&satellite) {
        Ok(Json(h.clone()))
    } else {
        Ok(Json(CompositeHealthScore {
            satellite_id: satellite,
            timestamp: Utc::now(),
            composite_score: 100,
            subsystems: SubsystemScores {
                power: 100,
                thermal: 100,
                adcs: 100,
                comms: 100,
                payload: 100,
                flight_computer: 100,
                propulsion: 100,
            },
            health_trend: "STABLE".to_string(),
            remaining_useful_life_days: 1825,
            failure_prediction: None,
        }))
    }
}
