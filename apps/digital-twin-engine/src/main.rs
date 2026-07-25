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

//! # Digital Twin Engine
//!
//! Maintains synchronized in-memory virtual states (digital twins) for all
//! active spacecraft in the OrbitLogs constellation. Consumes processed
//! telemetry from Kafka, applies recursive degradation model estimation,
//! and serves health projections and failure simulation via an HTTP API.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐      ┌──────────────────┐
//! │ Kafka: telemetry │─────▶│   consumer.rs    │──┐
//! │   .processed     │      │ (background task)│  │
//! └─────────────────┘      └──────────────────┘  │
//!                                                 ▼
//!                                          ┌────────────┐
//!                                          │  AppState   │
//!                                          │ (DashMaps)  │
//!                                          └────────────┘
//!                                                 ▲
//! ┌─────────────────┐      ┌──────────────────┐  │
//! │   HTTP Clients   │─────▶│   handlers.rs    │──┘
//! │  (Mission Ctrl)  │      │ (Axum routes)    │
//! └─────────────────┘      └──────────────────┘
//! ```

use axum::routing::{delete, get, post};
use axum::Router;
use clap::Parser;
use dashmap::DashMap;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use orbitlogs_models::TelemetryRecord;
use prometheus::{Encoder, TextEncoder};
use rdkafka::producer::FutureProducer;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

mod config;
mod consumer;
mod db;
mod error;
mod handlers;
mod metrics;
mod simulation;

use config::Config;
use simulation::DigitalTwinState;

// =========================================================================
// Application State
// =========================================================================

/// Shared state accessible by all handlers and the background consumer.
///
/// All mutable collections use `DashMap` for lock-free concurrent access
/// from the Kafka consumer task and HTTP handler tasks simultaneously.
pub struct AppState {
    /// Latest telemetry snapshot per satellite (satellite_id → record).
    pub twins: DashMap<String, TelemetryRecord>,

    /// Recursive degradation model state per satellite.
    pub twin_states: DashMap<String, DigitalTwinState>,

    /// Active simulated failure injections (satellite_id → [subsystem]).
    pub simulated_failures: DashMap<String, Vec<String>>,

    /// Database connection pool for timeline event queries.
    pub db_pool: PgPool,

    /// Kafka producer for emitting timeline events.
    pub kafka_producer: FutureProducer,

    /// Target Kafka topic for timeline event notifications.
    pub timeline_topic: String,

    /// Resolved API key for bearer token authentication.
    pub api_key: String,
}

// =========================================================================
// Signal Handling
// =========================================================================

/// Waits for a SIGTERM (Unix) or Ctrl+C shutdown signal.
///
/// Used as the single shutdown signal source. A `CancellationToken` is
/// triggered on receipt, propagating cancellation to all background tasks.
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

// =========================================================================
// Entry Point
// =========================================================================

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let config = Config::parse();

    // ---- Telemetry & Observability ----
    orbitlogs_models::telemetry::init_telemetry("digital-twin-engine", &config.otel_endpoint)?;
    metrics::init_metrics();

    info!("Starting the Digital Twin Engine...");

    // ---- Resolve secrets ----
    let api_key = config.resolve_api_key()?;

    // ---- Database ----
    let db_pool = orbitlogs_models::db::establish_db_connection(&config.database_url, 5).await?;

    // ---- Kafka Producer ----
    let producer_config = KafkaConfigBuilder::from_env(config.kafka_brokers.clone()).build();
    let kafka_producer: FutureProducer = producer_config.create()?;

    // ---- Shared State ----
    let state = Arc::new(AppState {
        twins: DashMap::new(),
        twin_states: DashMap::new(),
        simulated_failures: DashMap::new(),
        db_pool,
        kafka_producer,
        timeline_topic: config.timeline_topic.clone(),
        api_key,
    });

    // ---- Cancellation Token ----
    // Single token drives all background task shutdown.
    let cancel_token = CancellationToken::new();

    // ---- Background: Kafka Consumer ----
    let consumer_state = Arc::clone(&state);
    let consumer_brokers = config.kafka_brokers.clone();
    let consumer_topic = config.processed_topic.clone();
    let consumer_group = config.group_id.clone();
    let consumer_cancel = cancel_token.clone();
    tokio::spawn(async move {
        if let Err(e) = consumer::run_telemetry_consumer(
            consumer_state,
            consumer_brokers,
            consumer_topic,
            consumer_group,
            consumer_cancel,
        )
        .await
        {
            error!("Background digital twin consumer failed: {}", e);
        }
    });

    // ---- Background: Kubernetes Health Probes ----
    let health_port = config.health_port;
    tokio::spawn(async move {
        let health_server = orbitlogs_models::HealthServer::new();
        if let Err(e) = health_server.run(health_port).await {
            error!("Health probe server failed: {}", e);
        }
    });

    // ---- Background: Shutdown Signal Listener ----
    let cancel_trigger = cancel_token.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        info!("Shutdown signal received. Cancelling background digital twin tasks...");
        cancel_trigger.cancel();
    });

    // ---- CORS Configuration ----
    let cors = CorsLayer::new()
        .allow_origin(
            config
                .cors_allowed_origin
                .parse::<axum::http::HeaderValue>()
                .map(tower_http::cors::AllowOrigin::exact)
                .unwrap_or_else(|_| tower_http::cors::AllowOrigin::any()),
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::HeaderName::from_static("x-request-id"),
        ]);

    // ---- Routes ----
    // Authenticated routes (require bearer token)
    let authenticated_routes = Router::new()
        .route(
            "/:satellite/simulate-failure",
            post(handlers::simulate_failure),
        )
        .route(
            "/:satellite/simulated-failures",
            delete(handlers::clear_simulated_failures),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            handlers::auth_middleware,
        ));

    // Metrics endpoint
    let metrics_route = Router::new().route(
        "/metrics",
        get(|| async {
            let encoder = TextEncoder::new();
            let metric_families = prometheus::gather();
            let mut buffer = Vec::new();
            if encoder.encode(&metric_families, &mut buffer).is_err() {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to encode metrics".to_string(),
                );
            }
            (
                axum::http::StatusCode::OK,
                String::from_utf8(buffer).unwrap_or_default(),
            )
        }),
    );

    let app = Router::new()
        .route("/api/twin/:satellite", get(handlers::get_twin))
        .route(
            "/api/twin/:satellite/health",
            get(handlers::get_twin_health),
        )
        .route(
            "/api/twin/:satellite/timeline",
            get(handlers::get_twin_timeline),
        )
        .route(
            "/api/twin/:satellite/simulated-failures",
            get(handlers::list_simulated_failures),
        )
        .nest("/api/twin", authenticated_routes)
        .merge(metrics_route)
        .layer(cors)
        .layer(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(
            30,
        )))
        .with_state(state);

    // ---- Start Server ----
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(
        port = config.port,
        health_port = config.health_port,
        "Digital Twin Engine listening on http://{}",
        addr
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { cancel_token.cancelled().await })
        .await?;

    info!("Digital Twin Engine shutting down gracefully.");
    orbitlogs_models::telemetry::shutdown_telemetry();
    Ok(())
}
