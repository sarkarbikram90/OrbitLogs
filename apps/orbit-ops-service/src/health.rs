use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeHealthScore {
    pub satellite_id: orbitlogs_models::SatelliteId,
    pub timestamp: DateTime<Utc>,
    pub composite_score: i32, // 0 to 100
    pub subsystems: SubsystemScores,
    pub health_trend: String, // 'STABLE', 'IMPROVING', 'DEGRADED'
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

    pub fn calculate_health(
        &self,
        sat_id: &str,
        temp: f64,
        battery_v: f64,
        pitch: f64,
        loss: f64,
        fuel: f32,
    ) {
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
        } else if loss < 15.0 {
            100 - (loss * 3.0) as i32
        } else {
            40
        };

        // Propulsion
        let prop_score = if fuel > 20.0 {
            100
        } else {
            30 + (fuel * 3.0) as i32
        };

        // Payload & OBC
        let payload_score = 100;
        let fc_score = 100;

        let composite = (power_score
            + thermal_score
            + adcs_score
            + comms_score
            + prop_score
            + payload_score
            + fc_score)
            / 7;

        let trend = if composite < 60 {
            "DEGRADED".to_string()
        } else if composite < 85 {
            "WARNING".to_string()
        } else {
            "STABLE".to_string()
        };

        let useful_life = if composite < 50 {
            90
        } else if composite < 80 {
            730
        } else {
            1825
        }; // 5 years max

        let failure_prediction = if composite < 60 {
            Some(
                "Critical battery aging or thermal overload cascade predicted within 7 days."
                    .to_string(),
            )
        } else if composite < 85 {
            Some(
                "Attitude control gyroscopic momentum dump alignment required within 14 days."
                    .to_string(),
            )
        } else {
            None
        };

        self.scores.insert(
            sat_id.to_string(),
            CompositeHealthScore {
                satellite_id: orbitlogs_models::SatelliteId::new(sat_id),
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
            },
        );
    }
}

pub async fn get_health_score(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<orbitlogs_models::SatelliteId>,
) -> Result<Json<CompositeHealthScore>, StatusCode> {
    if let Some(h) = state.health.scores.get(satellite.as_str()) {
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

impl crate::processor::TelemetryProcessor for HealthScoringManager {
    type Error = std::convert::Infallible;
    type Output = ();

    fn process<'a>(
        &'a self,
        sat_id: &'a str,
        record: &'a orbitlogs_models::TelemetryRecord,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>> {
        let temp = record.temperature.unwrap_or(20.0);
        let batt = record.battery_voltage.unwrap_or(13.8);
        let pitch = record.orientation_pitch.unwrap_or(0.0) as f64;
        let loss = record.packet_loss.unwrap_or(0.0) as f64;
        let fuel = record.fuel_remaining.unwrap_or(100.0);

        self.calculate_health(sat_id, temp, batt, pitch, loss, fuel);

        Box::pin(async move { Ok(()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_scoring_manager_new() {
        let manager = HealthScoringManager::new();
        assert!(manager.scores.is_empty());
    }

    #[test]
    fn test_calculate_health_nominal() {
        let manager = HealthScoringManager::new();
        // High battery, low temp, low pitch, low packet loss, good fuel
        manager.calculate_health("SAT-NOMINAL", 30.0, 13.0, 2.0, 0.5, 30.0);

        let score = manager.scores.get("SAT-NOMINAL").unwrap();
        assert_eq!(score.composite_score, 100);
        assert_eq!(score.subsystems.power, 100);
        assert_eq!(score.subsystems.thermal, 100);
        assert_eq!(score.subsystems.adcs, 100);
        assert_eq!(score.subsystems.comms, 100);
        assert_eq!(score.subsystems.propulsion, 100);
        assert_eq!(score.health_trend, "STABLE");
        assert_eq!(score.remaining_useful_life_days, 1825);
        assert!(score.failure_prediction.is_none());
    }

    #[test]
    fn test_calculate_health_warning() {
        let manager = HealthScoringManager::new();
        // Moderate parameters to trigger Warning state (composite < 85 and >= 60)
        manager.calculate_health("SAT-WARNING", 60.0, 11.5, 10.0, 5.0, 10.0);

        let score = manager.scores.get("SAT-WARNING").unwrap();
        // Power score = 60 + ((11.5 - 11.0) / 1.5 * 40.0) = 60 + 13 = 73
        // Thermal score = 100 - ((60.0 - 45.0) / 30.0 * 40.0) = 100 - 20 = 80
        // ADCS score = 100 - ((10.0 - 5.0) / 10.0 * 50.0) = 100 - 25 = 75
        // Comms score = 100 - (5.0 * 3.0) = 85
        // Propulsion score = 30 + (10.0 * 3.0) = 60
        // Composite = (73 + 80 + 75 + 85 + 60 + 100 + 100) / 7 = 573 / 7 = 81
        assert_eq!(score.composite_score, 81);
        assert_eq!(score.subsystems.power, 73);
        assert_eq!(score.subsystems.thermal, 80);
        assert_eq!(score.subsystems.adcs, 75);
        assert_eq!(score.subsystems.comms, 85);
        assert_eq!(score.subsystems.propulsion, 60);
        assert_eq!(score.health_trend, "WARNING");
        assert_eq!(score.remaining_useful_life_days, 1825); // >= 80 composite score -> 1825 days
        assert_eq!(
            score.failure_prediction.as_deref(),
            Some("Attitude control gyroscopic momentum dump alignment required within 14 days.")
        );
    }

    #[test]
    fn test_calculate_health_degraded() {
        let manager = HealthScoringManager::new();
        // Very bad parameters to trigger Degraded state (composite < 60)
        manager.calculate_health("SAT-DEGRADED", 80.0, 10.0, 20.0, 20.0, 2.0);

        let score = manager.scores.get("SAT-DEGRADED").unwrap();
        // Power score = 20
        // Thermal score = 20
        // ADCS score = 30
        // Comms score = 40
        // Propulsion score = 30 + (2.0 * 3.0) = 36
        // Composite = (20 + 20 + 30 + 40 + 36 + 100 + 100) / 7 = 346 / 7 = 49
        assert_eq!(score.composite_score, 49);
        assert_eq!(score.health_trend, "DEGRADED");
        assert_eq!(score.remaining_useful_life_days, 90); // < 50 composite score -> 90 days
        assert_eq!(
            score.failure_prediction.as_deref(),
            Some("Critical battery aging or thermal overload cascade predicted within 7 days.")
        );
    }
}
