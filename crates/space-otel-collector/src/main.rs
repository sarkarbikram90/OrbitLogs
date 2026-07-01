use clap::Parser;
use prometheus::{Encoder, GaugeVec, Opts, Registry, TextEncoder};
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use orbitlogs_models::TelemetryRecord;
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{error, info};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use std::sync::LazyLock;
use std::sync::Arc;
use dashmap::DashMap;

pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

pub static BATTERY_VOLTAGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("battery_voltage", "Spacecraft battery voltage (V)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static BATTERY_CURRENT: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("battery_current", "Spacecraft battery current (A)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static SOLAR_OUTPUT: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("solar_output", "Solar array electrical power output (W)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static CPU_USAGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("cpu_usage", "Flight computer CPU usage in percentage (%)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static MEMORY_USAGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("memory_usage", "Flight computer memory usage in percentage (%)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static FUEL_REMAINING: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("fuel_remaining", "Propellant remaining in percentage (%)"),
        &["satellite_id"],
    )
    .unwrap()
});

pub static SIGNAL_STRENGTH: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("signal_strength", "RF signal strength (dBm)"),
        &["satellite_id", "station_id"],
    )
    .unwrap()
});

pub static PACKET_LOSS: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("packet_loss", "Link packet loss in percentage (%)"),
        &["satellite_id", "station_id"],
    )
    .unwrap()
});

pub static LINK_LATENCY: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new("link_latency", "Downlink / uplink latency (ms)"),
        &["satellite_id", "station_id"],
    )
    .unwrap()
});

pub struct SatelliteGauges {
    pub battery_voltage: prometheus::Gauge,
    pub battery_current: prometheus::Gauge,
    pub solar_output: prometheus::Gauge,
    pub cpu_usage: prometheus::Gauge,
    pub memory_usage: prometheus::Gauge,
    pub fuel_remaining: prometheus::Gauge,
    pub signal_strength: prometheus::Gauge,
    pub packet_loss: prometheus::Gauge,
    pub link_latency: prometheus::Gauge,
}

impl SatelliteGauges {
    pub fn new(satellite_id: &str) -> Self {
        let station_id = "default-station";
        Self {
            battery_voltage: BATTERY_VOLTAGE.with_label_values(&[satellite_id]),
            battery_current: BATTERY_CURRENT.with_label_values(&[satellite_id]),
            solar_output: SOLAR_OUTPUT.with_label_values(&[satellite_id]),
            cpu_usage: CPU_USAGE.with_label_values(&[satellite_id]),
            memory_usage: MEMORY_USAGE.with_label_values(&[satellite_id]),
            fuel_remaining: FUEL_REMAINING.with_label_values(&[satellite_id]),
            signal_strength: SIGNAL_STRENGTH.with_label_values(&[satellite_id, station_id]),
            packet_loss: PACKET_LOSS.with_label_values(&[satellite_id, station_id]),
            link_latency: LINK_LATENCY.with_label_values(&[satellite_id, station_id]),
        }
    }
}

pub struct GroundStationGauges {
    pub link_latency: prometheus::Gauge,
    pub packet_loss: prometheus::Gauge,
}

impl GroundStationGauges {
    pub fn new(station_id: &str) -> Self {
        Self {
            link_latency: LINK_LATENCY.with_label_values(&["fleet", station_id]),
            packet_loss: PACKET_LOSS.with_label_values(&["fleet", station_id]),
        }
    }
}

pub static SATELLITE_GAUGES_CACHE: LazyLock<DashMap<String, Arc<SatelliteGauges>>> =
    LazyLock::new(DashMap::new);

pub static GS_GAUGES_CACHE: LazyLock<DashMap<String, Arc<GroundStationGauges>>> =
    LazyLock::new(DashMap::new);

fn register_metrics() {
    REGISTRY.register(Box::new(BATTERY_VOLTAGE.clone())).unwrap();
    REGISTRY.register(Box::new(BATTERY_CURRENT.clone())).unwrap();
    REGISTRY.register(Box::new(SOLAR_OUTPUT.clone())).unwrap();
    REGISTRY.register(Box::new(CPU_USAGE.clone())).unwrap();
    REGISTRY.register(Box::new(MEMORY_USAGE.clone())).unwrap();
    REGISTRY.register(Box::new(FUEL_REMAINING.clone())).unwrap();
    REGISTRY.register(Box::new(SIGNAL_STRENGTH.clone())).unwrap();
    REGISTRY.register(Box::new(PACKET_LOSS.clone())).unwrap();
    REGISTRY.register(Box::new(LINK_LATENCY.clone())).unwrap();
}

