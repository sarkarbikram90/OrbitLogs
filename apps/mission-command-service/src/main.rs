use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use spaceops_models::kafka::KafkaConfigBuilder;
use sqlx::{PgPool, Row};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use opentelemetry::trace::TraceContextExt;
use uuid::Uuid;
use subtle::ConstantTimeEq;

#[derive(Parser, Debug)]
#[command(name = "Mission Command Service")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8085")]
    port: u16,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://postgres:postgrespassword@localhost:5432/spaceops"
    )]
    database_url: String,

    #[arg(
        long,
        env = "OTEL_EXPORTER_OTLP_ENDPOINT",
        default_value = "http://localhost:4317"
    )]
    otel_endpoint: String,

    #[arg(long, env = "COMMAND_EVENTS_TOPIC", default_value = "command.events")]
    command_events_topic: String,

    #[arg(long, env = "TIMELINE_EVENTS_TOPIC", default_value = "timeline.events")]
    timeline_events_topic: String,

    #[arg(long, env = "API_KEY")]
    api_key: Option<String>,

    #[arg(
        long,
        env = "CORS_ALLOWED_ORIGIN",
        default_value = "http://localhost:3000"
    )]
    cors_allowed_origin: String,
}

struct AppState {
    db_pool: PgPool,
    kafka_producer: FutureProducer,
    command_topic: String,
    timeline_topic: String,
    api_key: String,
    cors_allowed_origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Command {
    id: Uuid,
    satellite_id: String,
    command_type: String,
    status: String,
    parameters: serde_json::Value,
    created_by: String,
    approved_by: Option<String>,
    scheduled_time: Option<chrono::DateTime<chrono::Utc>>,
    executed_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateCommandRequest {
    satellite_id: String,
    command_type: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    satellite_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    role: String,
    exp: usize,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    api_key: String,
    role: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    token: String,
}

// Simple RBAC roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
enum Role {
    Operator = 1,
    FlightDirector = 2,
    MissionManager = 3,
    Administrator = 4,
}

impl Role {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "OPERATOR" => Some(Role::Operator),
            "FLIGHTDIRECTOR" | "FLIGHT_DIRECTOR" | "FLIGHT DIRECTOR" => Some(Role::FlightDirector),
            "MISSIONMANAGER" | "MISSION_MANAGER" | "MISSION MANAGER" => Some(Role::MissionManager),
            "ADMINISTRATOR" | "ADMIN" => Some(Role::Administrator),
            _ => None,
        }
    }
}

pub fn secure_compare(provided: &str, expected: &str) -> bool {
    if provided.len() != expected.len() {
        let _ = expected.as_bytes().ct_eq(expected.as_bytes());
        false
    } else {
        provided.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() == 1
    }
}

fn check_role(
    state: &AppState,
    headers: &HeaderMap,
    required: Role,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Missing Authorization Bearer token" })),
            )
        })?;

    let mut validation = jsonwebtoken::Validation::default();
    validation.validate_exp = true;

    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(state.api_key.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": format!("Invalid token: {}", e) })),
        )
    })?;

    let role = Role::from_str(&token_data.claims.role).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid role in token claims" })),
        )
    })?;

    if role >= required {
        Ok(token_data.claims.role)
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": format!("Insufficient permissions: required {:?}, got {:?}", required, role) })),
        ))
    }
}

