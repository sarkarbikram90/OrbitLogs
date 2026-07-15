// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
mod db;
mod engine;
mod handlers;
mod metrics;
mod session;

use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::{error, info};

#[derive(Parser, Debug, Clone)]
#[command(name = "OrbitLogs - Mission Replay Engine")]
#[command(about = "Historical data playback engine for telemetry and events", long_about = None)]
struct Cli {
    #[arg(long, env = "DATABASE_URL", default_value = "")]
    database_url: String,

    #[arg(long, env = "PORT", default_value = "8084")]
    port: u16,

    #[arg(long, env = "API_KEY")]
    api_key: Option<String>,

    #[arg(
        long,
        env = "CORS_ALLOWED_ORIGIN",
        default_value = "http://localhost:3000"
    )]
    cors_allowed_origin: String,
}

pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub api_key: String,
    pub cors_allowed_origin: String,
    pub sessions: dashmap::DashMap<uuid::Uuid, db::ReplaySession>,
}

use orbitlogs_models::load_secret;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load local .env
    let _ = dotenvy::dotenv();

    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let mut cli = Cli::parse();
    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) = load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty()) {
        val
    } else {
        return Err(anyhow::anyhow!("DATABASE_URL is required. Set env var DATABASE_URL, pass --database-url, or use Docker secrets DATABASE_URL."));
    };
    cli.database_url = db_url;
    let api_key = cli.api_key.clone()
        .or_else(|| load_secret("API_KEY"))
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("API key is required. Set env var API_KEY, pass --api-key, or use Docker secrets API_KEY."))?;

    info!(
        "Starting Mission Replay Engine with configuration: {:?}",
        cli
    );

    // Register Prometheus metrics
    metrics::register_metrics();

    // Connect to database
    let db_pool = orbitlogs_models::db::establish_db_connection(&cli.database_url, 10).await?;

    let state = Arc::new(AppState {
        db_pool,
        api_key,
        cors_allowed_origin: cli.cors_allowed_origin,
        sessions: dashmap::DashMap::new(),
    });

    // Spawn background cleanup task for old replay sessions
    let state_for_cleanup = state.clone();
    tokio::spawn(async move {
        info!("Starting background replay session cleanup task...");
        loop {
            tokio::time::sleep(Duration::from_secs(300)).await; // 5 minutes
            info!("Running cleanup of old replay sessions...");

            match sqlx::query(
                "DELETE FROM replay_sessions WHERE created_at < NOW() - INTERVAL '1 hour'",
            )
            .execute(&state_for_cleanup.db_pool)
            .await
            {
                Ok(result) => {
                    info!(
                        "Successfully cleaned up {} old replay sessions from DB.",
                        result.rows_affected()
                    );
                }
                Err(e) => {
                    error!("Failed to clean up old replay sessions from DB: {}", e);
                }
            }

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
        .route("/", post(handlers::create_replay))
        .route("/:id", get(handlers::get_replay))
        .route("/:id/control", post(handlers::control_replay))
        .route("/:id/stream", get(handlers::stream_replay))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            handlers::auth_middleware,
        ));

    let app = Router::new()
        .nest("/api/replays", replay_routes)
        .route("/api/satellites", get(handlers::list_satellites))
        .route("/api/ground-stations", get(handlers::list_ground_stations))
        .route("/api/events", get(handlers::list_events))
        .route("/metrics", get(handlers::handle_metrics))
        .route("/health", get(handlers::health_check))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            handlers::cors_middleware,
        ))
        .layer(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(
            30,
        )))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = TcpListener::bind(addr).await?;
    info!(
        "Mission Replay Engine listening on http://localhost:{}",
        cli.port
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(handlers::shutdown_signal())
        .await?;

    info!("Mission Replay Engine shut down gracefully.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use session::is_valid_speed;

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
            timestamp: chrono::Utc::now(),
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
    }

    #[tokio::test]
    async fn test_auth_middleware() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::util::ServiceExt;

        let state = Arc::new(AppState {
            db_pool: sqlx::Pool::connect_lazy("postgres://localhost/dummy").unwrap(),
            api_key: "secret-key".to_string(),
            cors_allowed_origin: "http://localhost:3000".to_string(),
            sessions: dashmap::DashMap::new(),
        });

        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                handlers::auth_middleware,
            ))
            .with_state(state);

        // Unauthorized request
        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);

        // Authorized request with header
        let req = Request::builder()
            .uri("/test")
            .header("Authorization", "Bearer secret-key")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        // Authorized request with query param
        let req = Request::builder()
            .uri("/test?token=secret-key")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_replay_lifecycle_integration() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::util::ServiceExt;

        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgrespassword@localhost:5432/orbitlogs".to_string()
        });

        let db_pool = match sqlx::PgPool::connect_with(database_url.parse().unwrap()).await {
            Ok(pool) => pool,
            Err(_) => sqlx::PgPool::connect_lazy(&database_url).unwrap(),
        };

        let state = Arc::new(AppState {
            db_pool,
            api_key: "secret-key".to_string(),
            cors_allowed_origin: "http://localhost:3000".to_string(),
            sessions: dashmap::DashMap::new(),
        });

        let app = Router::new()
            .route("/api/replays", post(handlers::create_replay))
            .route("/api/replays/:id", get(handlers::get_replay))
            .route("/api/replays/:id/control", post(handlers::control_replay))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                handlers::auth_middleware,
            ))
            .with_state(state);

        let now = chrono::Utc::now();
        let create_body = serde_json::json!({
            "satellite_id": "SAT-0001",
            "start_time": now - chrono::Duration::hours(1),
            "end_time": now,
            "speed": 5.0
        });

        let req = Request::builder()
            .method("POST")
            .uri("/api/replays")
            .header("Authorization", "Bearer secret-key")
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&create_body).unwrap()))
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();

        if resp.status() == axum::http::StatusCode::CREATED {
            let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap();
            let session: db::ReplaySession = serde_json::from_slice(&body_bytes).unwrap();
            let session_id = session.session_id;
            assert_eq!(session.speed, 5.0);
            assert_eq!(session.status, "PAUSED");

            // Control session: play
            let control_body = serde_json::json!({
                "action": "play"
            });
            let ctrl_req = Request::builder()
                .method("POST")
                .uri(format!("/api/replays/{}/control", session_id))
                .header("Authorization", "Bearer secret-key")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&control_body).unwrap()))
                .unwrap();

            let ctrl_resp = app.clone().oneshot(ctrl_req).await.unwrap();
            assert_eq!(ctrl_resp.status(), axum::http::StatusCode::OK);
        }
    }
}