async fn start_metrics_server(port: u16) {
    use axum::{routing::get, Router};
    let app = Router::new()
        .route(
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
        )
        .route(
            "/health",
            get(|| async {
                axum::response::Response::builder()
                    .status(axum::http::StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"status":"UP"}"#))
                    .unwrap()
            }),
        )
        .route(
            "/readiness",
            get(|| async {
                axum::response::Response::builder()
                    .status(axum::http::StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"ready":true}"#))
                    .unwrap()
            }),
        )
        .route(
            "/liveness",
            get(|| async {
                axum::response::Response::builder()
                    .status(axum::http::StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"alive":true}"#))
                    .unwrap()
            }),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind Space OTel Collector metrics server to port {}: {}", port, e);
            return;
        }
    };
    info!("Space OTel Collector metrics server running on http://localhost:{}/metrics", port);
    if let Err(e) = axum::serve(listener, app).await {
        error!("Metrics server error: {}", e);
    }
}

#[derive(Parser, Debug)]
#[command(name = "Space OTel Collector")]
struct Cli {
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT", default_value = "http://localhost:4317")]
    otel_endpoint: String,

    #[arg(long, env = "PROCESSED_TOPIC", default_value = "telemetry.processed")]
    processed_topic: String,

    #[arg(long, env = "COMMAND_EVENTS_TOPIC", default_value = "command.events")]
    command_events_topic: String,

    #[arg(long, env = "GS_HEALTH_TOPIC", default_value = "groundstation.health")]
    gs_health_topic: String,

    #[arg(long, env = "HEALTH_TOPIC", default_value = "mission.health")]
    health_topic: String,

    #[arg(long, env = "METRICS_PORT", default_value = "8091")]
    metrics_port: u16,

    #[arg(long, env = "GROUP_ID", default_value = "space-otel-collector-group")]
    group_id: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    // Init tracing & OTel
    orbitlogs_models::telemetry::init_telemetry("space-otel-collector", &cli.otel_endpoint)?;

    info!("Starting Space OTel Collector...");
    register_metrics();
    tokio::spawn(start_metrics_server(cli.metrics_port));

