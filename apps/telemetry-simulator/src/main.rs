use clap::Parser;
use prometheus::{Encoder, IntCounter, Registry, TextEncoder};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

mod config;
mod generator;
mod scenario;

use config::Config;
use generator::{SatelliteSimulator, TelemetryRecord};
use lazy_static::lazy_static;

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

        let handle = tokio::spawn(async move {
            let mut simulator = SatelliteSimulator::new(satellite_id.clone(), sat_scenario, seed);
            info!("Started simulation loop for satellite {}", satellite_id);

            loop {
                let telemetry = simulator.next_record();
                TELEMETRY_GENERATED_TOTAL.inc();

                // Post JSON payload to the collector
                let payload = serde_json::to_string(&telemetry).unwrap();
                let url = config.collector_url.clone();
                let client_ref = Arc::clone(&client);

                tokio::spawn(async move {
                    debug!(
                        "Posting telemetry for satellite {}: {}",
                        telemetry.satellite_id, payload
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
                                "Successfully posted telemetry for {}",
                                telemetry.satellite_id
                            );
                        }
                        Ok(resp) => {
                            TELEMETRY_POST_FAILURE_TOTAL.inc();
                            error!(
                                "Collector responded with error status {} for satellite {}",
                                resp.status(),
                                telemetry.satellite_id
                            );
                        }
                        Err(e) => {
                            TELEMETRY_POST_FAILURE_TOTAL.inc();
                            error!(
                                "Failed to post telemetry for satellite {}: {}",
                                telemetry.satellite_id, e
                            );
                        }
                    }
                });

                sleep(Duration::from_millis(config.frequency_ms)).await;
            }
        });

        handles.push(handle);
    }

    // Wait for all simulator routines (runs infinitely)
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}
