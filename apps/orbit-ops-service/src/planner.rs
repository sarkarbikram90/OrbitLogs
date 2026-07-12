use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionPlan {
    pub id: Uuid,
    pub satellite_id: orbitlogs_models::SatelliteId,
    pub plan_type: orbitlogs_models::MissionPlanType,
    pub description: String,
    pub objectives: String,
    pub required_contacts: serde_json::Value,
    pub required_commands: serde_json::Value,
    pub required_payloads: serde_json::Value,
    pub scheduled_time: DateTime<Utc>,
    pub status: orbitlogs_models::MissionPlanStatus,
    pub created_by: String,
    pub approved_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePlanRequest {
    pub satellite_id: orbitlogs_models::SatelliteId,
    pub plan_type: orbitlogs_models::MissionPlanType,
    pub description: String,
    pub objectives: String,
    pub required_contacts: Option<serde_json::Value>,
    pub required_commands: Option<serde_json::Value>,
    pub required_payloads: Option<serde_json::Value>,
    pub scheduled_time: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: orbitlogs_models::MissionPlanStatus,
    pub operator: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanQuery {
    pub satellite_id: Option<orbitlogs_models::SatelliteId>,
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
        sqlx::query("SELECT id, satellite_id, plan_type, description, objectives, required_contacts, required_commands, required_payloads, scheduled_time, status, created_by, approved_by, created_at FROM mission_plans WHERE satellite_id = $1 ORDER BY scheduled_time ASC")
            .bind(sat_id)
            .fetch_all(&state.planner.db_pool)
            .await
    } else {
        sqlx::query("SELECT id, satellite_id, plan_type, description, objectives, required_contacts, required_commands, required_payloads, scheduled_time, status, created_by, approved_by, created_at FROM mission_plans ORDER BY scheduled_time ASC")
            .fetch_all(&state.planner.db_pool)
            .await
    }.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let plans = rows
        .into_iter()
        .map(|row| MissionPlan {
            id: row.get("id"),
            satellite_id: row.get("satellite_id"),
            plan_type: row.get("plan_type"),
            description: row.get("description"),
            objectives: row.get("objectives"),
            required_contacts: row.get("required_contacts"),
            required_commands: row.get("required_commands"),
            required_payloads: row.get("required_payloads"),
            scheduled_time: row.get("scheduled_time"),
            status: row.get("status"),
            created_by: row.get("created_by"),
            approved_by: row.get("approved_by"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(plans))
}

pub async fn create_plan(
    State(state): State<Arc<crate::ServiceState>>,
    Json(req): Json<CreatePlanRequest>,
) -> Result<(StatusCode, Json<MissionPlan>), (StatusCode, String)> {
    let id = Uuid::new_v4();
    let status = orbitlogs_models::MissionPlanStatus::Draft;
    let created_at = Utc::now();

    let contacts = req.required_contacts.unwrap_or(serde_json::json!([]));
    let commands = req.required_commands.unwrap_or(serde_json::json!([]));
    let payloads = req.required_payloads.unwrap_or(serde_json::json!([]));

    sqlx::query(
        "INSERT INTO mission_plans (id, satellite_id, plan_type, description, objectives, required_contacts, required_commands, required_payloads, scheduled_time, status, created_by, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"
    )
    .bind(id)
    .bind(&req.satellite_id)
    .bind(req.plan_type)
    .bind(&req.description)
    .bind(&req.objectives)
    .bind(&contacts)
    .bind(&commands)
    .bind(&payloads)
    .bind(req.scheduled_time)
    .bind(status)
    .bind(&req.created_by)
    .bind(created_at)
    .execute(&state.planner.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let plan = MissionPlan {
        id,
        satellite_id: req.satellite_id,
        plan_type: req.plan_type,
        description: req.description,
        objectives: req.objectives,
        required_contacts: contacts,
        required_commands: commands,
        required_payloads: payloads,
        scheduled_time: req.scheduled_time,
        status,
        created_by: req.created_by,
        approved_by: None,
        created_at,
    };

    publish_planning_event(
        &state.planner,
        &plan,
        "PLAN_CREATED",
        "Plan created as draft",
    )
    .await;

    Ok((StatusCode::CREATED, Json(plan)))
}

pub async fn update_plan_status(
    State(state): State<Arc<crate::ServiceState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<MissionPlan>, (StatusCode, String)> {
    let status = req.status;

    let row = sqlx::query("SELECT * FROM mission_plans WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.planner.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Plan not found".to_string()))?;

    let sat_id: orbitlogs_models::SatelliteId = row.get("satellite_id");
    let plan_type: orbitlogs_models::MissionPlanType = row.get("plan_type");
    let description: String = row.get("description");
    let objectives: String = row.get("objectives");
    let required_contacts: serde_json::Value = row.get("required_contacts");
    let required_commands: serde_json::Value = row.get("required_commands");
    let required_payloads: serde_json::Value = row.get("required_payloads");
    let scheduled_time: DateTime<Utc> = row.get("scheduled_time");
    let created_by: String = row.get("created_by");
    let created_at: DateTime<Utc> = row.get("created_at");

    let approved_by = if status == orbitlogs_models::MissionPlanStatus::Approved
        || status == orbitlogs_models::MissionPlanStatus::Scheduled
    {
        Some(req.operator.clone())
    } else {
        row.get("approved_by")
    };

    sqlx::query("UPDATE mission_plans SET status = $1, approved_by = $2 WHERE id = $3")
        .bind(status)
        .bind(&approved_by)
        .bind(id)
        .execute(&state.planner.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let plan = MissionPlan {
        id,
        satellite_id: sat_id,
        plan_type,
        description,
        objectives,
        required_contacts,
        required_commands,
        required_payloads,
        scheduled_time,
        status,
        created_by,
        approved_by,
        created_at,
    };

    let event_type = format!("PLAN_{}", status);
    let msg = format!("Plan status updated to {}", status);
    publish_planning_event(&state.planner, &plan, &event_type, &msg).await;

    Ok(Json(plan))
}

async fn publish_planning_event(
    state: &PlannerState,
    plan: &MissionPlan,
    event_type: &str,
    msg: &str,
) {
    let timeline_payload = serde_json::json!({
        "timestamp": Utc::now(),
        "event_id": Uuid::new_v4(),
        "source": "COMMAND",
        "satellite_id": plan.satellite_id,
        "event_type": event_type,
        "severity": "INFO",
        "message": format!("{}: {} schedule of type '{}'", msg, plan.satellite_id, plan.plan_type),
        "details": {
            "plan_id": plan.id,
            "plan_type": plan.plan_type,
            "status": plan.status,
            "description": plan.description,
            "objectives": plan.objectives,
            "scheduled_time": plan.scheduled_time,
            "operator": plan.created_by
        }
    });

    if let Ok(payload) = serde_json::to_string(&timeline_payload) {
        let record = FutureRecord::to(&state.timeline_topic)
            .key(plan.satellite_id.as_str())
            .payload(&payload);
        let _ = state
            .kafka_producer
            .send(record, Duration::from_secs(5))
            .await;
    }
}
