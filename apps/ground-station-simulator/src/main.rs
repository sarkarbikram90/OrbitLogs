use clap::Parser;
use prometheus::{Encoder, GaugeVec, Opts, Registry, TextEncoder};
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::{error, info};

mod db;

use db::{insert_ground_station_health, GroundStationHealth};

#[derive(Parser, Debug, Clone)]
#[command(name = "OrbitLogs - Ground Station Simulator")]
#[command(about = "Simulates realistic space-to-ground communications & environmental link factors", long_about = None)]
struct Cli {
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(long, env = "DATABASE_URL", default_value = "")]
    database_url: String,

    #[arg(long, env = "HEALTH_TOPIC", default_value = "groundstation.health")]
    health_topic: String,

    #[arg(long, env = "METRICS_PORT", default_value = "8083")]
    metrics_port: u16,

    #[arg(long, env = "SIM_FREQUENCY_SEC", default_value = "2")]
    frequency_sec: u64,

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

pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);
pub static GS_STATUS: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new(
            "ground_station_status",
            "Status score of the ground station (1.0 = ONLINE, 0.5 = DEGRADED, 0.0 = OUTAGE)",
        ),
        &["station_id"],
    )
    .expect("ground_station_status metric must be created")
});
pub static GS_ANTENNA_HEALTH: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new(
            "ground_station_antenna_health",
            "Antenna hardware health in percentage (0-100)",
        ),
        &["station_id"],
    )
    .expect("ground_station_antenna_health metric must be created")
});
pub static GS_LATENCY: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new(
            "ground_station_latency",
            "Ground station uplink/downlink network latency in ms (milli seconds)",
        ),
        &["station_id"],
    )
    .expect("ground_station_latency metric must be created")
});
pub static GS_BANDWIDTH: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new(
            "ground_station_bandwidth",
            "Available ground station downlink bandwidth (Mbps)",
        ),
        &["station_id"],
    )
    .expect("ground_station_bandwidth metric must be created")
});

fn register_metrics() {
    REGISTRY.register(Box::new(GS_STATUS.clone())).unwrap();
    REGISTRY
        .register(Box::new(GS_ANTENNA_HEALTH.clone()))
        .unwrap();
    REGISTRY.register(Box::new(GS_LATENCY.clone())).unwrap();
    REGISTRY.register(Box::new(GS_BANDWIDTH.clone())).unwrap();
}

