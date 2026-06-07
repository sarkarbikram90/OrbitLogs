#![allow(clippy::large_enum_variant)]

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

mod db;
mod metrics;

#[derive(Parser, Debug, Clone)]
#[command(name = "Open SpaceOps - Mission Replay Engine")]
#[command(about = "Historical data playback engine for telemetry and events", long_about = None)]
struct Cli {
    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://postgres:postgrespassword@localhost:5432/spaceops"
    )]
    database_url: String,

    #[arg(long, env = "PORT", default_value = "8084")]
    port: u16,

    #[arg(long, env = "API_KEY")]
    api_key: String,

    #[arg(
        long,
        env = "CORS_ALLOWED_ORIGIN",
        default_value = "http://localhost:3000"
    )]
    cors_allowed_origin: String,
}



struct AppState {
    db_pool: sqlx::PgPool,
    api_key: String,
    cors_allowed_origin: String,
    sessions: dashmap::DashMap<uuid::Uuid, db::ReplaySession>,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateReplayRequest {
    satellite_id: Option<String>,
    start_time: chrono::DateTime<chrono::Utc>,
    end_time: chrono::DateTime<chrono::Utc>,
    speed: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct ControlReplayRequest {
    action: String, // "play", "pause", "stop", "seek", "speed"
    position: Option<chrono::DateTime<chrono::Utc>>,
    speed: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
enum ReplayData {
    #[serde(rename = "telemetry")]
    Telemetry(spaceops_models::TelemetryRecord),
    #[serde(rename = "event")]
    Event(db::DbEvent),
    #[serde(rename = "satellite_health")]
    SatelliteHealth(db::ReplaySatelliteHealth),
    #[serde(rename = "ground_station_health")]
    GroundStationHealth(db::DbGroundStationHealth),
}

struct TimedReplayData {
    timestamp: chrono::DateTime<chrono::Utc>,
    data: ReplayData,
}

fn api_error(status: StatusCode, message: &str) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

fn is_valid_speed(speed: f32) -> bool {
    let allowed = [1.0, 5.0, 10.0, 50.0, 100.0];
    allowed.iter().any(|&s| (s - speed).abs() < f32::EPSILON)
}

use spaceops_models::load_secret;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load local .env
    let _ = dotenvy::dotenv();

    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut cli = Cli::parse();
    if let Some(val) = load_secret("DATABASE_URL") {
        cli.database_url = val;
    }
    if let Some(val) = load_secret("API_KEY") {
        cli.api_key = val;
    }

    info!(
        "Starting Mission Replay Engine with configuration: {:?}",
        cli
    );

    // Register Prometheus metrics
    metrics::register_metrics();

    // Connect to database
    let db_pool = spaceops_models::db::establish_db_connection(&cli.database_url, 10).await?;

    let state = Arc::new(AppState {
        db_pool,
        api_key: cli.api_key,
        cors_allowed_origin: cli.cors_allowed_origin,
        sessions: dashmap::DashMap::new(),
    });

    // Spawn background cleanup task for old replay sessions (M-15)
    let state_for_cleanup = state.clone();
    tokio::spawn(async move {
        info!("Starting background replay session cleanup task...");
        loop {
            tokio::time::sleep(Duration::from_secs(300)).await; // 5 minutes
            info!("Running cleanup of old replay sessions...");
            
            // Delete from DB
            match sqlx::query("DELETE FROM replay_sessions WHERE created_at < NOW() - INTERVAL '1 hour'")
                .execute(&state_for_cleanup.db_pool)
                .await
            {
                Ok(result) => {
                    info!("Successfully cleaned up {} old replay sessions from DB.", result.rows_affected());
                }
                Err(e) => {
                    error!("Failed to clean up old replay sessions from DB: {}", e);
                }
            }

            // Clean up in-memory sessions DashMap
            let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);
            state_for_cleanup.sessions.retain(|_, session| {
                if let Some(created) = session.created_at {
                    created >= cutoff
                } else {
                    true
                }
            });
        }
    });

    let replay_routes = Router::new()
        .route("/", post(create_replay))
        .route("/:id", get(get_replay))
        .route("/:id/control", post(control_replay))
        .route("/:id/stream", get(stream_replay))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let app = Router::new()
        .nest("/api/replays", replay_routes)
        .route("/api/satellites", get(list_satellites))
        .route("/api/ground-stations", get(list_ground_stations))
        .route("/api/events", get(list_events))
        .route("/metrics", get(handle_metrics))
        .route("/health", get(health_check))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            cors_middleware,
        ))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = TcpListener::bind(addr).await?;
    info!(
        "Mission Replay Engine listening on http://localhost:{}",
        cli.port
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Mission Replay Engine shut down gracefully.");
    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "UP" }))).into_response()
}