async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    if !secure_compare(&req.api_key, &state.api_key) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid API key" })),
        ));
    }

    let Some(role) = Role::from_str(&req.role) else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid role specified" })),
        ));
    };

    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: "spaceops-operator".to_string(),
        role: format!("{:?}", role),
        exp,
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(state.api_key.as_bytes()),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Token generation failed: {}", e) })),
        )
    })?;

    Ok(Json(LoginResponse { token }))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    // Start telemetry
    spaceops_models::telemetry::init_telemetry("mission-command-service", &cli.otel_endpoint)?;

    info!("Starting Mission Command Service...");

    let api_key = cli.api_key.clone()
        .or_else(|| spaceops_models::load_secret("API_KEY"))
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("API key is required. Set env var API_KEY, pass --api-key, or use Docker secrets API_KEY."))?;

    let db_pool = spaceops_models::db::establish_db_connection(&cli.database_url, 5).await?;

    let producer_config = KafkaConfigBuilder::from_env(cli.kafka_brokers.clone()).build();
    let kafka_producer: FutureProducer = producer_config.create()?;

    let app_state = Arc::new(AppState {
        db_pool,
        kafka_producer,
        command_topic: cli.command_events_topic,
        timeline_topic: cli.timeline_events_topic,
        api_key,
        cors_allowed_origin: cli.cors_allowed_origin.clone(),
    });

    let cors = if let Ok(origin_val) = cli.cors_allowed_origin.parse::<axum::http::HeaderValue>() {
        CorsLayer::new()
            .allow_origin(origin_val)
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::OPTIONS])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderName::from_static("x-user-role"),
            ])
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    let app = Router::new()
        .route("/api/auth/login", post(login_handler))
        .route("/api/commands", get(list_commands).post(create_command))
        .route("/api/commands/:id/approve", post(approve_command))
        .route("/api/commands/:id/queue", post(queue_command))
        .route("/api/commands/:id/execute", post(execute_command))
        .layer(cors)
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Mission Command Service listening on http://localhost:{}", cli.port);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}


