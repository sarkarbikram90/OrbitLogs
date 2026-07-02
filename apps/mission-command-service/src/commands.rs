use crate::audit::log_audit;
use crate::auth::{check_role, Role};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use rdkafka::producer::FutureRecord;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, instrument};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: Uuid,
    pub satellite_id: String,
    pub command_type: String,
    pub status: String,
    pub parameters: serde_json::Value,
    pub created_by: String,
    pub approved_by: Option<String>,
    pub scheduled_time: Option<chrono::DateTime<chrono::Utc>>,
    pub executed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommandRequest {
    pub satellite_id: String,
    pub command_type: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub satellite_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_commands(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<Command>>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(50).clamp(1, 1000);
    let offset = params.offset.unwrap_or(0).max(0);

    let query_str = if let Some(ref sat_id) = params.satellite_id {
        sqlx::query("SELECT id, satellite_id, command_type, status, parameters, created_by, approved_by, scheduled_time, executed_at, created_at FROM commands WHERE satellite_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3")
            .bind(sat_id)
            .bind(limit)
            .bind(offset)
    } else {
        sqlx::query("SELECT id, satellite_id, command_type, status, parameters, created_by, approved_by, scheduled_time, executed_at, created_at FROM commands ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
    };

    let rows = query_str.fetch_all(&state.db_pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let commands = rows
        .into_iter()
        .map(|row| Command {
            id: row.get("id"),
            satellite_id: row.get("satellite_id"),
            command_type: row.get("command_type"),
            status: row.get("status"),
            parameters: row.get("parameters"),
            created_by: row.get("created_by"),
            approved_by: row.get("approved_by"),
            scheduled_time: row.get("scheduled_time"),
            executed_at: row.get("executed_at"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(commands))
}

#[instrument(skip(state, headers, req))]
pub async fn create_command(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateCommandRequest>,
) -> Result<(StatusCode, Json<Command>), (StatusCode, Json<serde_json::Value>)> {
    let user_name = check_role(&state, &headers, Role::Operator)?;

    let id = Uuid::new_v4();
    let status = "CREATED";

    // Write to DB
    sqlx::query(
        "INSERT INTO commands (id, satellite_id, command_type, status, parameters, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(&req.satellite_id)
    .bind(&req.command_type)
    .bind(status)
    .bind(&req.parameters)
    .bind(&user_name)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    // Audit trail
    log_audit(
        &state,
        id,
        None,
        status,
        &user_name,
        "Command registered in system",
    )
    .await?;

    let cmd = Command {
        id,
        satellite_id: req.satellite_id,
        command_type: req.command_type,
        status: status.to_string(),
        parameters: req.parameters,
        created_by: user_name,
        approved_by: None,
        scheduled_time: None,
        executed_at: None,
        created_at: chrono::Utc::now(),
    };

    // Publish event
    publish_command_event(&state, &cmd).await;
    publish_timeline_event(&state, &cmd, "INFO", "Command created").await;

    Ok((StatusCode::CREATED, Json(cmd)))
}

#[instrument(skip(state, headers))]
pub async fn approve_command(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Command>, (StatusCode, Json<serde_json::Value>)> {
    let approver = check_role(&state, &headers, Role::FlightDirector)?;

    // Fetch existing
    let mut cmd = fetch_command_by_id(&state, id).await?;

    if cmd.status != "CREATED" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Command must be in CREATED state to approve" })),
        ));
    }

    let prev_status = cmd.status.clone();
    cmd.status = "APPROVED".to_string();
    cmd.approved_by = Some(approver.clone());

    sqlx::query("UPDATE commands SET status = 'APPROVED', approved_by = $1 WHERE id = $2")
        .bind(&approver)
        .bind(id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    log_audit(
        &state,
        id,
        Some(&prev_status),
        "APPROVED",
        &approver,
        "Command approved by flight operations",
    )
    .await?;

    publish_command_event(&state, &cmd).await;
    publish_timeline_event(&state, &cmd, "INFO", "Command approved").await;

    Ok(Json(cmd))
}

#[instrument(skip(state, headers))]
pub async fn queue_command(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Command>, (StatusCode, Json<serde_json::Value>)> {
    let operator = check_role(&state, &headers, Role::Operator)?;

    let mut cmd = fetch_command_by_id(&state, id).await?;

    if cmd.status != "APPROVED" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Command must be APPROVED before queuing" })),
        ));
    }

    let prev_status = cmd.status.clone();
    cmd.status = "QUEUED".to_string();
    cmd.scheduled_time = Some(chrono::Utc::now());

    sqlx::query("UPDATE commands SET status = 'QUEUED', scheduled_time = $1 WHERE id = $2")
        .bind(cmd.scheduled_time)
        .bind(id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    log_audit(
        &state,
        id,
        Some(&prev_status),
        "QUEUED",
        &operator,
        "Command queued for transmission",
    )
    .await?;

    publish_command_event(&state, &cmd).await;
    publish_timeline_event(&state, &cmd, "INFO", "Command queued").await;

    Ok(Json(cmd))
}

#[instrument(skip(state, headers))]
pub async fn execute_command(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Command>, (StatusCode, Json<serde_json::Value>)> {
    let operator = check_role(&state, &headers, Role::Operator)?;

    let mut cmd = fetch_command_by_id(&state, id).await?;

    if cmd.status != "QUEUED" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Command must be QUEUED to execute" })),
        ));
    }

    // Execute state flow simulation - transition to TRANSMITTED
    let prev_status = cmd.status.clone();
    cmd.status = "TRANSMITTED".to_string();
    cmd.executed_at = Some(chrono::Utc::now());

    sqlx::query("UPDATE commands SET status = 'TRANSMITTED', executed_at = $1 WHERE id = $2")
        .bind(cmd.executed_at)
        .bind(id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    log_audit(
        &state,
        id,
        Some(&prev_status),
        "TRANSMITTED",
        &operator,
        "Uplink carrier lock confirmed, command transmitted",
    )
    .await?;
    publish_command_event(&state, &cmd).await;
    publish_timeline_event(&state, &cmd, "INFO", "Command transmitted to satellite").await;

    Ok(Json(cmd))
}

pub async fn fetch_command_by_id(
    state: &AppState,
    id: Uuid,
) -> Result<Command, (StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query("SELECT id, satellite_id, command_type, status, parameters, created_by, approved_by, scheduled_time, executed_at, created_at FROM commands WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Command not found" }))))?;

    Ok(Command {
        id: row.get("id"),
        satellite_id: row.get("satellite_id"),
        command_type: row.get("command_type"),
        status: row.get("status"),
        parameters: row.get("parameters"),
        created_by: row.get("created_by"),
        approved_by: row.get("approved_by"),
        scheduled_time: row.get("scheduled_time"),
        executed_at: row.get("executed_at"),
        created_at: row.get("created_at"),
    })
}