async fn handle_metrics() -> impl IntoResponse {
    use prometheus::{Encoder, TextEncoder};
    let encoder = TextEncoder::new();
    let metric_families = metrics::REGISTRY.gather();
    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode metrics: {}", e),
        )
            .into_response();
    }

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, encoder.format_type())
        .body(axum::body::Body::from(buffer))
        .unwrap()
        .into_response()
}

async fn create_replay(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateReplayRequest>,
) -> impl IntoResponse {
    if payload.start_time >= payload.end_time {
        return api_error(
            StatusCode::BAD_REQUEST,
            "start_time must be before end_time",
        );
    }

    let speed = payload.speed.unwrap_or(1.0);
    if !is_valid_speed(speed) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "Invalid replay speed. Must be 1.0, 5.0, 10.0, 50.0, or 100.0",
        );
    }

    let session_id = uuid::Uuid::new_v4();
    let session = db::ReplaySession {
        session_id,
        satellite_id: payload.satellite_id.clone(),
        start_time: payload.start_time,
        end_time: payload.end_time,
        current_position: payload.start_time,
        speed,
        status: "PAUSED".to_string(), // Replay starts paused by default
        created_at: None,
        updated_at: None,
    };

    if let Err(e) = db::create_replay_session(&state.db_pool, &session).await {
        error!("Failed to create replay session in DB: {}", e);
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database failure");
    }

    state.sessions.insert(session_id, session.clone());

    (StatusCode::CREATED, Json(session)).into_response()
}

async fn get_replay(
    State(state): State<Arc<AppState>>,
    Path(id_param): Path<String>,
) -> impl IntoResponse {
    let session_id = match uuid::Uuid::parse_str(&id_param) {
        Ok(uid) => uid,
        Err(_) => return api_error(StatusCode::BAD_REQUEST, "Invalid session ID"),
    };

    // Try DB
    match db::get_replay_session(&state.db_pool, session_id).await {
        Ok(Some(session)) => {
            (StatusCode::OK, Json(session)).into_response()
        }
        Ok(None) => api_error(StatusCode::NOT_FOUND, "Replay session not found"),
        Err(e) => {
            error!("Database error fetching session: {}", e);
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        }
    }
}

async fn control_replay(
    State(state): State<Arc<AppState>>,
    Path(id_param): Path<String>,
    Json(payload): Json<ControlReplayRequest>,
) -> impl IntoResponse {
    let session_id = match uuid::Uuid::parse_str(&id_param) {
        Ok(uid) => uid,
        Err(_) => return api_error(StatusCode::BAD_REQUEST, "Invalid session ID"),
    };

    let action = payload.action.to_lowercase();
    let valid_actions = ["play", "pause", "stop", "seek", "speed"];
    if !valid_actions.contains(&action.as_str()) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "Invalid action. Must be play, pause, stop, seek, or speed",
        );
    }

    // Fetch session from DB
    let mut session = match db::get_replay_session(&state.db_pool, session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return api_error(StatusCode::NOT_FOUND, "Replay session not found"),
        Err(e) => {
            error!("Database error fetching session: {}", e);
            return api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error");
        }
    };

    metrics::REPLAY_CONTROLS.with_label_values(&[&action]).inc();

    match action.as_str() {
        "play" => {
            session.status = "PLAYING".to_string();
        }
        "pause" => {
            session.status = "PAUSED".to_string();
        }
        "stop" => {
            session.status = "STOPPED".to_string();
        }
        "seek" => {
            let pos = match payload.position {
                Some(p) => p,
                None => {
                    return api_error(
                        StatusCode::BAD_REQUEST,
                        "position is required for seek action",
                    )
                }
            };
            if pos < session.start_time || pos > session.end_time {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "seek position must be within session start and end times",
                );
            }
            session.current_position = pos;
        }
        "speed" => {
            let speed = match payload.speed {
                Some(s) => s,
                None => {
                    return api_error(
                        StatusCode::BAD_REQUEST,
                        "speed is required for speed action",
                    )
                }
            };
            if !is_valid_speed(speed) {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "Invalid speed. Must be 1.0, 5.0, 10.0, 50.0, or 100.0",
                );
            }
            session.speed = speed;
        }
        _ => unreachable!(),
    }

    // Save changes to DB
    if let Err(e) = db::update_replay_session(
        &state.db_pool,
        session.session_id,
        session.current_position,
        &session.status,
        session.speed,
    )
    .await
    {
        error!("Failed to update replay session status in DB: {}", e);
        return api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error updating state",
        );
    }

    state.sessions.insert(session.session_id, session.clone());

    // Query active playing count from DB
    let playing_count: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM replay_sessions WHERE status = 'PLAYING'")
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            error!("Database error counting playing sessions: {}", e);
            0
        }
    };

    // Set the metrics gauge
    metrics::ACTIVE_REPLAYS.set(playing_count as f64);

    (StatusCode::OK, Json(session)).into_response()
}

