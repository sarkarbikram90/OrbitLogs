use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::{PgPool, Row};
use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionPlan {
    pub id: Uuid,
    pub satellite_id: String,
    pub plan_type: String, // 'PAYLOAD', 'IMAGE', 'MANEUVER', 'UPDATE', 'COMMAND'
    pub description: String,
    pub scheduled_time: DateTime<Utc>,
    pub status: String, // 'DRAFT', 'REVIEW', 'APPROVED', 'SCHEDULED', 'EXECUTED', 'CANCELLED'
    pub created_by: String,
    pub approved_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePlanRequest {
    pub satellite_id: String,
    pub plan_type: String,
    pub description: String,
    pub scheduled_time: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
    pub operator: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanQuery {
    pub satellite_id: Option<String>,
}

pub struct PlannerState {
    pub db_pool: PgPool,
    pub kafka_producer: FutureProducer,
    pub timeline_topic: String,
}

pub async fn list_plans(
    State(state): State<Arc<crate::ServiceState>>,
    Query(query): Query<PlanQuery>,
) -> Result<Json<Vec<MissionPlan>>, (StatusCode, String)> {
    let rows = if let Some(sat_id) = query.satellite_id {
        sqlx::query("SELECT * FROM mission_planning WHERE satellite_id = $1 ORDER BY scheduled_time ASC")
            .bind(sat_id)
            .fetch_all(&state.db_pool)
            .await
    } else {
        sqlx::query("SELECT * FROM mission_planning ORDER BY scheduled_time ASC")
            .fetch_all(&state.db_pool)
            .await
    }.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let plans = rows.into_iter().map(|row| MissionPlan {
        id: row.get("id"),
        satellite_id: row.get("satellite_id"),
        plan_type: row.get("plan_type"),
        description: row.get("description"),
        scheduled_time: row.get("scheduled_time"),
        status: row.get("status"),
        created_by: row.get("created_by"),
        approved_by: row.get("approved_by"),
        created_at: row.get("created_at"),
    }).collect();

    Ok(Json(plans))
}

pub async fn create_plan(
    State(state): State<Arc<crate::ServiceState>>,
    Json(req): Json<CreatePlanRequest>,
) -> Result<(StatusCode, Json<MissionPlan>), (StatusCode, String)> {
    let id = Uuid::new_v4();
    let status = "DRAFT".to_string();
    let created_at = Utc::now();

    sqlx::query(
        "INSERT INTO mission_planning (id, satellite_id, plan_type, description, scheduled_time, status, created_by, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(id)
    .bind(&req.satellite_id)
    .bind(&req.plan_type)
    .bind(&req.description)
    .bind(req.scheduled_time)
    .bind(&status)
    .bind(&req.created_by)
    .bind(created_at)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let plan = MissionPlan {
        id,
        satellite_id: req.satellite_id,
        plan_type: req.plan_type,
        description: req.description,
        scheduled_time: req.scheduled_time,
        status,
        created_by: req.created_by,
        approved_by: None,
        created_at,
    };

    publish_planning_event(&state.planner, &plan, "PLAN_CREATED", "Plan created as draft").await;

    Ok((StatusCode::CREATED, Json(plan)))
}

pub async fn update_plan_status(
    State(state): State<Arc<crate::ServiceState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<MissionPlan>, (StatusCode, String)> {
    // Validate status value
    if !["DRAFT", "REVIEW", "APPROVED", "SCHEDULED", "EXECUTED", "CANCELLED"].contains(&req.status.to_uppercase().as_str()) {
        return Err((StatusCode::BAD_REQUEST, "Invalid plan status".to_string()));
    }

    let status = req.status.to_uppercase();
    
    // Fetch current to log transition
    let row = sqlx::query("SELECT * FROM mission_planning WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?;

    let sat_id: String = row.get("satellite_id");
    let plan_type: String = row.get("plan_type");
    let description: String = row.get("description");
    let scheduled_time: DateTime<Utc> = row.get("scheduled_time");
    let created_by: String = row.get("created_by");
    let created_at: DateTime<Utc> = row.get("created_at");

    let approved_by = if status == "APPROVED" || status == "SCHEDULED" {
        Some(req.operator.clone())
    } else {
        row.get("approved_by")
    };

    sqlx::query("UPDATE mission_planning SET status = $1, approved_by = $2 WHERE id = $3")
        .bind(&status)
        .bind(&approved_by)
        .bind(id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let plan = MissionPlan {
        id,
        satellite_id: sat_id,
        plan_type,
        description,
        scheduled_time,
        status: status.clone(),
        created_by,
        approved_by,
        created_at,
    };

    let event_type = format!("PLAN_{}", status);
    let msg = format!("Plan status updated to {}", status);
    publish_planning_event(&state.planner, &plan, &event_type, &msg).await;

    Ok(Json(plan))
}

async fn publish_planning_event(state: &PlannerState, plan: &MissionPlan, event_type: &str, msg: &str) {
    let timeline_payload = serde_json::json!({
        "timestamp": Utc::now(),
        "event_id": Uuid::new_v4(),
        "source": "COMMAND", // Categorize as operational command flow
        "satellite_id": plan.satellite_id,
        "event_type": event_type,
        "severity": "INFO",
        "message": format!("{}: {} schedule of type '{}'", msg, plan.satellite_id, plan.plan_type),
        "details": {
            "plan_id": plan.id,
            "plan_type": plan.plan_type,
            "status": plan.status,
            "description": plan.description,
            "scheduled_time": plan.scheduled_time,
            "operator": plan.created_by
        }
    });

    if let Ok(payload) = serde_json::to_string(&timeline_payload) {
        let record = FutureRecord::to(&state.timeline_topic)
            .key(&plan.satellite_id)
            .payload(&payload);
        let _ = state.kafka_producer.send(record, Duration::from_secs(5)).await;
    }
}
