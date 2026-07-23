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

use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use dashmap::DashMap;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use orbitlogs_models::TelemetryRecord;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::FutureProducer;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

mod db;
mod handlers;
mod simulation;

#[derive(Parser, Debug)]
#[command(name = "Digital Twin Engine")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8086")]
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

    #[arg(long, env = "PROCESSED_TOPIC", default_value = "telemetry.processed")]
    processed_topic: String,

    #[arg(long, env = "TIMELINE_EVENTS_TOPIC", default_value = "timeline.events")]
    timeline_topic: String,

    #[arg(long, env = "GROUP_ID", default_value = "digital-twin-group")]
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
    pub twins: DashMap<String, TelemetryRecord>,
    // satellite_id -> list of failed subsystems
    pub simulated_failures: DashMap<String, Vec<String>>, 
    pub db_pool: PgPool,
    pub kafka_producer: FutureProducer,
    pub timeline_topic: String,
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

    // Init tracing
    orbitlogs_models::telemetry::init_telemetry("digital-twin-engine", &cli.otel_endpoint)?;

    info!("Starting the Digital Twin Engine...");

    let api_key = cli.api_key.clone()
        .or_else(|| orbitlogs_models::load_secret("API_KEY"))
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("API key is required. Set env var API_KEY, pass --api-key, or use Docker secrets API_KEY."))?;

    let db_pool = orbitlogs_models::db::establish_db_connection(&cli.database_url, 5).await?;

    let producer_config = KafkaConfigBuilder::from_env(cli.kafka_brokers.clone()).build();

    let kafka_producer: FutureProducer = producer_config.create()?;

    let state = Arc::new(AppState {
        twins: DashMap::new(),
        simulated_failures: DashMap::new(),
        db_pool,
        kafka_producer,
        timeline_topic: cli.timeline_topic,
        api_key,
        cors_allowed_origin: cli.cors_allowed_origin.clone(),
    });

    let cancel_token = CancellationToken::new();
    let cancel_token_trigger = cancel_token.clone();

    // Start background Kafka consumer
    let state_clone = Arc::clone(&state);
    let brokers = cli.kafka_brokers.clone();
    let processed_topic = cli.processed_topic.clone();
    let group_id = cli.group_id.clone();
    let cancel_token_clone = cancel_token.clone();
    tokio::spawn(async move {
        if let Err(e) = run_telemetry_consumer(
            state_clone,
            brokers,
            processed_topic,
            group_id,
            cancel_token_clone,
        )
        .await
        {
            error!("Background digital twin consumer failed: {}", e);
        }
    });

    tokio::spawn(async move {
        shutdown_signal().await;
        info!("Shutdown signal received. Cancelling background digital twin tasks...");
        cancel_token_trigger.cancel();
    });

    let state_routes = Router::new().route(
        "/:satellite/simulate-failure",
        post(handlers::simulate_failure).layer(axum::middleware::from_fn_with_state(
            state.clone(),
            handlers::auth_middleware,
        )),
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
        .nest("/api/twin", state_routes)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            handlers::cors_middleware,
        ))
        .layer(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(
            30,
        )))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 
        cli.port
    ));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(
        "The Digital Twin Engine is listening on http://localhost:{}",
        cli.port
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("The Digital Twin Engine is shutting down gracefully.");
    orbitlogs_models::telemetry::shutdown_telemetry();
    Ok(())
}

async fn run_telemetry_consumer(
    state: Arc<AppState>,
    brokers: String,
    topic: String,
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
    consumer.subscribe(&[&topic])?;

    info!("The Digital twin background consumer subscribed to {}", topic);

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("The Digital twin background consumer stopping...");
                break;
            }
            msg_res = consumer.recv() => {
                match msg_res {
                    Ok(msg) => {
                        if let Some(Ok(payload)) = msg.payload_view::<str>() {
                            if let Ok(mut rec) = serde_json::from_str::<TelemetryRecord>(payload) {
                                // Apply active simulated failures
                                if let Some(failures) = state.simulated_failures.get(rec.satellite_id.as_str()) {
                                    for sub in failures.value() {
                                        match sub.as_str() {
                                            "power" => {
                                                rec.battery_voltage = Some(rec.battery_voltage.unwrap_or(12.0) * 0.75);
                                                rec.solar_panel_output = Some(0.0);
                                            }
                                            "thermal" => {
                                                rec.temperature = Some(rec.temperature.unwrap_or(25.0) + 65.0);
                                            }
                                            "adcs" => {
                                                rec.orientation_pitch = Some(rec.orientation_pitch.unwrap_or(0.0) + 15.0);
                                            }
                                            "communications" => {
                                                rec.packet_loss = Some(95.0);
                                                rec.communication_latency = Some(5000.0);
                                            }
                                            "propulsion" => {
                                                rec.fuel_remaining = Some(rec.fuel_remaining.unwrap_or(10.0) * 0.1);
                                            }
                                            _ => {}
                                        }
                                    }
                                }

                                // Store twin state
                                state.twins.insert(rec.satellite_id.to_string(), rec);
                            }
                        }
                        let _ = consumer.commit_message(&msg, CommitMode::Async);
                    }
                    Err(e) => {
                        error!("Error consuming telemetry for the digital twin: {}", e);
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
    Ok(())
}