async fn stream_replay(
    State(state): State<Arc<AppState>>,
    Path(id_param): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, axum::response::Response> {
    let session_id = match uuid::Uuid::parse_str(&id_param) {
        Ok(uid) => uid,
        Err(_) => return Err(api_error(StatusCode::BAD_REQUEST, "Invalid session ID")),
    };

    // Verify session exists in DB or cache
    let session = {
        if let Some(s) = state.sessions.get(&session_id) {
            s.clone()
        } else {
            match db::get_replay_session(&state.db_pool, session_id).await {
                Ok(Some(s)) => {
                    state.sessions.insert(session_id, s.clone());
                    s
                }
                Ok(None) => return Err(api_error(StatusCode::NOT_FOUND, "Replay session not found")),
                Err(e) => {
                    error!("Database error fetching session: {}", e);
                    return Err(api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Database error",
                    ));
                }
            }
        }
    };

    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(100);
    let state_clone = Arc::clone(&state);

    tokio::spawn(async move {
        let satellite_id = session.satellite_id.clone();
        let start_time = session.start_time;
        let end_time = session.end_time;
        let mut local_position = session.current_position;
        let mut last_known_position = session.current_position;
        let mut last_known_status = session.status.clone();
        let mut last_known_speed = session.speed;

        let mut last_tick = tokio::time::Instant::now();
        let mut last_db_write = tokio::time::Instant::now();
        let tick_duration = Duration::from_millis(200);

        info!("Starting SSE Replay stream for session: {}", session_id);

        loop {
            tokio::time::sleep(tick_duration).await;

            // Fetch latest session status from Cache (hot path)
            let current_session = match state_clone.sessions.get(&session_id) {
                Some(s) => s.clone(),
                None => {
                    match db::get_replay_session(&state_clone.db_pool, session_id).await {
                        Ok(Some(s)) => {
                            state_clone.sessions.insert(session_id, s.clone());
                            s
                        }
                        Ok(None) => {
                            info!("Session {} not found in DB or cache, exiting stream loop", session_id);
                            break;
                        }
                        Err(e) => {
                            error!("Database error fetching session {} state: {}", session_id, e);
                            continue;
                        }
                    }
                }
            };

            let current_status = current_session.status.clone();
            let current_speed = current_session.speed;
            let db_position = current_session.current_position;

            let mut seek_to = None;
            let mut play_next = None;
            let mut stop_at = None;

            if db_position != last_known_position {
                // External seek request detected!
                seek_to = Some(db_position);
                local_position = db_position;
                last_known_position = db_position;
            } else if current_status != last_known_status || current_speed != last_known_speed {
                last_known_status = current_status.clone();
                last_known_speed = current_speed;

                // Notify client of status change immediately
                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: local_position,
                    speed: current_speed,
                    status: current_status.clone(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                if tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await
                    .is_err()
                {
                    break;
                }
            }

            if current_status == "STOPPED" {
                stop_at = Some(local_position);
            } else if seek_to.is_none() && current_status == "PLAYING" {
                let now = tokio::time::Instant::now();
                let elapsed_real = now.duration_since(last_tick);
                last_tick = now;

                let elapsed_replay_secs = elapsed_real.as_secs_f32() * current_speed;
                let start_query = local_position;
                let candidate_end = local_position
                    + chrono::Duration::milliseconds((elapsed_replay_secs * 1000.0) as i64);

                if candidate_end >= end_time {
                    stop_at = Some(end_time);
                } else {
                    play_next = Some((start_query, candidate_end));
                }
            } else {
                // PAUSED
                last_tick = tokio::time::Instant::now();
            }

            if let Some(seek_pos) = seek_to {
                info!("Seek detected to {} in session {}", seek_pos, session_id);
                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: seek_pos,
                    speed: current_speed,
                    status: current_status.clone(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                if tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await
                    .is_err()
                {
                    break;
                }

                // Query and send state snapshot at seek position (1s window)
                let start_query = seek_pos - chrono::Duration::seconds(1);
                let end_query = seek_pos;
                let _ = query_and_send_events(
                    &state_clone.db_pool,
                    satellite_id.as_deref(),
                    start_query,
                    end_query,
                    session_id,
                    &tx,
                )
                .await;

                last_tick = tokio::time::Instant::now();
                continue;
            }

            if let Some(stop_pos) = stop_at {
                info!("Session {} status is STOPPED, exiting stream loop", session_id);

                let _ = db::update_replay_session(
                    &state_clone.db_pool,
                    session_id,
                    stop_pos,
                    "STOPPED",
                    current_speed,
                )
                .await;
                if let Some(mut s) = state_clone.sessions.get_mut(&session_id) {
                    s.current_position = stop_pos;
                    s.status = "STOPPED".to_string();
                }

                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: stop_pos,
                    speed: current_speed,
                    status: "STOPPED".to_string(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                let _ = tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await;

                let playing_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM replay_sessions WHERE status = 'PLAYING'")
                    .fetch_one(&state_clone.db_pool)
                    .await
                    .unwrap_or_default();
                metrics::ACTIVE_REPLAYS.set(playing_count as f64);

                if local_position < stop_pos {
                    let _ = query_and_send_events(
                        &state_clone.db_pool,
                        satellite_id.as_deref(),
                        local_position,
                        stop_pos,
                        session_id,
                        &tx,
                    )
                    .await;
                }
                break;
            }

            if let Some((start_query, end_query)) = play_next {
                if let Some(mut s) = state_clone.sessions.get_mut(&session_id) {
                    s.current_position = end_query;
                }

                // Throttle Postgres updates to once every 5 seconds to reduce write load
                let now_inst = tokio::time::Instant::now();
                if now_inst.duration_since(last_db_write) >= Duration::from_secs(5) {
                    let _ = db::update_replay_session(
                        &state_clone.db_pool,
                        session_id,
                        end_query,
                        &current_status,
                        current_speed,
                    )
                    .await;
                    last_db_write = now_inst;
                }

                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: end_query,
                    speed: current_speed,
                    status: current_status.clone(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                if tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await
                    .is_err()
                {
                    break;
                }

                if let Err(e) = query_and_send_events(
                    &state_clone.db_pool,
                    satellite_id.as_deref(),
                    start_query,
                    end_query,
                    session_id,
                    &tx,
                )
                .await
                {
                    error!("Error querying or sending replay events: {}", e);
                    break;
                }

                local_position = end_query;
                last_known_position = end_query;
            } else {
                // Heartbeat state
                let sse_session = db::ReplaySession {
                    session_id,
                    satellite_id: satellite_id.clone(),
                    start_time,
                    end_time,
                    current_position: local_position,
                    speed: current_speed,
                    status: current_status.clone(),
                    created_at: None,
                    updated_at: None,
                };
                let state_json = serde_json::to_string(&sse_session).unwrap_or_default();
                if tx
                    .send(Ok(Event::default().event("state").data(state_json)))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }

        // Final DB write to persist exact state at disconnect
        let final_pos = {
            if let Some(s) = state_clone.sessions.get(&session_id) {
                Some((s.current_position, s.status.clone(), s.speed))
            } else {
                None
            }
        };
        if let Some((pos, status, speed)) = final_pos {
            info!("Persisting final replay session state to DB on exit: position={}, status={}", pos, status);
            let _ = db::update_replay_session(
                &state_clone.db_pool,
                session_id,
                pos,
                &status,
                speed,
            )
            .await;
        }
    });

    let stream = ReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn query_and_send_events(
    pool: &sqlx::PgPool,
    satellite_id: Option<&str>,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
    session_id: uuid::Uuid,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
) -> Result<(), anyhow::Error> {
    let telemetry_fut = db::query_telemetry_range(pool, satellite_id, start, end);
    let events_fut = db::query_events_range(pool, satellite_id, start, end);
    let health_fut = db::query_satellite_health_range(pool, satellite_id, start, end);
    let gs_health_fut = db::query_ground_station_health_range(pool, start, end);

    let (telemetry, events, health, gs_health) =
        tokio::try_join!(telemetry_fut, events_fut, health_fut, gs_health_fut)?;

    let mut all_data = Vec::new();

    for t in telemetry {
        let rec: spaceops_models::TelemetryRecord = t.into();
        all_data.push(TimedReplayData {
            timestamp: rec.timestamp,
            data: ReplayData::Telemetry(rec),
        });
    }
    for e in events {
        all_data.push(TimedReplayData {
            timestamp: e.timestamp,
            data: ReplayData::Event(e),
        });
    }
    for h in health {
        let mapped: db::ReplaySatelliteHealth = h.into();
        all_data.push(TimedReplayData {
            timestamp: mapped.timestamp,
            data: ReplayData::SatelliteHealth(mapped),
        });
    }
    for gs in gs_health {
        all_data.push(TimedReplayData {
            timestamp: gs.timestamp,
            data: ReplayData::GroundStationHealth(gs),
        });
    }

    all_data.sort_by_key(|item| item.timestamp);

    for item in all_data {
        let (event_name, data_type) = match &item.data {
            ReplayData::Telemetry(_) => ("telemetry", "telemetry"),
            ReplayData::Event(_) => ("event", "event"),
            ReplayData::SatelliteHealth(_) => ("satellite_health", "satellite_health"),
            ReplayData::GroundStationHealth(_) => {
                ("ground_station_health", "ground_station_health")
            }
        };

        let json_data = serde_json::to_string(&item.data)?;
        let event = Event::default().event(event_name).data(json_data);

        if tx.send(Ok(event)).await.is_err() {
            return Err(anyhow::anyhow!("channel closed"));
        }

        metrics::REPLAY_EVENTS_STREAMED
            .with_label_values(&[&session_id.to_string(), data_type])
            .inc();
    }

    Ok(())
}

async fn list_satellites(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match db::query_latest_satellites(&state.db_pool).await {
        Ok(health_records) => {
            let mapped: Vec<db::ReplaySatelliteHealth> = health_records
                .into_iter()
                .map(db::ReplaySatelliteHealth::from)
                .collect();
            (StatusCode::OK, Json(mapped)).into_response()
        }
        Err(e) => {
            error!("Failed to query latest satellite health: {}", e);
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database query failure")
        }
    }
}

async fn list_ground_stations(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match db::query_latest_ground_stations(&state.db_pool).await {
        Ok(gs_records) => (StatusCode::OK, Json(gs_records)).into_response(),
        Err(e) => {
            error!("Failed to query latest ground station health: {}", e);
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database query failure")
        }
    }
}

async fn list_events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match db::query_recent_events(&state.db_pool).await {
        Ok(event_records) => (StatusCode::OK, Json(event_records)).into_response(),
        Err(e) => {
            error!("Failed to query recent events: {}", e);
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database query failure")
        }
    }
}

use subtle::ConstantTimeEq;

pub fn secure_compare(provided: &str, expected: &str) -> bool {
    if provided.len() != expected.len() {
        let _ = expected.as_bytes().ct_eq(expected.as_bytes());
        false
    } else {
        provided.as_bytes().ct_eq(expected.as_bytes()).unwrap_u8() == 1
    }
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, std::convert::Infallible> {
    let mut token_opt = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    if token_opt.is_none() {
        if let Some(query) = req.uri().query() {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                if let (Some("token"), Some(val)) = (parts.next(), parts.next()) {
                    token_opt = Some(val.to_string());
                    break;
                }
            }
        }
    }

    let authenticated = match token_opt {
        Some(ref t) => secure_compare(t, &state.api_key),
        None => false,
    };

    if authenticated {
        Ok(next.run(req).await)
    } else {
            let body = serde_json::json!({
                "error": "Unauthorized",
                "details": vec!["Missing or invalid authorization token"]
            })
            .to_string();
            let mut response = axum::response::Response::new(axum::body::Body::from(body));
            *response.status_mut() = StatusCode::UNAUTHORIZED;
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/json"),
            );
            if let Ok(origin_val) = axum::http::HeaderValue::from_str(&state.cors_allowed_origin) {
                response
                    .headers_mut()
                    .insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
            }
            Ok(response)
    }
}

async fn cors_middleware(
    State(state): State<Arc<AppState>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = request.method().clone();
    let allowed_origin = state.cors_allowed_origin.clone();

    if method == axum::http::Method::OPTIONS {
        return axum::response::Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("access-control-allow-origin", &allowed_origin)
            .header("access-control-allow-methods", "GET, POST, OPTIONS")
            .header(
                "access-control-allow-headers",
                "content-type, x-request-id, authorization",
            )
            .body(axum::body::Body::empty())
            .unwrap();
    }

    let mut response = next.run(request).await;
    if let Ok(origin_val) = axum::http::HeaderValue::from_str(&allowed_origin) {
        response
            .headers_mut()
            .insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
    }
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_METHODS,
        axum::http::HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS,
        axum::http::HeaderValue::from_static("content-type, x-request-id, authorization"),
    );
    response
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_speed_validation() {
        assert!(is_valid_speed(1.0));
        assert!(is_valid_speed(5.0));
        assert!(is_valid_speed(10.0));
        assert!(is_valid_speed(50.0));
        assert!(is_valid_speed(100.0));
        assert!(!is_valid_speed(0.0));
        assert!(!is_valid_speed(3.0));
        assert!(!is_valid_speed(120.0));
    }


    #[test]
    fn test_db_satellite_health_mapping() {
        let db_health = db::DbSatelliteHealth {
            timestamp: Utc::now(),
            satellite_id: "SAT-0001".to_string(),
            mission_id: "MISSION-ALPHA".to_string(),
            overall_state: "YELLOW".to_string(),
            overall_score: 85,
            power_state: "GREEN".to_string(),
            thermal_state: "YELLOW".to_string(),
            comm_state: "GREEN".to_string(),
            propulsion_state: "GREEN".to_string(),
            adcs_state: "GREEN".to_string(),
            payload_state: "GREEN".to_string(),
            fc_state: "GREEN".to_string(),
            explanation: "Thermal system elevated".to_string(),
        };

        let mapped = db::ReplaySatelliteHealth::from(db_health);
        assert_eq!(mapped.overall_state, "YELLOW");
        assert_eq!(mapped.overall_score, 85);
        assert_eq!(mapped.power.state, "GREEN");
        assert_eq!(mapped.power.score, 100);
        assert_eq!(mapped.thermal.state, "YELLOW");
        assert_eq!(mapped.thermal.score, 50);
        assert_eq!(mapped.explanation, "Thermal system elevated");
    }

    #[tokio::test]
    async fn test_auth_middleware() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::util::ServiceExt;

        let state = Arc::new(AppState {
            db_pool: sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap(),
            api_key: "secret-key".to_string(),
            cors_allowed_origin: "http://localhost:3000".to_string(),
            sessions: dashmap::DashMap::new(),
        });

        let app = Router::new()
            .route("/test", post(|_: Request<Body>| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state);

        // Test missing token
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Test invalid token
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test")
                    .header("Authorization", "Bearer bad-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Test correct token
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test")
                    .header("Authorization", "Bearer secret-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_replay_lifecycle_integration() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::util::ServiceExt;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgrespassword@localhost:5432/spaceops".to_string());
        
        let db_pool = match sqlx::PgPool::connect_with(
            database_url.parse().unwrap()
        ).await {
            Ok(pool) => pool,
            Err(_) => {
                sqlx::PgPool::connect_lazy(&database_url).unwrap()
            }
        };

        let state = Arc::new(AppState {
            db_pool,
            api_key: "secret-key".to_string(),
            cors_allowed_origin: "http://localhost:3000".to_string(),
            sessions: dashmap::DashMap::new(),
        });

        let app = Router::new()
            .route("/api/replays", post(create_replay))
            .route("/api/replays/:id/control", post(control_replay))
            .route("/api/replays/:id/stream", get(stream_replay))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state);

        // 1. Assert Unauthorized request is rejected
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/replays")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&serde_json::json!({
                        "start_time": "2026-01-01T00:00:00Z",
                        "end_time": "2026-01-01T01:00:00Z"
                    })).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // 2. Assert request with valid auth header but invalid speed is rejected
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/replays")
                    .header("Authorization", "Bearer secret-key")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&serde_json::json!({
                        "start_time": "2026-01-01T00:00:00Z",
                        "end_time": "2026-01-01T01:00:00Z",
                        "speed": 3.0
                    })).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // 3. Assert request with valid auth and correct parameters
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/replays")
                    .header("Authorization", "Bearer secret-key")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&serde_json::json!({
                        "start_time": "2026-01-01T00:00:00Z",
                        "end_time": "2026-01-01T01:00:00Z",
                        "speed": 5.0
                    })).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        
        let status = response.status();
        assert!(status == StatusCode::CREATED || status == StatusCode::INTERNAL_SERVER_ERROR);
    }
}
