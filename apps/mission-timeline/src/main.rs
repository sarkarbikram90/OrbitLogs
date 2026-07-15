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
use axum::{routing::get, Router};
use clap::Parser;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use uuid::Uuid;

mod handlers;

#[derive(Parser, Debug)]
#[command(name = "Mission Timeline Engine")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8087")]
    port: u16,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(long, env = "DATABASE_URL", default_value = "")]
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
    pub db_pool: PgPool,
    pub api_key: String,
    pub cors_allowed_origin: String,
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

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    // Start tracing
    orbitlogs_models::telemetry::init_telemetry("mission-timeline", &cli.otel_endpoint)?;

    info!("Starting Mission Timeline Engine...");

    let api_key = cli.api_key.clone()
        .or_else(|| orbitlogs_models::load_secret("API_KEY"))
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("API key is required. Set env var API_KEY, pass --api-key, or use Docker secrets API_KEY."))?;

    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) =
        orbitlogs_models::load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty())
    {
        val
    } else {
        return Err(anyhow::anyhow!("DATABASE_URL is required. Set env var DATABASE_URL, pass --database-url, or use Docker secrets DATABASE_URL."));
    };

    let db_pool = orbitlogs_models::db::establish_db_connection(&db_url, 5).await?;

    let state = Arc::new(AppState {
        db_pool,
        api_key,
        cors_allowed_origin: cli.cors_allowed_origin.clone(),
    });

    let cancel_token = CancellationToken::new();
    let cancel_token_trigger = cancel_token.clone();

    // Spawn background consumer for events
    let state_clone = Arc::clone(&state);
    let brokers = cli.kafka_brokers.clone();
    let timeline_topic = cli.timeline_topic.clone();
    let health_topic = cli.health_topic.clone();
    let group_id = cli.group_id.clone();
    let cancel_token_clone = cancel_token.clone();
    tokio::spawn(async move {
        if let Err(e) = run_events_consumer(
            state_clone,
            brokers,
            timeline_topic,
            health_topic,
            group_id,
            cancel_token_clone,
        )
        .await
        {
            error!("Background events consumer failed: {}", e);
        }
    });

    tokio::spawn(async move {
        shutdown_signal().await;
        info!("Shutdown signal received. Cancelling background timeline tasks...");
        cancel_token_trigger.cancel();
    });

    let app = Router::new()
        .route(
            "/api/timeline",
            get(handlers::get_timeline)
                .post(handlers::create_timeline_event)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    handlers::auth_middleware,
                )),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            handlers::cors_middleware,
        ))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(
        "Mission Timeline Engine listening on http://localhost:{}",
        cli.port
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Mission Timeline Engine shut down gracefully.");
    orbitlogs_models::telemetry::shutdown_telemetry();
    Ok(())
}

async fn run_events_consumer(
    state: Arc<AppState>,
    brokers: String,
    timeline_topic: String,
    health_topic: String,
    group_id: String,
    cancel_token: CancellationToken,
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

    info!(
        "Timeline consumer subscribed to topics: {}, {}",
        timeline_topic, health_topic
    );

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Timeline consumer stopping...");
                break Ok(());
            }
            msg_res = consumer.recv() => {
                match msg_res {
                    Ok(msg) => {
                        let topic = msg.topic();
                        if let Some(Ok(payload)) = msg.payload_view::<str>() {
                            if topic == timeline_topic {
                                if let Ok(evt) = serde_json::from_str::<handlers::TimelineEvent>(payload) {
                                    let _ = handlers::insert_event(&state.db_pool, &evt).await;
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

                                    let evt = handlers::TimelineEvent {
                                        timestamp: chrono::Utc::now(),
                                        event_id: Uuid::new_v4(),
                                        source: "TELEMETRY".to_string(),
                                        satellite_id: sat_id,
                                        event_type: "HEALTH_ALERT".to_string(),
                                        severity: severity.to_string(),
                                        message: format!("Satellite health state changed to {} (Score: {}) - {}", ostate, oscore, explanation),
                                        details: Some(val),
                                    };

                                    let _ = handlers::insert_event(&state.db_pool, &evt).await;
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
    }
}
