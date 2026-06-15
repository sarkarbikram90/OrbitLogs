use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use spaceops_models::kafka::KafkaConfigBuilder;
use sqlx::{PgPool, Row};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "Mission Timeline Engine")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8087")]
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

    #[arg(long, env = "TIMELINE_EVENTS_TOPIC", default_value = "timeline.events")]
    timeline_topic: String,

    #[arg(long, env = "HEALTH_TOPIC", default_value = "mission.health")]
    health_topic: String,

    #[arg(long, env = "GROUP_ID", default_value = "mission-timeline-group")]
    group_id: String,
}

struct AppState {
    db_pool: PgPool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TimelineEvent {
    timestamp: chrono::DateTime<chrono::Utc>,
    event_id: Uuid,
    source: String, // "TELEMETRY", "COMMAND", "ALERT", "INCIDENT", "GROUND_STATION", "DIGITAL_TWIN"
    satellite_id: Option<String>,
    event_type: String,
    severity: String,
    message: String,
    details: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct TimelineQuery {
    satellite_id: Option<String>,
    source: Option<String>,
    severity: Option<String>,
    limit: Option<i64>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    // Start tracing
    spaceops_models::telemetry::init_telemetry("mission-timeline", &cli.otel_endpoint)?;

    info!("Starting Mission Timeline Engine...");

    let db_pool = spaceops_models::db::establish_db_connection(&cli.database_url, 5).await?;

    let state = Arc::new(AppState { db_pool });

    // Spawn background consumer for events
    let state_clone = Arc::clone(&state);
    let brokers = cli.kafka_brokers.clone();
    let timeline_topic = cli.timeline_topic.clone();
    let health_topic = cli.health_topic.clone();
    let group_id = cli.group_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_events_consumer(state_clone, brokers, timeline_topic, health_topic, group_id).await {
            error!("Background events consumer failed: {}", e);
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/timeline", get(get_timeline).post(create_timeline_event))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Mission Timeline Engine listening on http://localhost:{}", cli.port);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn run_events_consumer(
    state: Arc<AppState>,
    brokers: String,
    timeline_topic: String,
    health_topic: String,
    group_id: String,
) -> Result<(), anyhow::Error> {
    let mut config = KafkaConfigBuilder::from_env(brokers).build();
    config
        .set("group.id", &group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest");

    let consumer: StreamConsumer = config.create()?;
    consumer.subscribe(&[&timeline_topic, &health_topic])?;

    info!("Timeline consumer subscribed to topics: {}, {}", timeline_topic, health_topic);

    loop {
        match consumer.recv().await {
            Ok(msg) => {
                let topic = msg.topic();
                if let Some(Ok(payload)) = msg.payload_view::<str>() {
                    if topic == timeline_topic {
                        if let Ok(evt) = serde_json::from_str::<TimelineEvent>(payload) {
                            let _ = insert_event(&state.db_pool, &evt).await;
                        }
                    } else if topic == health_topic {
                        // Map mission health events into timeline events
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(payload) {
                            let sat_id = val["satellite_id"].as_str().map(|s| s.to_string());
                            let ostate = val["overall_state"].as_str().unwrap_or("GREEN");
                            let oscore = val["overall_score"].as_i64().unwrap_or(100);
                            let explanation = val["explanation"].as_str().unwrap_or("");

                            let severity = match ostate {
                                "RED" => "CRITICAL",
                                "YELLOW" => "WARNING",
                                _ => "INFO",
                            };

                            let evt = TimelineEvent {
                                timestamp: chrono::Utc::now(),
                                event_id: Uuid::new_v4(),
                                source: "TELEMETRY".to_string(),
                                satellite_id: sat_id,
                                event_type: "HEALTH_ALERT".to_string(),
                                severity: severity.to_string(),
                                message: format!("Satellite health state changed to {} (Score: {}) - {}", ostate, oscore, explanation),
                                details: Some(val),
                            };

                            let _ = insert_event(&state.db_pool, &evt).await;
                        }
                    }
                }
                let _ = consumer.commit_message(&msg, CommitMode::Async);
            }
            Err(e) => {
                error!("Error consuming events for timeline: {}", e);
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

async fn insert_event(pool: &PgPool, evt: &TimelineEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO timeline_events (timestamp, event_id, source, satellite_id, event_type, severity, message, details)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(evt.timestamp)
    .bind(evt.event_id)
    .bind(&evt.source)
    .bind(&evt.satellite_id)
    .bind(&evt.event_type)
    .bind(&evt.severity)
    .bind(&evt.message)
    .bind(&evt.details)
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_timeline(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineEvent>>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(100);

    let mut sql = "SELECT * FROM timeline_events WHERE 1=1".to_string();
    let mut param_index = 1;
    let mut bindings = Vec::new();

    if let Some(ref sat_id) = params.satellite_id {
        sql.push_str(&format!(" AND satellite_id = ${}", param_index));
        bindings.push(sat_id.clone());
        param_index += 1;
    }

    if let Some(ref src) = params.source {
        sql.push_str(&format!(" AND source = ${}", param_index));
        bindings.push(src.clone());
        param_index += 1;
    }

    if let Some(ref sev) = params.severity {
        sql.push_str(&format!(" AND severity = ${}", param_index));
        bindings.push(sev.clone());
        param_index += 1;
    }

    sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${}", param_index));

    let mut query = sqlx::query(&sql);
    for bind_val in bindings {
        query = query.bind(bind_val);
    }
    query = query.bind(limit);

    let rows = query.fetch_all(&state.db_pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let events = rows
        .into_iter()
        .map(|row| TimelineEvent {
            timestamp: row.get("timestamp"),
            event_id: row.get("event_id"),
            source: row.get("source"),
            satellite_id: row.get("satellite_id"),
            event_type: row.get("event_type"),
            severity: row.get("severity"),
            message: row.get("message"),
            details: row.get("details"),
        })
        .collect();

    Ok(Json(events))
}

async fn create_timeline_event(
    State(state): State<Arc<AppState>>,
    Json(evt): Json<TimelineEvent>,
) -> Result<(StatusCode, Json<TimelineEvent>), (StatusCode, Json<serde_json::Value>)> {
    insert_event(&state.db_pool, &evt).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok((StatusCode::CREATED, Json(evt)))
}
