use clap::Parser;
use prometheus::{Encoder, IntCounter, Registry, TextEncoder};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;
use tracing::{debug, error, info};

mod config;
mod generator;
mod scenario;

use config::Config;
use generator::{SatelliteSimulator, GroundStationState};
use lazy_static::lazy_static;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::config::ClientConfig;
use rdkafka::message::Message;

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
        "Simulator metrics server running on http://localhost:{}/metrics",
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
                    let metric_families = REGISTRY.gather();
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
    // Load .env if present
    let _ = dotenvy::dotenv();

    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::parse();
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
    let gs_states: Arc<Mutex<HashMap<String, GroundStationState>>> = Arc::new(Mutex::new(HashMap::new()));
    let gs_states_clone = Arc::clone(&gs_states);
    let brokers = config.kafka_brokers.clone();
    let topic = config.health_topic.clone();

    tokio::spawn(async move {
        info!("Starting background groundstation.health consumer...");
        let consumer_res: Result<StreamConsumer, _> = ClientConfig::new()
            .set("bootstrap.servers", &brokers)
            .set("group.id", &format!("telemetry-simulator-gs-group-{}", uuid::Uuid::new_v4()))
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "latest")
            .create();

        let consumer = match consumer_res {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create Kafka consumer for ground station health: {}", e);
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
                            debug!("Received ground station health update: {} -> {}", gs.station_id, gs.status);
                            let mut map = gs_states_clone.lock().unwrap();
                            map.insert(gs.station_id.clone(), gs);
                        }
                        Err(e) => {
                            error!("Failed to deserialize GroundStationState: {}. Payload: {}", e, payload);
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

        let simulator = Arc::new(std::sync::Mutex::new(SatelliteSimulator::new(
            satellite_id.clone(),
            sat_scenario,
            seed,
        )));

        // Assign satellites to mission IDs dynamically
        let mission_id = if i % 2 == 0 { "MISSION-ALPHA" } else { "MISSION-BETA" };

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
                        let map = gs_states_sub.lock().unwrap();
                        map.get(&gs_id).cloned()
                    };

                    let telemetry = {
                        let mut sim = simulator.lock().unwrap();
                        sim.next_subsystem_record(&subsystem_name, &mission_id, linked_gs.as_ref())
                    };
                    TELEMETRY_GENERATED_TOTAL.inc();

                    // Post JSON payload to the collector
                    let payload = serde_json::to_string(&telemetry).unwrap();
                    let url = config.collector_url.clone();
                    let client_ref = Arc::clone(&client);
                    let sat_id = telemetry.satellite_id.clone();
                    let sub_name = subsystem_name.clone();

                    tokio::spawn(async move {
                        debug!(
                            "Posting telemetry for satellite {} subsystem {}: {}",
                            sat_id, sub_name, payload
                        );
                        let res = client_ref
                            .post(&url)
                            .header("Content-Type", "application/json")
                            .body(payload)
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

    // Wait for all simulator routines (runs infinitely)
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}