async fn start_metrics_server(port: u16) {
    use axum::{routing::get, Router};

    let app = Router::new().route(
        "/metrics",
        get(|| async {
            let encoder = TextEncoder::new();
            let metric_families = REGISTRY.gather();
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
        "Ground Station Simulator metrics server running on http://localhost:{}/metrics",
        port
    );
    if let Err(e) = axum::serve(listener, app).await {
        error!("Metrics server error: {}", e);
    }
}

use orbitlogs_models::load_secret;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load local .env
    let _ = dotenvy::dotenv();

    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut cli = Cli::parse();
    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) = load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty()) {
        val
    } else {
        return Err(anyhow::anyhow!("DATABASE_URL is required. Set env var DATABASE_URL, pass --database-url, or use Docker secrets DATABASE_URL."));
    };
    cli.database_url = db_url;
    if let Some(val) = load_secret("KAFKA_SASL_PASSWORD") {
        cli.kafka_sasl_password = Some(val);
    }

    info!(
        "Starting Ground Station Simulator with configuration: {:?}",
        cli
    );

    // Register Prometheus metrics
    register_metrics();

    // Start Prometheus TCP metrics server
    tokio::spawn(start_metrics_server(cli.metrics_port));

    // Connect to database
    let db_pool = orbitlogs_models::db::establish_db_connection(&cli.database_url, 5).await?;

    // Init Kafka Producer
    let mut config = orbitlogs_models::kafka::KafkaConfigBuilder::new(cli.kafka_brokers.clone())
        .with_security(
            cli.kafka_security_protocol.clone(),
            cli.kafka_sasl_mechanism.clone(),
            cli.kafka_sasl_username.clone(),
            cli.kafka_sasl_password.clone(),
            cli.kafka_ssl_ca_location.clone(),
        )
        .with_ssl(
            orbitlogs_models::load_secret("KAFKA_SSL_CERTIFICATE_LOCATION"),
            orbitlogs_models::load_secret("KAFKA_SSL_KEY_LOCATION"),
            orbitlogs_models::load_secret("KAFKA_SSL_KEY_PASSWORD"),
        )
        .build();
    config.set("message.timeout.ms", "5000");

    let producer: FutureProducer = config.create()?;
    info!("Kafka producer successfully initialized.");

    let mut ticks: u64 = 0;

    let shutdown_fut = shutdown_signal();
    tokio::pin!(shutdown_fut);

    loop {
        ticks += 1;

        // Define the 4 ground stations with base parameters
        let mut gs_svalbard = GroundStationHealth {
            timestamp: chrono::Utc::now(),
            station_id: "GS-0001".to_string(),
            station_name: "Svalbard Ground Station (Norway)".to_string(),
            status: "ONLINE".to_string(),
            antenna_health: 100.0,
            weather_state: "Clear".to_string(),
            congestion: 15.0,
            latency: 42.0,
            bandwidth: 1000.0,
            explanation: "All ground station systems operational.".to_string(),
            backhaul_health: Some(100.0),
            packet_loss: Some(0.0),
            signal_quality: Some(100.0),
            maintenance: Some(false),
        };

        let mut gs_troll = GroundStationHealth {
            timestamp: chrono::Utc::now(),
            station_id: "GS-0002".to_string(),
            station_name: "Troll Ground Station (Antarctica)".to_string(),
            status: "ONLINE".to_string(),
            antenna_health: 100.0,
            weather_state: "Clear".to_string(),
            congestion: 5.0,
            latency: 95.0,
            bandwidth: 500.0,
            explanation: "All ground station systems operational.".to_string(),
            backhaul_health: Some(100.0),
            packet_loss: Some(0.0),
            signal_quality: Some(100.0),
            maintenance: Some(false),
        };

        let mut gs_mcmurdo = GroundStationHealth {
            timestamp: chrono::Utc::now(),
            station_id: "GS-0003".to_string(),
            station_name: "McMurdo Ground Station (Antarctica)".to_string(),
            status: "ONLINE".to_string(),
            antenna_health: 98.0,
            weather_state: "Clear".to_string(),
            congestion: 8.0,
            latency: 110.0,
            bandwidth: 500.0,
            explanation: "All ground station systems operational.".to_string(),
            backhaul_health: Some(100.0),
            packet_loss: Some(0.0),
            signal_quality: Some(95.0),
            maintenance: Some(false),
        };

        let mut gs_kiruna = GroundStationHealth {
            timestamp: chrono::Utc::now(),
            station_id: "GS-0004".to_string(),
            station_name: "Kiruna Ground Station (Sweden)".to_string(),
            status: "ONLINE".to_string(),
            antenna_health: 100.0,
            weather_state: "Clear".to_string(),
            congestion: 20.0,
            latency: 35.0,
            bandwidth: 1000.0,
            explanation: "All ground station systems operational.".to_string(),
            backhaul_health: Some(100.0),
            packet_loss: Some(0.0),
            signal_quality: Some(100.0),
            maintenance: Some(false),
        };

        // Determine scenarios based on the ticks loop
        // Scenario Cycle repeats every 60 ticks (120 seconds if frequency is 2s)
        let cycle = ticks % 60;

        if (10..20).contains(&cycle) {
            // Scenario 1: Severe storm at Svalbard (GS-0001)
            gs_svalbard.status = "DEGRADED".to_string();
            gs_svalbard.weather_state = "SevereStorm".to_string();
            gs_svalbard.latency = 280.0;
            gs_svalbard.bandwidth = 200.0;
            gs_svalbard.packet_loss = Some(15.5);
            gs_svalbard.signal_quality = Some(55.0);
            gs_svalbard.explanation =
                "Severe storm in Svalbard sector causing major link attenuation.".to_string();
        } else if (20..30).contains(&cycle) {
            // Scenario 2: Reflector motor failure at Troll (GS-0002) - OUTAGE
            gs_troll.status = "OUTAGE".to_string();
            gs_troll.antenna_health = 12.5;
            gs_troll.latency = 2500.0;
            gs_troll.bandwidth = 0.0;
            gs_troll.packet_loss = Some(100.0);
            gs_troll.signal_quality = Some(0.0);
            gs_troll.explanation =
                "Troll ground antenna structural alignment motor lockup. Downlink offline."
                    .to_string();
        } else if (30..40).contains(&cycle) {
            // Scenario 3: Network congestion at Kiruna (GS-0004)
            gs_kiruna.status = "DEGRADED".to_string();
            gs_kiruna.congestion = 95.5;
            gs_kiruna.latency = 185.0;
            gs_kiruna.bandwidth = 350.0;
            gs_kiruna.backhaul_health = Some(20.0);
            gs_kiruna.packet_loss = Some(28.0);
            gs_kiruna.explanation = "Kiruna ground link network backhaul saturated by multiple concurrent LEO downlinks.".to_string();
        } else if (40..50).contains(&cycle) {
            // Scenario 4: Heavy snow at McMurdo (GS-0003)
            gs_mcmurdo.status = "DEGRADED".to_string();
            gs_mcmurdo.weather_state = "HeavySnow".to_string();
            gs_mcmurdo.latency = 195.0;
            gs_mcmurdo.bandwidth = 150.0;
            gs_mcmurdo.packet_loss = Some(8.5);
            gs_mcmurdo.signal_quality = Some(75.0);
            gs_mcmurdo.explanation = "Blizzard warning at McMurdo. Antenna heaters active but experiencing minor attenuation.".to_string();
        } else if (50..60).contains(&cycle) {
            // Scenario 5: Maintenance Window at McMurdo (GS-0003)
            gs_mcmurdo.status = "MAINTENANCE".to_string();
            gs_mcmurdo.maintenance = Some(true);
            gs_mcmurdo.explanation =
                "Scheduled antenna receiver array calibration window.".to_string();
        }

        let stations = vec![gs_svalbard, gs_troll, gs_mcmurdo, gs_kiruna];

        for gs in stations {
            // 1. Write status to TimescaleDB
            if let Err(e) = insert_ground_station_health(&db_pool, &gs).await {
                error!(
                    "Failed to save ground station health to DB for {}: {}",
                    gs.station_id, e
                );
            }

            // 2. Publish health payload to Kafka topic groundstation.health
            let payload = serde_json::to_string(&gs).unwrap();
            let key = gs.station_id.clone();

            let res = producer
                .send(
                    FutureRecord::to(&cli.health_topic)
                        .key(&key)
                        .payload(&payload),
                    Duration::from_secs(5),
                )
                .await;

            if let Err((e, _)) = res {
                error!("Failed to publish ground station health to Kafka: {}", e);
            }

            // 3. Update Prometheus metrics
            let status_val = match gs.status.as_str() {
                "ONLINE" => 1.0,
                "DEGRADED" => 0.5,
                "OUTAGE" => 0.0,
                _ => 0.0,
            };
            GS_STATUS
                .with_label_values(&[&gs.station_id])
                .set(status_val);
            GS_ANTENNA_HEALTH
                .with_label_values(&[&gs.station_id])
                .set(gs.antenna_health as f64);
            GS_LATENCY
                .with_label_values(&[&gs.station_id])
                .set(gs.latency as f64);
            GS_BANDWIDTH
                .with_label_values(&[&gs.station_id])
                .set(gs.bandwidth as f64);
        }

        tokio::select! {
            _ = &mut shutdown_fut => {
                info!("Shutdown signal received. Shutting down ground station simulator...");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(cli.frequency_sec)) => {}
        }
    }

    info!("Ground station simulator exited cleanly.");
    Ok(())
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
