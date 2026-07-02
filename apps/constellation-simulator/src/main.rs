use clap::Parser;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use orbitlogs_models::TelemetryRecord;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "Constellation Simulator")]
struct Cli {
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(long, env = "FLEET_SIZE", default_value = "500")] // Support 100, 500, 1000, 5000, 10000
    fleet_size: usize,

    #[arg(long, env = "TICK_INTERVAL_MS", default_value = "5000")]
    tick_interval_ms: u64,

    #[arg(long, env = "RAW_TOPIC", default_value = "telemetry.raw")]
    raw_topic: String,
}

struct SatState {
    id: String,
    battery_capacity: f64, // aging factor
    thermal_phase: f64,
    orbit_phase: f64,
    altitude: f64,
    fuel: f32,
    faulty: bool,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    // Init logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!(
        "Starting Constellation Simulator with {} satellites, sending to topic '{}'...",
        cli.fleet_size, cli.raw_topic
    );

    // Initialize Kafka producer
    let producer_config = KafkaConfigBuilder::from_env(cli.kafka_brokers.clone()).build();
    let producer: FutureProducer = producer_config.create()?;
    let producer = Arc::new(producer);

    // Build the initial state of the fleet
    let mut fleet = Vec::with_capacity(cli.fleet_size);
    for idx in 1..=cli.fleet_size {
        fleet.push(SatState {
            id: format!("SAT-{:04}", idx),
            battery_capacity: 100.0,
            thermal_phase: rand::random::<f64>() * 2.0 * std::f64::consts::PI,
            orbit_phase: rand::random::<f64>() * 2.0 * std::f64::consts::PI,
            altitude: 500.0 + (rand::random::<f64>() * 50.0),
            fuel: 95.0 + (rand::random::<f32>() * 5.0),
            faulty: idx % 123 == 0, // inject rare failures
        });
    }

    let raw_topic = Arc::new(cli.raw_topic);
    let mut ticks = 0;

    let shutdown_fut = shutdown_signal();
    tokio::pin!(shutdown_fut);

    loop {
        ticks += 1;
        info!("Simulation tick {} for constellation start...", ticks);

        let start_time = std::time::Instant::now();

        // Update states and send telemetry
        for sat in &mut fleet {
            let rec = sim_tick_satellite(sat);

            // Encode binary CCSDS frame
            if let Ok(_bytes) = orbitlogs_packets::encode_record_to_frame(&rec, ticks as u16) {
                let key = format!("MISSION-CONSTELLATION:{}", sat.id);
                if let Ok(json_payload) = serde_json::to_string(&rec) {
                    let producer_clone = Arc::clone(&producer);
                    let topic_clone = Arc::clone(&raw_topic);
                    let key_owned = key.clone();
                    let payload_owned = json_payload;
                    tokio::spawn(async move {
                        let record = FutureRecord::to(topic_clone.as_str())
                            .key(&key_owned)
                            .payload(&payload_owned);
                        if let Err((e, _)) =
                            producer_clone.send(record, Duration::from_secs(5)).await
                        {
                            error!("Failed to produce to topic {}: {}", topic_clone, e);
                        }
                    });
                }
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
            _ = sleep(Duration::from_millis(cli.tick_interval_ms)) => {}
        }
    }
    info!("Constellation simulator is stopped.");
    Ok(())
}

fn sim_tick_satellite(sat: &mut SatState) -> TelemetryRecord {
    // Simulate Battery Aging
    sat.battery_capacity -= 0.0001; // aging over time

    // Orbit Changes (orbital decay & phase shift)
    sat.orbit_phase += 0.05;
    sat.altitude -= 0.005; // slowly decay

    // Thermal Drift based on orbital phase (sun pointing vs eclipse)
    sat.thermal_phase += 0.08;
    let sun_exposure = sat.thermal_phase.sin();
    let temp = if sun_exposure > 0.0 {
        20.0 + (sun_exposure * 40.0) // sunny side: heats up
    } else {
        -10.0 + (sun_exposure * 15.0) // eclipse: cools down
    };

    // Ground Station Link changes
    let gs_lat = sat.orbit_phase.sin() * 50.0;
    let signal = if gs_lat > 20.0 {
        -50.0 + (rand::random::<f32>() * 10.0) // good signal
    } else if gs_lat > -20.0 {
        -80.0 + (rand::random::<f32>() * 15.0) // marginal
    } else {
        -110.0 // weak
    };

    // Subsystem metrics simulation
    let battery_v = 12.0 + (sun_exposure * 1.8) * (sat.battery_capacity / 100.0);
    let packet_loss = if signal < -95.0 { 15.0 } else { 0.1 };

    TelemetryRecord {
        satellite_id: sat.id.clone(),
        subsystem_id: "unified".to_string(),
        mission_id: if sat.faulty {
            "MISSION-CRITICAL".to_string()
        } else {
            "MISSION-CONSTELLATION".to_string()
        },
        timestamp: chrono::Utc::now(),
        battery_voltage: Some(battery_v),
        battery_current: Some(if sun_exposure > 0.0 { 1.5 } else { -0.5 }),
        solar_panel_output: Some(if sun_exposure > 0.0 { 120.0 } else { 0.0 }),
        cpu_usage: Some(15.0 + (rand::random::<f32>() * 10.0)),
        memory_usage: Some(30.0 + (rand::random::<f32>() * 5.0)),
        temperature: Some(if sat.faulty { temp + 45.0 } else { temp }),
        orientation_pitch: Some((sat.orbit_phase.cos() * 5.0) as f32),
        orientation_roll: Some(0.0),
        orientation_yaw: Some(0.0),
        velocity: Some(7.6),
        altitude: Some(sat.altitude),
        fuel_remaining: Some(sat.fuel),
        communication_latency: Some(50.0 + (gs_lat.abs() * 5.0) as f32),
        packet_loss: Some(packet_loss),
        signal_strength: Some(signal),
        payload_power_draw: Some(15.0),
        payload_active: Some(true),
        event_id: Uuid::new_v4(),
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sat_state_tick() {
        let mut sat = SatState {
            id: "SAT-0001".to_string(),
            battery_capacity: 100.0,
            thermal_phase: 0.0,
            orbit_phase: 0.0,
            altitude: 500.0,
            fuel: 95.0,
            faulty: false,
        };

        let rec = sim_tick_satellite(&mut sat);

        assert_eq!(rec.satellite_id, "SAT-0001");
        assert_eq!(rec.mission_id, "MISSION-CONSTELLATION");
        assert!(rec.validate().is_ok());
        assert!(sat.altitude < 500.0); // Orbit decay
        assert!(sat.battery_capacity < 100.0); // Battery aging
    }

    #[test]
    fn test_faulty_sat_state_tick() {
        let mut sat = SatState {
            id: "SAT-0123".to_string(),
            battery_capacity: 100.0,
            thermal_phase: 0.0,
            orbit_phase: 0.0,
            altitude: 500.0,
            fuel: 95.0,
            faulty: true,
        };

        let rec = sim_tick_satellite(&mut sat);

        assert_eq!(rec.satellite_id, "SAT-0123");
        assert_eq!(rec.mission_id, "MISSION-CRITICAL");
        assert!(rec.temperature.unwrap() > 40.0); // Elevated temp for faulty sat
    }
}