pub async fn publish_command_event(state: &AppState, cmd: &Command) {
    if let Ok(payload) = serde_json::to_string(cmd) {
        let key = format!("{}:{}", cmd.satellite_id, cmd.id);
        let record = FutureRecord::to(&state.command_topic)
            .key(&key)
            .payload(&payload);
        if let Err((e, _)) = state
            .kafka_producer
            .send(record, Duration::from_secs(5))
            .await
        {
            error!("Failed to publish command event: {}", e);
        }
    }
}

pub async fn publish_timeline_event(state: &AppState, cmd: &Command, severity: &str, msg: &str) {
    let timeline_payload = serde_json::json!({
        "timestamp": chrono::Utc::now(),
        "event_id": Uuid::new_v4(),
        "source": "COMMAND",
        "satellite_id": cmd.satellite_id,
        "event_type": format!("COMMAND_{}", cmd.status),
        "severity": severity,
        "message": format!("{}: Command '{}' is currently {}", msg, cmd.command_type, cmd.status),
        "details": {
            "command_id": cmd.id,
            "command_type": cmd.command_type,
            "status": cmd.status,
            "operator": cmd.created_by
        }
    });

    if let Ok(payload) = serde_json::to_string(&timeline_payload) {
        let record = FutureRecord::to(&state.timeline_topic)
            .key(&cmd.satellite_id)
            .payload(&payload);
        if let Err((e, _)) = state
            .kafka_producer
            .send(record, Duration::from_secs(5))
            .await
        {
            error!("Failed to publish command to timeline: {}", e);
        }
    }
}