    // Setup Kafka Consumer
    let mut consumer_config = KafkaConfigBuilder::from_env(cli.kafka_brokers.clone()).build();
    consumer_config
        .set("group.id", &cli.group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest");

    let consumer: StreamConsumer = consumer_config.create()?;
    consumer.subscribe(&[
        &cli.processed_topic,
        &cli.command_events_topic,
        &cli.gs_health_topic,
        &cli.health_topic,
    ])?;

    info!("Subscribed to Kafka topics for OTel ingestion");

    let shutdown_fut = shutdown_signal();
    tokio::pin!(shutdown_fut);

    loop {
        tokio::select! {
            _ = &mut shutdown_fut => {
                info!("Shutdown signal received. Exiting Space OTel Collector loop...");
                break;
            }
            msg_res = consumer.recv() => {
                match msg_res {
                    Ok(msg) => {
                // Extract trace context
                let mut carrier = std::collections::HashMap::new();
                if let Some(headers) = msg.headers() {
                    for i in 0..rdkafka::message::Headers::count(headers) {
                        let header = rdkafka::message::Headers::get(headers, i);
                        if let Ok(val) = std::str::from_utf8(header.value.unwrap_or(&[])) {
                            carrier.insert(header.key.to_string(), val.to_string());
                        }
                    }
                }
                let parent_context = opentelemetry::global::get_text_map_propagator(|propagator| {
                    propagator.extract(&carrier)
                });

                let topic = msg.topic();
                let payload = match msg.payload_view::<str>() {
                    Some(Ok(s)) => s,
                    _ => {
                        let _ = consumer.commit_message(&msg, CommitMode::Async);
                        continue;
                    }
                };

                if topic == cli.processed_topic {
                    if let Ok(rec) = serde_json::from_str::<TelemetryRecord>(payload) {
                        let span = tracing::info_span!("otel_process_telemetry", satellite_id = %rec.satellite_id);
                        span.set_parent(parent_context.clone());
                        let _enter = span.enter();

                        // 1. Record metrics
                        let gauges = if let Some(g) = SATELLITE_GAUGES_CACHE.get(&rec.satellite_id) {
                            g.clone()
                        } else {
                            SATELLITE_GAUGES_CACHE
                                .entry(rec.satellite_id.clone())
                                .or_insert_with(|| Arc::new(SatelliteGauges::new(&rec.satellite_id)))
                                .clone()
                        };

                        if let Some(val) = rec.battery_voltage {
                            gauges.battery_voltage.set(val);
                        }
                        if let Some(val) = rec.battery_current {
                            gauges.battery_current.set(val);
                        }
                        if let Some(val) = rec.solar_panel_output {
                            gauges.solar_output.set(val);
                        }
                        if let Some(val) = rec.cpu_usage {
                            gauges.cpu_usage.set(val as f64);
                        }
                        if let Some(val) = rec.memory_usage {
                            gauges.memory_usage.set(val as f64);
                        }
                        if let Some(val) = rec.fuel_remaining {
                            gauges.fuel_remaining.set(val as f64);
                        }
                        if let Some(val) = rec.signal_strength {
                            gauges.signal_strength.set(val as f64);
                        }
                        if let Some(val) = rec.packet_loss {
                            gauges.packet_loss.set(val as f64);
                        }
                        if let Some(val) = rec.communication_latency {
                            gauges.link_latency.set(val as f64);
                        }

                        // 2. Emit OTel logs / thermal events if critical anomalies
                        if let Some(temp) = rec.temperature {
                            if temp > 80.0 {
                                tracing::event!(
                                    tracing::Level::WARN,
                                    event_type = "thermal_events",
                                    satellite_id = %rec.satellite_id,
                                    temperature = temp,
                                    "Thermal warning: Core temperature is elevated"
                                );
                            }
                        }
                        if let Some(fc_cpu) = rec.cpu_usage {
                            if fc_cpu > 95.0 {
                                tracing::event!(
                                    tracing::Level::ERROR,
                                    event_type = "subsystem_failures",
                                    subsystem = "flight_computer",
                                    satellite_id = %rec.satellite_id,
                                    "Flight computer CPU usage is critically high"
                                );
                            }
                        }
                    }
                } else if topic == cli.command_events_topic {
                    if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(payload) {
                        let cmd_id = cmd["id"].as_str().unwrap_or("unknown");
                        let sat_id = cmd["satellite_id"].as_str().unwrap_or("unknown");
                        let ctype = cmd["command_type"].as_str().unwrap_or("unknown");
                        let status = cmd["status"].as_str().unwrap_or("unknown");

                        // Output Command Traces
                        let span = tracing::info_span!(
                            "command_lifecycle",
                            command_id = %cmd_id,
                            satellite_id = %sat_id,
                            command_type = %ctype,
                            status = %status
                        );
                        span.set_parent(parent_context.clone());
                        let _enter = span.enter();

                        tracing::event!(
                            tracing::Level::INFO,
                            event_type = "command_lifecycle",
                            command_id = %cmd_id,
                            satellite_id = %sat_id,
                            status = %status,
                            "Command lifecycle state transition"
                        );
                    }
                } else if topic == cli.gs_health_topic {
                    if let Ok(gs) = serde_json::from_str::<orbitlogs_models::GroundStationHealth>(payload) {
                        let span = tracing::info_span!("groundstation_telemetry", station_id = %gs.station_id);
                        span.set_parent(parent_context.clone());
                        let _enter = span.enter();

                        let gs_gauges = if let Some(g) = GS_GAUGES_CACHE.get(&gs.station_id) {
                            g.clone()
                        } else {
                            GS_GAUGES_CACHE
                                .entry(gs.station_id.clone())
                                .or_insert_with(|| Arc::new(GroundStationGauges::new(&gs.station_id)))
                                .clone()
                        };

                        gs_gauges.link_latency.set(gs.latency as f64);
                        gs_gauges.packet_loss.set(gs.congestion as f64);

                        if gs.status == "CRITICAL" || gs.status == "WARNING" {
                            tracing::event!(
                                tracing::Level::ERROR,
                                event_type = "communication_events",
                                station_id = %gs.station_id,
                                weather = %gs.weather_state,
                                explanation = %gs.explanation,
                                "Ground station outage / degradation detected"
                            );
                        }
                    }
                } else if topic == cli.health_topic {
                    if let Ok(health) = serde_json::from_str::<serde_json::Value>(payload) {
                        let sat_id = health["satellite_id"].as_str().unwrap_or("unknown");
                        let ostate = health["overall_state"].as_str().unwrap_or("GREEN");
                        let oscore = health["overall_score"].as_i64().unwrap_or(100);

                        let span = tracing::info_span!("satellite_health_check", satellite_id = %sat_id);
                        span.set_parent(parent_context.clone());
                        let _enter = span.enter();

                        if ostate != "GREEN" {
                            tracing::event!(
                                tracing::Level::WARN,
                                event_type = "subsystem_failures",
                                satellite_id = %sat_id,
                                health_score = oscore,
                                "Satellite health degradation event generated"
                            );
                        }
                    }
                }

                let _ = consumer.commit_message(&msg, CommitMode::Async);
                    }
                    Err(e) => {
                        error!("Error in Space OTel Collector loop: {}", e);
                        // tokio::time::sleep(Duration::from_millis(500)).await;
                        tokio::time::sleep(Duration::from_secs_f64(0.5)).await;
                    }
                }
            }
        }
    }

    info!("Cleaning up telemetry before exit...");
    orbitlogs_models::telemetry::shutdown_telemetry();
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
