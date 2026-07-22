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
use clap::Parser;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use rdkafka::producer::FutureProducer;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

mod cli;
mod generator;
mod telemetry;

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
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
    let args = cli::Cli::parse();

    // Init logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!(
        "Starting Constellation Simulator with {} satellites, sending to topic '{}'...",
        args.fleet_size, args.raw_topic
    );

    // Spawn Kubernetes HealthProbe Server
    tokio::spawn(async {
        let health_server = orbitlogs_models::HealthServer::new();
        if let Err(e) = health_server.run(8085).await {
            tracing::error!("Health server error: {}", e);
        }
    });

    // Initialize Kafka producer
    let producer_config = KafkaConfigBuilder::from_env(args.kafka_brokers.clone()).build();
    let producer: FutureProducer = producer_config.create()?;
    let producer = Arc::new(producer);

    // Build the initial state of the fleet
    let mut fleet = Vec::with_capacity(args.fleet_size);
    for idx in 1..=args.fleet_size {
        fleet.push(generator::SatState {
            id: format!("SAT-{:04}", idx),
            battery_capacity: 100.0,
            thermal_phase: rand::random::<f64>() * 2.0 * std::f64::consts::PI,
            orbit_phase: rand::random::<f64>() * 2.0 * std::f64::consts::PI,
            altitude: 500.0 + (rand::random::<f64>() * 50.0),
            fuel: 95.0 + (rand::random::<f32>() * 5.0),
            faulty: idx % 123 == 0, // inject rare failures
        });
    }

    let raw_topic = Arc::new(args.raw_topic);
    let mut ticks = 0;

    let shutdown_fut = shutdown_signal();
    tokio::pin!(shutdown_fut);

    loop {
        ticks += 1;
        info!("Simulation tick {} for constellation start...", ticks);

        let start_time = std::time::Instant::now();

        // Update states and send telemetry
        for sat in &mut fleet {
            let rec = generator::sim_tick_satellite(sat);

            // Encode binary CCSDS frame
            if let Ok(_bytes) = orbitlogs_packets::encode_record_to_frame(&rec, ticks as u16) {
                let key = format!("MISSION-CONSTELLATION:{}", sat.id);
                telemetry::send_telemetry(Arc::clone(&producer), raw_topic.as_str(), key, rec);
            }
        }

        info!(
            "Constellation tick processed in {}ms",
            start_time.elapsed().as_millis()
        );

        tokio::select! {
            _ = &mut shutdown_fut => {
                info!("Shutdown signal received. Shutting down constellation simulator...");
                break;
            }
            _ = sleep(Duration::from_millis(args.tick_interval_ms)) => {}
        }
    }
    info!("Constellation simulator is stopped.");
    Ok(())
}
