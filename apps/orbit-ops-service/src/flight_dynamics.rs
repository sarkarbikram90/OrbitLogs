use serde::{Serialize, Deserialize};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use dashmap::DashMap;
use sqlx::{PgPool, Row};
use rdkafka::producer::{FutureProducer, FutureRecord};
use chrono::Utc;
use std::time::Duration;
use tracing::error;

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
    db_pool: PgPool,
    kafka_producer: FutureProducer,
    timeline_topic: String,
}

impl FlightDynamicsManager {
    pub fn new(
        db_pool: PgPool,
        kafka_producer: FutureProducer,
        timeline_topic: String,
    ) -> Self {
        Self {
            states: DashMap::new(),
            db_pool,
            kafka_producer,
            timeline_topic,
        }
    }

    pub async fn update_dynamics(&self, sat_id: &str, current_altitude: f64, current_fuel: f32) {
        let idx = sat_id.strip_prefix("SAT-")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);
        
        let base_decay = if current_altitude < 480.0 { 
            180.0 
        } else if current_altitude < 500.0 { 
            85.0 
        }else { 
            35.0 
        };

        let decay = base_decay + (idx as f64 * 3.1) % 15.0;

        let fuel_burn = 0.01 + (idx as f64 * 0.005) % 0.08;
        let drift = 0.05 + (idx as f64 * 0.007) % 0.04;
        let effectiveness = 0.88 + (idx as f64 * 0.01) % 0.10;

        let mut recommendation = None;
        let mut days = 45;
        let mut event_type = None;
        let mut maneuver_type = None;
        let mut delta_v = 0.0;
        let mut fuel_req_kg = 0.0;

        if current_altitude < 490.0 {
            recommendation = Some(format!("Satellite {} altitude has decayed to {:.2}km. Station keeping orbit raising burn recommended.", sat_id, current_altitude));
            days = 2;
            event_type = Some("ORBIT_DECAY_ALERT");
            maneuver_type = Some("STATION_KEEPING");
            delta_v = 12.5; // m/s
            fuel_req_kg = 2.1;
        } else if current_fuel < 15.0 {
            recommendation = Some(format!("Warning: propellant fuel levels low ({:.1}%). Station-keeping maneuvers should use sparse cold gas bursts.", current_fuel));
            days = 15;
            event_type = Some("FUEL_WARNING");
        } else if (idx + 3) % 7 == 0 {
            recommendation = Some("Routine RAAN alignment maneuver recommended to maintain constellation plane spacing.".to_string());
            days = 5;
            event_type = Some("MANEUVER_RECOMMENDED");
            maneuver_type = Some("RAAN_ALIGNMENT");
            delta_v = 3.2; // m/s
            fuel_req_kg = 0.5;
        }

        self.states.insert(sat_id.to_string(), FlightDynamicsState {
            satellite_id: sat_id.to_string(),
            altitude_decay_rate_m_day: decay,
            raan_drift_deg_day: drift,
            fuel_burn_rate_pct_day: fuel_burn,
            maneuver_effectiveness_index: effectiveness,
            recommendation: recommendation.clone(),
            days_until_maneuver: days,
        });

        // Persist maneuver to DB and publish event
        if let Some(ref rec_msg) = recommendation {
            let event_name = event_type.unwrap_or("MANEUVER_RECOMMENDED");
            let m_type = maneuver_type.unwrap_or("STATION_KEEPING");
            let id = uuid::Uuid::new_v4();
            let scheduled_time = Utc::now() + chrono::Duration::days(days as i64);

            let _ = sqlx::query(
                "INSERT INTO orbit_maneuvers (id, satellite_id, maneuver_type, delta_v, fuel_consumption_kg, scheduled_time, status)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)"
            )
            .bind(id)
            .bind(sat_id)
            .bind(m_type)
            .bind(delta_v)
            .bind(fuel_req_kg)
            .bind(scheduled_time)
            .bind("RECOMMENDED")
            .execute(&self.db_pool)
            .await;

            let timeline_payload = serde_json::json!({
                "timestamp": Utc::now(),
                "event_id": uuid::Uuid::new_v4(),
                "source": "FLIGHT_DYNAMICS",
                "satellite_id": sat_id,
                "event_type": event_name,
                "severity": if event_name == "ORBIT_DECAY_ALERT" { "CRITICAL" } else { "WARNING" },
                "message": rec_msg,
                "details": {
                    "maneuver_id": id,
                    "maneuver_type": m_type,
                    "delta_v_m_s": delta_v,
                    "fuel_required_kg": fuel_req_kg,
                    "scheduled_time": scheduled_time,
                    "days_until_maneuver": days
                }
            });

            if let Ok(payload) = serde_json::to_string(&timeline_payload) {
                let record = FutureRecord::to(&self.timeline_topic)
                    .key(sat_id)
                    .payload(&payload);
                if let Err(e) = self.kafka_producer.send(record, Duration::from_secs(5)).await {
                    error!("Failed to publish flight dynamics event: {:?}", e);
                }
            }
        }
    }
}

pub async fn get_flight_dynamics(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<FlightDynamicsState>, StatusCode> {
    if let Some(fd) = state.flight_dynamics.states.get(&satellite) {
        Ok(Json(fd.clone()))
    } else {
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

#[derive(Serialize)]
pub struct TrendAnalysis {
    pub satellite_id: String,
    pub decay_trend: String,             // 'STABLE', 'ACCELERATING', 'RECOVERING'
    pub estimated_reentry_days: i32,
    pub total_maneuvers_executed: i64,
    pub total_fuel_spent_kg: f64,
}

pub async fn get_dynamics_trends(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<TrendAnalysis>, (StatusCode, String)> {
    let count_row = sqlx::query("SELECT COUNT(*) as count, COALESCE(SUM(fuel_consumption_kg), 0.0) as fuel FROM orbit_maneuvers WHERE satellite_id = $1 AND status = 'COMPLETED'")
        .bind(&satellite)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let count: i64 = count_row.get("count");
    let fuel: f64 = count_row.get("fuel");

    let fd_state = state.flight_dynamics.states.get(&satellite);
    let decay_rate = fd_state.as_ref().map(|s| s.altitude_decay_rate_m_day).unwrap_or(50.0);
    
    let trend = if decay_rate > 100.0 
    { "ACCELERATING".to_string() 
    } else { 
        "STABLE".to_string() 
    };
    let reentry_days = if decay_rate > 0.0 
    { (400.0 * 1000.0 / decay_rate) as i32 
    } else { 
        9999 
    };

    Ok(Json(TrendAnalysis {
        satellite_id: satellite,
        decay_trend: trend,
        estimated_reentry_days: reentry_days,
        total_maneuvers_executed: count,
        total_fuel_spent_kg: fuel,
    }))
}
