#![allow(clippy::needless_borrows_for_generic_args)]

use clap::Parser;
use prometheus::{Encoder, TextEncoder};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::{error, info};

mod db;
mod health;
mod metrics;

use health::{assess_satellite_health, GroundStationState};
use spaceops_models::TelemetryRecord;

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
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match TcpListener::bind(addr).await {
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

    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buffer = [0; 1024];
                    let _ = socket.read(&mut buffer).await;

                    let encoder = TextEncoder::new();
                    let metric_families = metrics::REGISTRY.gather();
                    let mut buffer = Vec::new();
                    encoder.encode(&metric_families, &mut buffer).unwrap();

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        encoder.format_type(),
                        buffer.len()
                    );

                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.write_all(&buffer).await;
                    let _ = socket.flush().await;
                });
            }
            Err(e) => {
                error!("Error accepting connection in metrics server: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load local .env
    let _ = dotenvy::dotenv();

    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    info!("Starting Mission Health Engine with configuration: {:?}", cli);

    // Register Prometheus metrics
    metrics::register_metrics();

    // Start Prometheus TCP metrics server
    tokio::spawn(start_metrics_server(cli.metrics_port));

    // Connect to database
    info!("Connecting to TimescaleDB...");
    let mut retries = 5;
    let mut db_pool = None;
    while retries > 0 {
        match PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&cli.database_url)
            .await
        {
            Ok(pool) => {
                info!("TimescaleDB connection pool established.");
                db_pool = Some(pool);
                break;
            }
            Err(e) => {
                retries -= 1;
                error!("TimescaleDB connection failed. Retries left: {}. Error: {}", retries, e);
                if retries > 0 {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }
    }
    let db_pool = db_pool.ok_or_else(|| anyhow::anyhow!("Failed to connect to database"))?;

    // Init Kafka Producer
    let mut producer_config = ClientConfig::new();
    producer_config
        .set("bootstrap.servers", &cli.kafka_brokers)
        .set("message.timeout.ms", "5000");

    if let Some(ref proto) = cli.kafka_security_protocol {
        producer_config.set("security.protocol", proto);
    }
    if let Some(ref mech) = cli.kafka_sasl_mechanism {
        producer_config.set("sasl.mechanism", mech);
    }
    if let Some(ref user) = cli.kafka_sasl_username {
        producer_config.set("sasl.username", user);
    }
    if let Some(ref pass) = cli.kafka_sasl_password {
        producer_config.set("sasl.password", pass);
    }
    if let Some(ref ca) = cli.kafka_ssl_ca_location {
        producer_config.set("ssl.ca.location", ca);
    }

    let producer: FutureProducer = producer_config.create()?;
    info!("Kafka producer successfully initialized.");

    // Init Kafka Consumer
    let mut consumer_config = ClientConfig::new();
    consumer_config
        .set("bootstrap.servers", &cli.kafka_brokers)
        .set("group.id", &cli.group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest");

    if let Some(ref proto) = cli.kafka_security_protocol {
        consumer_config.set("security.protocol", proto);
    }
    if let Some(ref mech) = cli.kafka_sasl_mechanism {
        consumer_config.set("sasl.mechanism", mech);
    }
    if let Some(ref user) = cli.kafka_sasl_username {
        consumer_config.set("sasl.username", user);
    }
    if let Some(ref pass) = cli.kafka_sasl_password {
        consumer_config.set("sasl.password", pass);
    }
    if let Some(ref ca) = cli.kafka_ssl_ca_location {
        consumer_config.set("ssl.ca.location", ca);
    }

    let consumer: StreamConsumer = consumer_config.create()?;
    consumer.subscribe(&[&cli.processed_topic])?;
    info!("Kafka consumer subscribed to topic '{}'", cli.processed_topic);

    let state = Arc::new(EngineState {
        db_pool,
        kafka_producer: producer,
        health_topic: cli.health_topic,
        satellite_scores: Mutex::new(HashMap::new()),
        gs_states: Mutex::new(HashMap::new()),
    });

    // Spawn background consumer for ground station health
    let gs_states_clone = Arc::clone(&state);
    let brokers = cli.kafka_brokers.clone();
    let gs_topic = cli.gs_health_topic.clone();
    let security_protocol = cli.kafka_security_protocol.clone();
    let sasl_mechanism = cli.kafka_sasl_mechanism.clone();
    let sasl_username = cli.kafka_sasl_username.clone();
    let sasl_password = cli.kafka_sasl_password.clone();
    let ssl_ca_location = cli.kafka_ssl_ca_location.clone();

    tokio::spawn(async move {
        info!("Starting ground station health consumer for mission-health-engine...");
        let mut bg_config = ClientConfig::new();
        bg_config
            .set("bootstrap.servers", &brokers)
            .set("group.id", &format!("mission-health-engine-gs-group-{}", uuid::Uuid::new_v4()))
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "latest");

        if let Some(ref proto) = security_protocol {
            bg_config.set("security.protocol", proto);
        }
        if let Some(ref mech) = sasl_mechanism {
            bg_config.set("sasl.mechanism", mech);
        }
        if let Some(ref user) = sasl_username {
            bg_config.set("sasl.username", user);
        }
        if let Some(ref pass) = sasl_password {
            bg_config.set("sasl.password", pass);
        }
        if let Some(ref ca) = ssl_ca_location {
            bg_config.set("ssl.ca.location", ca);
        }

        let consumer_res: Result<StreamConsumer, _> = bg_config.create();

        let consumer = match consumer_res {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create ground station health consumer: {}", e);
                return;
            }
        };

        if let Err(e) = consumer.subscribe(&[&gs_topic]) {
            error!("Failed to subscribe to ground station health topic {}: {}", gs_topic, e);
            return;
        }

        loop {
            match consumer.recv().await {
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
                            info!("Engine received ground station update: {} -> {}", gs.station_id, gs.status);
                            let mut map = gs_states_clone.gs_states.lock().unwrap();
                            map.insert(gs.station_id.clone(), gs);
                        }
                        Err(e) => {
                            error!("Failed to deserialize GroundStationState: {}. Payload: {}", e, payload);
                        }
                    }
                }
                Err(e) => {
                    error!("Error in ground station consumer: {}", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    loop {
        match consumer.recv().await {
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
                        error!("Failed to deserialize telemetry record: {}. Payload: {}", e, payload);
                        let _ = consumer.commit_message(&msg, CommitMode::Async);
                        continue;
                    }
                };

                let state_ref = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = process_telemetry_health(&state_ref, &record).await {
                        error!("Error processing health for satellite {}: {}", record.satellite_id, e);
                    }
                });

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

async fn process_telemetry_health(state: &EngineState, record: &TelemetryRecord) -> Result<(), anyhow::Error> {
    // 1. Run health assessment rules
    let gs_states_map = {
        let map = state.gs_states.lock().unwrap();
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
    state.kafka_producer
        .send(
            FutureRecord::to(&state.health_topic)
                .key(&assessment.satellite_id)
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
        let mut scores = state.satellite_scores.lock().unwrap();
        scores.insert(assessment.satellite_id.clone(), assessment.overall_score);
        let total: i32 = scores.values().sum();
        total as f64 / scores.len() as f64
    };
    metrics::FLEET_HEALTH_SCORE.set(avg_fleet);

    Ok(())
}