async fn list_commands(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<Command>>, (StatusCode, Json<serde_json::Value>)> {
    let query_str = if let Some(ref sat_id) = params.satellite_id {
        sqlx::query("SELECT * FROM commands WHERE satellite_id = $1 ORDER BY created_at DESC")
            .bind(sat_id)
    } else {
        sqlx::query("SELECT * FROM commands ORDER BY created_at DESC")
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
async fn create_command(
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
         VALUES ($1, $2, $3, $4, $5, $6)"
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
    log_audit(&state, id, None, status, &user_name, "Command registered in system").await?;

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
async fn approve_command(
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

    log_audit(&state, id, Some(&prev_status), "APPROVED", &approver, "Command approved by flight operations").await?;

    publish_command_event(&state, &cmd).await;
    publish_timeline_event(&state, &cmd, "INFO", "Command approved").await;

    Ok(Json(cmd))
}

#[instrument(skip(state, headers))]
async fn queue_command(
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

    log_audit(&state, id, Some(&prev_status), "QUEUED", &operator, "Command queued for transmission").await?;

    publish_command_event(&state, &cmd).await;
    publish_timeline_event(&state, &cmd, "INFO", "Command queued").await;

    Ok(Json(cmd))
}

#[instrument(skip(state, headers))]
async fn execute_command(
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

    // Execute state flow simulation
    let prev_status = cmd.status.clone();
    cmd.status = "TRANSMITTED".to_string();
    cmd.executed_at = Some(chrono::Utc::now());

    sqlx::query("UPDATE commands SET status = 'TRANSMITTED', executed_at = $1 WHERE id = $2")
        .bind(cmd.executed_at)
        .bind(id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;

    log_audit(&state, id, Some(&prev_status), "TRANSMITTED", &operator, "Uplink carrier lock confirmed, command transmitted").await?;
    publish_command_event(&state, &cmd).await;
    publish_timeline_event(&state, &cmd, "INFO", "Command transmitted to satellite").await;

    // Simulate async execution success/failure transition
    let state_clone = Arc::clone(&state);
    let operator_clone = operator.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let mut cur_cmd = match fetch_command_by_id(&state_clone, id).await {
            Ok(c) => c,
            _ => return,
        };

        // Transition: RECEIVED
        let t_prev = cur_cmd.status.clone();
        cur_cmd.status = "RECEIVED".to_string();
        let _ = sqlx::query("UPDATE commands SET status = 'RECEIVED' WHERE id = $1").bind(id).execute(&state_clone.db_pool).await;
        let _ = log_audit(&state_clone, id, Some(&t_prev), "RECEIVED", &operator_clone, "Spacecraft acknowledged execution start").await;
        publish_command_event(&state_clone, &cur_cmd).await;

        tokio::time::sleep(Duration::from_secs(2)).await;

        // Transition: EXECUTED -> VERIFIED or FAILED
        let is_fail = cur_cmd.parameters.get("should_fail").and_then(|v| v.as_bool()).unwrap_or(false);
        let final_status = if is_fail { "FAILED" } else { "VERIFIED" };

        let t_prev2 = cur_cmd.status.clone();
        cur_cmd.status = final_status.to_string();
        let _ = sqlx::query("UPDATE commands SET status = $1 WHERE id = $2").bind(final_status).bind(id).execute(&state_clone.db_pool).await;

        let notes = if is_fail { "Command execution failed: validation bounds exceeded on payload" } else { "Command execution completed and verified via telemetry feedback loops" };
        let severity = if is_fail { "CRITICAL" } else { "INFO" };

        let _ = log_audit(&state_clone, id, Some(&t_prev2), final_status, &operator_clone, notes).await;
        publish_command_event(&state_clone, &cur_cmd).await;
        publish_timeline_event(&state_clone, &cur_cmd, severity, notes).await;
    });

    Ok(Json(cmd))
}

async fn fetch_command_by_id(state: &AppState, id: Uuid) -> Result<Command, (StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query("SELECT * FROM commands WHERE id = $1")
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

async fn log_audit(
    state: &AppState,
    command_id: Uuid,
    from: Option<&str>,
    to: &str,
    by: &str,
    notes: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let trace_id = Span::current().context().span().span_context().trace_id().to_string();
    sqlx::query(
        "INSERT INTO command_audit_trail (command_id, transition_from, transition_to, changed_by, trace_id, notes)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(command_id)
    .bind(from)
    .bind(to)
    .bind(by)
    .bind(trace_id)
    .bind(notes)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;

    Ok(())
}

async fn publish_command_event(state: &AppState, cmd: &Command) {
    if let Ok(payload) = serde_json::to_string(cmd) {
        let key = format!("{}:{}", cmd.satellite_id, cmd.id);
        let record = FutureRecord::to(&state.command_topic)
            .key(&key)
            .payload(&payload);
        if let Err((e, _)) = state.kafka_producer.send(record, Duration::from_secs(5)).await {
            error!("Failed to publish command event: {}", e);
        }
    }
}

async fn publish_timeline_event(state: &AppState, cmd: &Command, severity: &str, msg: &str) {
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
        if let Err((e, _)) = state.kafka_producer.send(record, Duration::from_secs(5)).await {
            error!("Failed to publish command to timeline: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_compare() {
        assert!(secure_compare("secretkey", "secretkey"));
        assert!(!secure_compare("secretkey", "wrongkey"));
        assert!(!secure_compare("secretkey", "short"));
    }

    #[test]
    fn test_role_parsing() {
        assert_eq!(Role::from_str("OPERATOR"), Some(Role::Operator));
        assert_eq!(Role::from_str("FLIGHT_DIRECTOR"), Some(Role::FlightDirector));
        assert_eq!(Role::from_str("MISSION_MANAGER"), Some(Role::MissionManager));
        assert_eq!(Role::from_str("ADMINISTRATOR"), Some(Role::Administrator));
        assert_eq!(Role::from_str("UNKNOWN"), None);
    }

    #[test]
    fn test_role_hierarchy() {
        assert!(Role::Operator < Role::FlightDirector);
        assert!(Role::FlightDirector < Role::MissionManager);
        assert!(Role::MissionManager < Role::Administrator);
        
        assert!(Role::Administrator >= Role::Operator);
        assert!(Role::FlightDirector >= Role::Operator);
        assert!(Role::Operator >= Role::Operator);
    }

    #[test]
    fn test_jwt_encoding_decoding() {
        let secret = "test-secret-key";
        let claims = Claims {
            sub: "test-user".to_string(),
            role: "OPERATOR".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        ).unwrap();

        let mut validation = jsonwebtoken::Validation::default();
        validation.validate_exp = true;

        let token_data = jsonwebtoken::decode::<Claims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        ).unwrap();

        assert_eq!(token_data.claims.sub, "test-user");
        assert_eq!(token_data.claims.role, "OPERATOR");
    }
}
