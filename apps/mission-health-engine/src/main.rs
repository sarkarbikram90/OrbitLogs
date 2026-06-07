#![allow(clippy::needless_borrows_for_generic_args)]

use clap::Parser;
use prometheus::{Encoder, TextEncoder};
use rdkafka::client::ClientContext;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use spaceops_models::db::establish_db_connection;
use spaceops_models::kafka::KafkaConfigBuilder;
use sqlx::PgPool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

mod db;
mod health;
mod metrics;

use health::{assess_satellite_health, GroundStationState};
use spaceops_models::TelemetryRecord;

struct MetricsContext;

impl ClientContext for MetricsContext {
    fn stats_raw(&self, statistics: &[u8]) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(statistics) {
            if let Some(topics) = v.get("topics").and_then(|t| t.as_object()) {
                for (topic_name, topic_val) in topics {
                    if let Some(partitions) =
                        topic_val.get("partitions").and_then(|p| p.as_object())
                    {
                        for (partition_id, partition_val) in partitions {
                            if let Some(lag) =
                                partition_val.get("consumer_lag").and_then(|l| l.as_i64())
                            {
                                metrics::KAFKA_CONSUMER_LAG
                                    .with_label_values(&[topic_name, partition_id])
                                    .set(lag as f64);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl ConsumerContext for MetricsContext {}

#[derive(Parser, Debug, Clone)]
#[command(name = "Open SpaceOps - Mission Health Engine")]
#[command(about = "Processes satellite telemetry and publishes spacecraft health status", long_about = None)]
struct Cli {
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://postgres:postgrespassword@localhost:5432/spaceops"
    )]
    database_url: String,

    #[arg(long, env = "PROCESSED_TOPIC", default_value = "telemetry.processed")]
    processed_topic: String,

    #[arg(long, env = "HEALTH_TOPIC", default_value = "mission.health")]
    health_topic: String,

    #[arg(long, env = "GS_HEALTH_TOPIC", default_value = "groundstation.health")]
    gs_health_topic: String,

    #[arg(long, env = "GROUP_ID", default_value = "mission-health-group")]
    group_id: String,

    #[arg(long, env = "METRICS_PORT", default_value = "8082")]
    metrics_port: u16,

    #[arg(long, env = "KAFKA_SECURITY_PROTOCOL")]
    kafka_security_protocol: Option<String>,

    #[arg(long, env = "KAFKA_SASL_MECHANISM")]
    kafka_sasl_mechanism: Option<String>,

    #[arg(long, env = "KAFKA_SASL_USERNAME")]
    kafka_sasl_username: Option<String>,

    #[arg(long, env = "KAFKA_SASL_PASSWORD")]
    kafka_sasl_password: Option<String>,

    #[arg(long, env = "KAFKA_SSL_CA_LOCATION")]
    kafka_ssl_ca_location: Option<String>,
}

struct EngineState {
    db_pool: PgPool,
    kafka_producer: FutureProducer,
    health_topic: String,
    satellite_scores: Mutex<HashMap<String, i32>>,
    gs_states: Mutex<HashMap<String, GroundStationState>>,
}

async fn start_metrics_server(port: u16) {
    use axum::{routing::get, Router};

    let app = Router::new().route(
        "/metrics",
        get(|| async {
            let encoder = TextEncoder::new();
            let metric_families = metrics::REGISTRY.gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer).unwrap();

            axum::response::Response::builder()
                .status(axum::http::StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, encoder.format_type())
                .body(axum::body::Body::from(buffer))
                .unwrap()
        }),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind metrics server to port {}: {}", port, e);
            return;
        }
    };
    info!(
        "Mission Health Engine metrics server running on http://localhost:{}/metrics",
        port
    );
    if let Err(e) = axum::serve(listener, app).await {
        error!("Metrics server error: {}", e);
    }
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
    if let Some(val) = load_secret("KAFKA_SASL_PASSWORD") {
        cli.kafka_sasl_password = Some(val);
    }

    info!(
        "Starting Mission Health Engine with configuration: {:?}",
        cli
    );

    // Register Prometheus metrics
    metrics::register_metrics();

    // Start Prometheus TCP metrics server
    tokio::spawn(start_metrics_server(cli.metrics_port));

    // Connect to database
    let db_pool = establish_db_connection(&cli.database_url, 10).await?;

    // Init Kafka Producer
    let producer_config = KafkaConfigBuilder::new(cli.kafka_brokers.clone())
        .with_security(
            cli.kafka_security_protocol.clone(),
            cli.kafka_sasl_mechanism.clone(),
            cli.kafka_sasl_username.clone(),
            cli.kafka_sasl_password.clone(),
            cli.kafka_ssl_ca_location.clone(),
        )
        .build();

    let producer: FutureProducer = producer_config.create()?;
    info!("Kafka producer successfully initialized.");

    // Init Kafka Consumer
    let mut consumer_config = KafkaConfigBuilder::new(cli.kafka_brokers.clone())
        .with_security(
            cli.kafka_security_protocol.clone(),
            cli.kafka_sasl_mechanism.clone(),
            cli.kafka_sasl_username.clone(),
            cli.kafka_sasl_password.clone(),
            cli.kafka_ssl_ca_location.clone(),
        )
        .build();
    consumer_config
        .set("group.id", &cli.group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .set("statistics.interval.ms", "1000");

    let consumer: StreamConsumer<MetricsContext> =
        consumer_config.create_with_context(MetricsContext)?;
    consumer.subscribe(&[&cli.processed_topic])?;
    info!(
        "Kafka consumer subscribed to topic '{}'",
        cli.processed_topic
    );

    let state = Arc::new(EngineState {
        db_pool,
        kafka_producer: producer,
        health_topic: cli.health_topic,
        satellite_scores: Mutex::new(HashMap::new()),
        gs_states: Mutex::new(HashMap::new()),
    });

    let cancel_token = CancellationToken::new();
    let cancel_token_trigger = cancel_token.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutdown signal received. Cancelling consumer tasks...");
        cancel_token_trigger.cancel();
    });

    // Spawn background consumer for ground station health
    let gs_states_clone = Arc::clone(&state);
    let cancel_token_gs = cancel_token.clone();
    let brokers = cli.kafka_brokers.clone();
    let gs_topic = cli.gs_health_topic.clone();
    let security_protocol = cli.kafka_security_protocol.clone();
    let sasl_mechanism = cli.kafka_sasl_mechanism.clone();
    let sasl_username = cli.kafka_sasl_username.clone();
    let sasl_password = cli.kafka_sasl_password.clone();
    let ssl_ca_location = cli.kafka_ssl_ca_location.clone();

    tokio::spawn(async move {
        info!("Starting ground station health consumer for mission-health-engine...");
        let mut bg_config = KafkaConfigBuilder::new(brokers)
            .with_security(
                security_protocol,
                sasl_mechanism,
                sasl_username,
                sasl_password,
                ssl_ca_location,
            )
            .build();
        bg_config
            .set(
                "group.id",
                &format!("mission-health-engine-gs-group-{}", uuid::Uuid::new_v4()),
            )
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "latest")
            .set("statistics.interval.ms", "1000");

        let consumer_res: Result<StreamConsumer<MetricsContext>, _> =
            bg_config.create_with_context(MetricsContext);

        let consumer = match consumer_res {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create ground station health consumer: {}", e);
                return;
            }
        };

        if let Err(e) = consumer.subscribe(&[&gs_topic]) {
            error!(
                "Failed to subscribe to ground station health topic {}: {}",
                gs_topic, e
            );
            return;
        }

        loop {
            tokio::select! {
                _ = cancel_token_gs.cancelled() => {
                    info!("Ground station health consumer shutting down...");
                    break;
                }
                msg_res = consumer.recv() => {
                    match msg_res {
                        Ok(msg) => {
                            let payload = match msg.payload_view::<str>() {
                                Some(Ok(s)) => s,
                                Some(Err(e)) => {
                                    error!("Failed to parse ground station payload: {}", e);
                                    continue;
                                }
                                None => continue,
                            };

                            match serde_json::from_str::<GroundStationState>(payload) {
                                Ok(gs) => {
                                    info!(
                                        "Engine received ground station update: {} -> {}",
                                        gs.station_id, gs.status
                                    );
                                    let mut map = gs_states_clone.gs_states.lock().await;
                                    map.insert(gs.station_id.clone(), gs);
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to deserialize GroundStationState: {}. Payload: {}",
                                        e, payload
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error in ground station consumer: {}", e);
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    }
                }
            }
        }
    });

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Main consumer loop shutting down...");
                break;
            }
            msg_res = consumer.recv() => {
                match msg_res {
                    Ok(msg) => {
                        let payload = match msg.payload_view::<str>() {
                            Some(Ok(s)) => s,
                            Some(Err(e)) => {
                                error!("Failed to parse Kafka payload as string: {}", e);
                                let _ = consumer.commit_message(&msg, CommitMode::Async);
                                continue;
                            }
                            None => {
                                let _ = consumer.commit_message(&msg, CommitMode::Async);
                                continue;
                            }
                        };

                        let record: TelemetryRecord = match serde_json::from_str(payload) {
                            Ok(r) => r,
                            Err(e) => {
                                error!(
                                    "Failed to deserialize telemetry record: {}. Payload: {}",
                                    e, payload
                                );
                                let _ = consumer.commit_message(&msg, CommitMode::Async);
                                continue;
                            }
                        };

                        if let Err(e) = process_telemetry_health(&state, &record).await {
                            error!(
                                "Error processing health for satellite {}: {}",
                                record.satellite_id, e
                            );
                        }

                        if let Err(e) = consumer.commit_message(&msg, CommitMode::Async) {
                            error!("Failed to commit consumer offset: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Error receiving from Kafka: {}", e);
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }

    info!("Mission Health Engine shutdown complete.");
    Ok(())
}

async fn process_telemetry_health(
    state: &EngineState,
    record: &TelemetryRecord,
) -> Result<(), anyhow::Error> {
    // 1. Run health assessment rules
    let gs_states_map = {
        let map = state.gs_states.lock().await;
        map.clone()
    };
    let assessment = assess_satellite_health(record, &gs_states_map);
    info!(
        "Satellite {} overall health: {:?} (Score: {})",
        assessment.satellite_id, assessment.overall_state, assessment.overall_score
    );

    // 2. Persist assessment to TimescaleDB
    db::insert_satellite_health(&state.db_pool, &assessment).await?;

    // 3. Publish assessment to Kafka mission.health topic
    let payload = serde_json::to_string(&assessment)?;
    let key = format!("{}:{}", assessment.mission_id, assessment.satellite_id);
    state
        .kafka_producer
        .send(
            FutureRecord::to(&state.health_topic)
                .key(&key)
                .payload(&payload),
            Duration::from_secs(5),
        )
        .await
        .map_err(|(e, _)| anyhow::anyhow!("Kafka send failed: {}", e))?;

    // 4. Update Prometheus metrics
    metrics::SATELLITE_HEALTH_SCORE
        .with_label_values(&[&assessment.satellite_id])
        .set(assessment.overall_score as f64);

    metrics::SUBSYSTEM_HEALTH_SCORE
        .with_label_values(&[&assessment.satellite_id, "power"])
        .set(assessment.power.score as f64);
    metrics::SUBSYSTEM_HEALTH_SCORE
        .with_label_values(&[&assessment.satellite_id, "thermal"])
        .set(assessment.thermal.score as f64);
    metrics::SUBSYSTEM_HEALTH_SCORE
        .with_label_values(&[&assessment.satellite_id, "communications"])
        .set(assessment.communications.score as f64);
    metrics::SUBSYSTEM_HEALTH_SCORE
        .with_label_values(&[&assessment.satellite_id, "propulsion"])
        .set(assessment.propulsion.score as f64);
    metrics::SUBSYSTEM_HEALTH_SCORE
        .with_label_values(&[&assessment.satellite_id, "adcs"])
        .set(assessment.adcs.score as f64);
    metrics::SUBSYSTEM_HEALTH_SCORE
        .with_label_values(&[&assessment.satellite_id, "payload"])
        .set(assessment.payload.score as f64);
    metrics::SUBSYSTEM_HEALTH_SCORE
        .with_label_values(&[&assessment.satellite_id, "flight_computer"])
        .set(assessment.flight_computer.score as f64);

    // Update fleet health average score
    let avg_fleet = {
        let mut scores = state.satellite_scores.lock().await;
        scores.insert(assessment.satellite_id.clone(), assessment.overall_score);
        let total: i32 = scores.values().sum();
        total as f64 / scores.len() as f64
    };
    metrics::FLEET_HEALTH_SCORE.set(avg_fleet);

    Ok(())
}
