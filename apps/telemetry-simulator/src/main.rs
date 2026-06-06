#![allow(clippy::needless_borrows_for_generic_args)]

use clap::Parser;
use prometheus::{Encoder, IntCounter, Registry, TextEncoder};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, error, info};

mod config;
mod generator;
mod scenario;

use config::Config;
use generator::{GroundStationState, SatelliteSimulator};
use lazy_static::lazy_static;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::client::ClientContext;
use rdkafka::consumer::ConsumerContext;

struct MetricsContext;

impl ClientContext for MetricsContext {
    fn stats_raw(&self, statistics: &[u8]) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(statistics) {
            if let Some(topics) = v.get("topics").and_then(|t| t.as_object()) {
                for (topic_name, topic_val) in topics {
                    if let Some(partitions) = topic_val.get("partitions").and_then(|p| p.as_object()) {
                        for (partition_id, partition_val) in partitions {
                            if let Some(lag) = partition_val.get("consumer_lag").and_then(|l| l.as_i64()) {
                                KAFKA_CONSUMER_LAG
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

fn get_linked_ground_station(satellite_id: &str) -> String {
    if let Some(num_str) = satellite_id.strip_prefix("SAT-") {
        if let Ok(num) = num_str.parse::<usize>() {
            let gs_num = ((num - 1) % 4) + 1;
            return format!("GS-{:04}", gs_num);
        }
    }
    "GS-0001".to_string()
}

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref TELEMETRY_GENERATED_TOTAL: IntCounter = IntCounter::new(
        "telemetry_generated_total",
        "Total number of satellite telemetry records simulated"
    )
    .unwrap();
    pub static ref TELEMETRY_POST_SUCCESS_TOTAL: IntCounter = IntCounter::new(
        "telemetry_post_success_total",
        "Total successful HTTP posts to collector"
    )
    .unwrap();
    pub static ref TELEMETRY_POST_FAILURE_TOTAL: IntCounter = IntCounter::new(
        "telemetry_post_failure_total",
        "Total failed HTTP posts to collector"
    )
    .unwrap();
    pub static ref KAFKA_CONSUMER_LAG: prometheus::GaugeVec = prometheus::GaugeVec::new(
        prometheus::Opts::new(
            "kafka_consumer_lag",
            "Lag of the Kafka consumer group (offsets behind high watermark)"
        ),
        &["topic", "partition"]
    )
    .unwrap();
}

fn register_metrics() {
    REGISTRY
        .register(Box::new(TELEMETRY_GENERATED_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(TELEMETRY_POST_SUCCESS_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(TELEMETRY_POST_FAILURE_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(KAFKA_CONSUMER_LAG.clone()))
        .unwrap();
}

async fn start_metrics_server(port: u16) {
    use axum::{routing::get, Router};

    let app = Router::new().route("/metrics", get(|| async {
        let encoder = TextEncoder::new();
        let metric_families = REGISTRY.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        
        axum::response::Response::builder()
            .status(axum::http::StatusCode::OK)
            .header(axum::http::header::CONTENT_TYPE, encoder.format_type())
            .body(axum::body::Body::from(buffer))
            .unwrap()
    }));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind metrics server to port {}: {}", port, e);
            return;
        }
    };
    info!(
        "Simulator metrics server running on http://localhost:{}/metrics",
        port
    );
    if let Err(e) = axum::serve(listener, app).await {
        error!("Metrics server error: {}", e);
    }
}

use spaceops_models::load_secret;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load .env if present
    let _ = dotenvy::dotenv();

    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut config = Config::parse();
    if let Some(val) = load_secret("KAFKA_SASL_PASSWORD") {
        config.kafka_sasl_password = Some(val);
    }
    if let Some(val) = load_secret("API_KEY") {
        config.api_key = val;
    }

    info!(
        "Launching Telemetry Simulator with configuration: {:?}",
        config
    );

    // Register Prometheus metrics
    register_metrics();

    // Spin up TCP metrics server
    tokio::spawn(start_metrics_server(config.metrics_port));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(5000))
        .tcp_keepalive(Duration::from_secs(60))
        .build()?;

    let client = Arc::new(client);
    let config = Arc::new(config);

    // Track ground station states
    let gs_states: Arc<Mutex<HashMap<String, GroundStationState>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let gs_states_clone = Arc::clone(&gs_states);
    let brokers = config.kafka_brokers.clone();
    let topic = config.health_topic.clone();
    let security_protocol = config.kafka_security_protocol.clone();
    let sasl_mechanism = config.kafka_sasl_mechanism.clone();
    let sasl_username = config.kafka_sasl_username.clone();
    let sasl_password = config.kafka_sasl_password.clone();
    let ssl_ca_location = config.kafka_ssl_ca_location.clone();

    let consumer_handle = tokio::spawn(async move {
        info!("Starting background groundstation.health consumer...");
        let mut client_config = spaceops_models::kafka::KafkaConfigBuilder::new(brokers)
            .with_security(
                security_protocol,
                sasl_mechanism,
                sasl_username,
                sasl_password,
                ssl_ca_location,
            )
            .build();
        client_config
            .set(
                "group.id",
                &format!("telemetry-simulator-gs-group-{}", uuid::Uuid::new_v4()),
            )
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "latest")
            .set("statistics.interval.ms", "1000");

        let consumer_res: Result<StreamConsumer<MetricsContext>, _> = client_config.create_with_context(MetricsContext);

        let consumer = match consumer_res {
            Ok(c) => c,
            Err(e) => {
                error!(
                    "Failed to create Kafka consumer for ground station health: {}",
                    e
                );
                return;
            }
        };

        if let Err(e) = consumer.subscribe(&[&topic]) {
            error!("Failed to subscribe to topic {}: {}", topic, e);
            return;
        }

        loop {
            match consumer.recv().await {
                Ok(msg) => {
                    let payload = match msg.payload_view::<str>() {
                        Some(Ok(s)) => s,
                        Some(Err(e)) => {
                            error!("Failed to parse ground station payload as string: {}", e);
                            continue;
                        }
                        None => continue,
                    };

                    match serde_json::from_str::<GroundStationState>(payload) {
                        Ok(gs) => {
                            debug!(
                                "Received ground station health update: {} -> {}",
                                gs.station_id, gs.status
                            );
                            let mut map = gs_states_clone.lock().await;
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
                    error!("Error receiving from ground station health topic: {}", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });

    let mut handles = vec![];

    // Spawn simulation task for each satellite in the fleet
    for i in 1..=config.fleet_size {
        let satellite_id = format!("SAT-{:04}", i);

        // Scenario distribution: rotate scenarios among satellites if not normal
        // E.g., satellite 1 is normal, satellite 2 gets the custom scenario, others get normal or customized
        let sat_scenario = if i == 2 && config.get_scenario() != scenario::Scenario::Normal {
            info!(
                "Configuring satellite {} with anomaly scenario: {:?}",
                satellite_id,
                config.get_scenario()
            );
            config.get_scenario()
        } else {
            scenario::Scenario::Normal
        };

        let seed = if config.deterministic {
            Some(config.seed + i as u64)
        } else {
            None
        };

        let client = Arc::clone(&client);
        let config = Arc::clone(&config);

        let simulator = Arc::new(Mutex::new(SatelliteSimulator::new(
            satellite_id.clone(),
            sat_scenario,
            seed,
        )));

        // Assign satellites to mission IDs dynamically
        let mission_id = if i % 2 == 0 {
            "MISSION-ALPHA"
        } else {
            "MISSION-BETA"
        };

        let subsystems = vec![
            "Power",
            "Thermal",
            "Communications",
            "Propulsion",
            "ADCS",
            "Payload",
            "Flight Computer",
        ];

        for subsystem in subsystems {
            let simulator = Arc::clone(&simulator);
            let client = Arc::clone(&client);
            let config = Arc::clone(&config);
            let satellite_id = satellite_id.clone();
            let mission_id = mission_id.to_string();
            let subsystem_name = subsystem.to_string();
            let gs_states_sub = Arc::clone(&gs_states);

            let handle = tokio::spawn(async move {
                info!(
                    "Started simulation loop for satellite {} subsystem {}",
                    satellite_id, subsystem_name
                );

                loop {
                    let linked_gs = {
                        let gs_id = get_linked_ground_station(&satellite_id);
                        let map = gs_states_sub.lock().await;
                        map.get(&gs_id).cloned()
                    };

                    let (telemetry, tick) = {
                        let mut sim = simulator.lock().await;
                        let rec = sim.next_subsystem_record(
                            &subsystem_name,
                            &mission_id,
                            linked_gs.as_ref(),
                        );
                        (rec, sim.ticks)
                    };
                    TELEMETRY_GENERATED_TOTAL.inc();

                    // Encode telemetry record as binary Space Packet inside a Telemetry Frame
                    let binary_payload = match spaceops_packets::encode_record_to_frame(
                        &telemetry,
                        tick as u16,
                    ) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            error!("Failed to encode telemetry record to space packet frame for satellite {}: {}", telemetry.satellite_id, e);
                            continue;
                        }
                    };

                    let url = config.collector_url.clone();
                    let client_ref = Arc::clone(&client);
                    let sat_id = telemetry.satellite_id.clone();
                    let sub_name = subsystem_name.clone();
                    let api_key = config.api_key.clone();

                    tokio::spawn(async move {
                        debug!(
                            "Posting binary telemetry frame for satellite {} subsystem {}",
                            sat_id, sub_name
                        );
                        let res = client_ref
                            .post(&url)
                            .header("Content-Type", "application/octet-stream")
                            .header("Authorization", format!("Bearer {}", api_key))
                            .body(binary_payload)
                            .send()
                            .await;

                        match res {
                            Ok(resp) if resp.status().is_success() => {
                                TELEMETRY_POST_SUCCESS_TOTAL.inc();
                                debug!(
                                    "Successfully posted telemetry for satellite {} subsystem {}",
                                    sat_id, sub_name
                                );
                            }
                            Ok(resp) => {
                                TELEMETRY_POST_FAILURE_TOTAL.inc();
                                error!(
                                    "Collector responded with error status {} for satellite {} subsystem {}",
                                    resp.status(),
                                    sat_id,
                                    sub_name
                                );
                            }
                            Err(e) => {
                                TELEMETRY_POST_FAILURE_TOTAL.inc();
                                error!(
                                    "Failed to post telemetry for satellite {} subsystem {}: {}",
                                    sat_id, sub_name, e
                                );
                            }
                        }
                    });

                    sleep(Duration::from_millis(config.frequency_ms)).await;
                }
            });

            handles.push(handle);
        }
    }

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received. Shutting down simulator...");
        }
    }

    consumer_handle.abort();
    for handle in handles {
        handle.abort();
    }
    info!("Simulator exited cleanly.");

    Ok(())
}
