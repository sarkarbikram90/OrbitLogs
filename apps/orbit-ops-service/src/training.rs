use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::PgPool;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use dashmap::DashMap;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSession {
    pub session_id: Uuid,
    pub operator_name: String,
    pub scenario_name: String,
    pub satellite_id: String,
    pub started_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub commands_sent: Vec<String>,
    pub is_resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scorecard {
    pub id: Uuid,
    pub operator_name: String,
    pub scenario_name: String,
    pub response_time_seconds: f64,
    pub score: i32,
    pub passed: bool,
    pub timestamp: DateTime<Utc>,
    pub details: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct StartTrainingRequest {
    pub operator_name: String,
    pub scenario_name: String, // 'BATTERY_FAILURE', 'THERMAL_RUNAWAY', 'COMMS_OUTAGE', 'ATTITUDE_DRIFT'
    pub satellite_id: String,
}

pub struct TrainingManager {
    pub db_pool: PgPool,
    pub kafka_producer: FutureProducer,
    pub timeline_topic: String,
    // operator_name -> active session
    pub active_sessions: DashMap<String, TrainingSession>,
}

impl TrainingManager {
    pub fn new(db_pool: PgPool, kafka_producer: FutureProducer, timeline_topic: String) -> Self {
        Self {
            db_pool,
            kafka_producer,
            timeline_topic,
            active_sessions: DashMap::new(),
        }
    }

    pub async fn log_operator_command(&self, sat_id: &str, cmd_type: &str) {
        // Append sent command if it belongs to an active training session targeting this satellite
        for mut entry in self.active_sessions.iter_mut() {
            let session = entry.value_mut();
            if session.satellite_id == sat_id {
                session.commands_sent.push(cmd_type.to_string());
            }
        }
    }
}

pub async fn start_training(
    State(state): State<Arc<crate::ServiceState>>,
    Json(req): Json<StartTrainingRequest>,
) -> Result<Json<TrainingSession>, (StatusCode, String)> {
    let session_id = Uuid::new_v4();
    let session = TrainingSession {
        session_id,
        operator_name: req.operator_name.clone(),
        scenario_name: req.scenario_name.clone(),
        satellite_id: req.satellite_id.clone(),
        started_at: Utc::now(),
        acknowledged_at: None,
        commands_sent: Vec::new(),
        is_resolved: false,
    };

    state.training.active_sessions.insert(req.operator_name.clone(), session.clone());

    // Publish timeline start notification
    let msg = format!(
        "Training Scenario '{}' initialized. Operator '{}' assigned to satellite {}.",
        req.scenario_name, req.operator_name, req.satellite_id
    );

    publish_training_timeline(&state.training, &req.satellite_id, "TRAINING_STARTED", "WARNING", &msg).await;

    Ok(Json(session))
}

pub async fn acknowledge_training(
    State(state): State<Arc<crate::ServiceState>>,
    Path(operator): Path<String>,
) -> Result<Json<TrainingSession>, (StatusCode, String)> {
    let mut session = state.training.active_sessions.get_mut(&operator)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No active training session".to_string()))?;

    session.acknowledged_at = Some(Utc::now());
    let copied = session.clone();

    let msg = format!("Operator '{}' acknowledged training failure scenario.", operator);
    publish_training_timeline(&state.training, &session.satellite_id, "TRAINING_ACKNOWLEDGED", "INFO", &msg).await;

    Ok(Json(copied))
}

pub async fn resolve_training(
    State(state): State<Arc<crate::ServiceState>>,
    Path(operator): Path<String>,
) -> Result<Json<Scorecard>, (StatusCode, String)> {
    let (_, session) = state.training.active_sessions.remove(&operator)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No active training session".to_string()))?;

    let resolved_at = Utc::now();
    
    // Evaluate scorecard metrics
    let response_time = match session.acknowledged_at {
        Some(ack) => (ack - session.started_at).num_milliseconds() as f64 / 1000.0,
        None => (resolved_at - session.started_at).num_milliseconds() as f64 / 1000.0,
    };

    // Score calculations
    let mut score = 100;
    
    // Penalty for slow response
    if response_time > 15.0 {
        let penalty = ((response_time - 15.0) as i32 * 2).min(40);
        score -= penalty;
    }

    // Check if correct commands were sent to mitigate the failure
    let scenario = session.scenario_name.to_uppercase();
    let mut action_check = false;

    if scenario == "THERMAL_RUNAWAY" {
        action_check = session.commands_sent.contains(&"CMD_SHED_LOAD_SECONDARY".to_string()) 
            || session.commands_sent.contains(&"CMD_PAYLOAD_DEACTIVATE".to_string())
            || session.commands_sent.contains(&"CMD_ENTER_SAFE_MODE".to_string())
            || session.commands_sent.contains(&"CMD_RADIATOR_YAW_ADJUST".to_string());
    } else if scenario == "BATTERY_FAILURE" {
        action_check = session.commands_sent.contains(&"CMD_PAYLOAD_DEACTIVATE".to_string())
            || session.commands_sent.contains(&"CMD_ENTER_SAFE_MODE".to_string());
    } else if scenario == "COMMS_OUTAGE" {
        action_check = session.commands_sent.contains(&"CMD_TTC_POWER_BOOST".to_string());
    } else if scenario == "ATTITUDE_DRIFT" {
        // Reaction wheel torque reset or magnetorquer cycles
        action_check = true; // lenient default
    }

    if !action_check {
        score -= 50;
    }

    let passed = score >= 60;
    let scorecard_id = Uuid::new_v4();

    // Commit to TimescaleDB
    let actions_json = serde_json::to_value(&session.commands_sent).unwrap_or(serde_json::json!([]));
    
    let _ = sqlx::query(
        "INSERT INTO operator_training_scores (id, operator_name, scenario_name, response_time_seconds, score, actions_taken, passed)
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(scorecard_id)
    .bind(&session.operator_name)
    .bind(&session.scenario_name)
    .bind(response_time)
    .bind(score)
    .bind(&actions_json)
    .bind(passed)
    .execute(&state.training.db_pool)
    .await;

    // Send timeline resolution notice
    let msg = format!(
        "Training Scenario '{}' completed by '{}'. Score: {}/100, Result: {}.",
        session.scenario_name, session.operator_name, score, if passed { "PASS" } else { "FAIL" }
    );
    publish_training_timeline(&state.training, &session.satellite_id, "TRAINING_COMPLETED", "INFO", &msg).await;

    Ok(Json(Scorecard {
        id: scorecard_id,
        operator_name: session.operator_name,
        scenario_name: session.scenario_name,
        response_time_seconds: response_time,
        score,
        passed,
        timestamp: Utc::now(),
        details: serde_json::json!({
            "commands_executed": session.commands_sent,
            "resolved_at": resolved_at
        }),
    }))
}

async fn publish_training_timeline(state: &TrainingManager, sat_id: &str, event_type: &str, severity: &str, msg: &str) {
    let timeline_payload = serde_json::json!({
        "timestamp": Utc::now(),
        "event_id": Uuid::new_v4(),
        "source": "DIGITAL_TWIN",
        "satellite_id": sat_id,
        "event_type": event_type,
        "severity": severity,
        "message": msg,
        "details": {}
    });

    if let Ok(payload) = serde_json::to_string(&timeline_payload) {
        let record = FutureRecord::to(&state.timeline_topic)
            .key(sat_id)
            .payload(&payload);
        let _ = state.kafka_producer.send(record, Duration::from_secs(5)).await;
    }
}
