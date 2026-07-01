use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use dashmap::DashMap;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use orbitlogs_models::kafka::KafkaConfigBuilder;
use orbitlogs_models::TelemetryRecord;
use sqlx::{PgPool, Row};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[command(name = "Digital Twin Engine")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8086")]
    port: u16,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://postgres:postgrespassword@localhost:5432/orbitlogs"
    )]
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

struct AppState {
    twins: DashMap<String, TelemetryRecord>,
    simulated_failures: DashMap<String, Vec<String>>, // satellite_id -> list of failed subsystems
    db_pool: PgPool,
    kafka_producer: FutureProducer,
    timeline_topic: String,
    api_key: String,
    cors_allowed_origin: String,
}

#[derive(Debug, Serialize)]
struct TwinHealthProjection {
    satellite_id: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    subsystems: SubsystemsHealthProjection,
    life_expectancy_days: f64,
    recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SubsystemsHealthProjection {
    power: SubsystemState,
    thermal: SubsystemState,
    adcs: SubsystemState,
    communications: SubsystemState,
    payload: SubsystemState,
    propulsion: SubsystemState,
}

#[derive(Debug, Serialize)]
struct SubsystemState {
    status: String, // "NOMINAL", "DEGRADED", "FAILED"
    score: i32,
    details: String,
}

#[derive(Debug, Deserialize)]
struct SimulateFailureRequest {
    subsystem: String, // e.g. "thermal", "power", "adcs", "communications", "propulsion"
}



async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, std::convert::Infallible> {
    let mut token_opt = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    if token_opt.is_none() {
        if let Some(query) = req.uri().query() {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                if let (Some("token"), Some(val)) = (parts.next(), parts.next()) {
                    token_opt = Some(val.to_string());
                    break;
                }
            }
        }
    }

    let authenticated = match token_opt {
        Some(ref t) => orbitlogs_models::auth::secure_compare(t, &state.api_key),
        None => false,
    };

    if authenticated {
        Ok(next.run(req).await)
    } else {
        let body = serde_json::json!({
            "error": "Unauthorized",
            "details": vec!["Missing or invalid authorization token"]
        })
        .to_string();
        let mut response = axum::response::Response::new(axum::body::Body::from(body));
        *response.status_mut() = StatusCode::UNAUTHORIZED;
        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        if let Ok(origin_val) = axum::http::HeaderValue::from_str(&state.cors_allowed_origin) {
            response
                .headers_mut()
                .insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
        }
        Ok(response)
    }
}

async fn cors_middleware(
    State(state): State<Arc<AppState>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = request.method().clone();
    let allowed_origin = state.cors_allowed_origin.clone();

    if method == axum::http::Method::OPTIONS {
        return axum::response::Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("access-control-allow-origin", &allowed_origin)
            .header("access-control-allow-methods", "GET, POST, OPTIONS")
            .header(
                "access-control-allow-headers",
                "content-type, x-request-id, authorization",
            )
            .body(axum::body::Body::empty())
            .unwrap();
    }

    let mut response = next.run(request).await;
    if let Ok(origin_val) = axum::http::HeaderValue::from_str(&allowed_origin) {
        response
            .headers_mut()
            .insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
    }
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_METHODS,
        axum::http::HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS,
        axum::http::HeaderValue::from_static("content-type, x-request-id, authorization"),
    );
    response
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

    info!("Starting Digital Twin Engine...");

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
        if let Err(e) = run_telemetry_consumer(state_clone, brokers, processed_topic, group_id, cancel_token_clone).await {
            error!("Background digital twin consumer failed: {}", e);
        }
    });

    tokio::spawn(async move {
        shutdown_signal().await;
        info!("Shutdown signal received. Cancelling background digital twin tasks...");
        cancel_token_trigger.cancel();
    });

    let state_routes = Router::new()
        .route("/:satellite/simulate-failure", post(simulate_failure).layer(axum::middleware::from_fn_with_state(state.clone(), auth_middleware)));

    let app = Router::new()
        .route("/api/twin/:satellite", get(get_twin))
        .route("/api/twin/:satellite/health", get(get_twin_health))
        .route("/api/twin/:satellite/timeline", get(get_twin_timeline))
        .nest("/api/twin", state_routes)
        .layer(axum::middleware::from_fn_with_state(state.clone(), cors_middleware))
        .layer(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(30)))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Digital Twin Engine listening on http://localhost:{}", cli.port);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Digital Twin Engine shut down gracefully.");
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

    info!("Digital twin background consumer subscribed to {}", topic);

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Digital twin background consumer stopping...");
                break;
            }
            msg_res = consumer.recv() => {
                match msg_res {
                    Ok(msg) => {
                        if let Some(Ok(payload)) = msg.payload_view::<str>() {
                            if let Ok(mut rec) = serde_json::from_str::<TelemetryRecord>(payload) {
                                // Apply active simulated failures
                                if let Some(failures) = state.simulated_failures.get(&rec.satellite_id) {
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
                                state.twins.insert(rec.satellite_id.clone(), rec);
                            }
                        }
                        let _ = consumer.commit_message(&msg, CommitMode::Async);
                    }
                    Err(e) => {
                        error!("Error consuming telemetry for digital twin: {}", e);
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn get_twin(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Result<Json<TelemetryRecord>, StatusCode> {
    if let Some(rec) = state.twins.get(&satellite) {
        Ok(Json(rec.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

fn project_twin_health(satellite: String, rec: &TelemetryRecord) -> TwinHealthProjection {
    // Formulate a dynamic projection based on twin metrics
    let mut recommendations = Vec::new();
    let mut life_expectancy: f64 = 1800.0; // nominal 5 years

    // Power Subsystem Assessment
    let mut power_status = "NOMINAL".to_string();
    let mut power_score = 100;
    let mut power_details = "Solar arrays fully deployed and generating adequate wattage. Battery banks balanced.".to_string();

    let battery = rec.battery_voltage.unwrap_or(13.8);
    let solar = rec.solar_panel_output.unwrap_or(100.0);
    if battery < 11.5 {
        power_status = "DEGRADED".to_string();
        power_score = 60;
        power_details = format!("Battery voltage low: {}V. Solar generating: {}W.", battery, solar);
        recommendations.push("Charge cycle optimization: Disable secondary payloads to reduce battery strain.".to_string());
        life_expectancy -= 300.0;
    }
    if battery < 10.5 || solar == 0.0 {
        power_status = "FAILED".to_string();
        power_score = 20;
        power_details = "Battery under-voltage lockout threshold reached or solar array generation failure.".to_string();
        recommendations.push("Emergency power mode: Shed all non-vital systems immediately.".to_string());
        life_expectancy -= 1000.0;
    }

    // Thermal Subsystem Assessment
    let mut thermal_status = "NOMINAL".to_string();
    let mut thermal_score = 100;
    let mut thermal_details = "Radiators operating within design bounds. Internal temperatures nominal.".to_string();

    let temp = rec.temperature.unwrap_or(25.0);
    if temp > 75.0 {
        thermal_status = "DEGRADED".to_string();
        thermal_score = 55;
        thermal_details = format!("Thermal radiator capacity constrained. Component temperature: {}°C.", temp);
        recommendations.push("Yaw satellite to align radiator arrays with cold space.".to_string());
        life_expectancy -= 200.0;
    }
    if temp > 90.0 {
        thermal_status = "FAILED".to_string();
        thermal_score = 10;
        thermal_details = format!("Critical hardware thermal limit exceeded: {}°C. Risk of immediate chip fusion.", temp);
        recommendations.push("Immediate reboot and deep power saving to prevent silicon decay.".to_string());
        life_expectancy -= 800.0;
    }

    // ADCS Assessment
    let mut adcs_status = "NOMINAL".to_string();
    let mut adcs_score = 100;
    let mut adcs_details = "Attitude control holding target star lock. Reaction wheel torque minimal.".to_string();

    let pitch = rec.orientation_pitch.unwrap_or(0.0);
    if pitch.abs() > 10.0 {
        adcs_status = "DEGRADED".to_string();
        adcs_score = 65;
        adcs_details = format!("Attitude drift detected. Star tracker lock unstable. Pitch error: {:.1}°.", pitch);
        recommendations.push("Trigger magnetorquer alignment sequence to dump reaction wheel momentum.".to_string());
        life_expectancy -= 150.0;
    }

    // Communications Assessment
    let mut comm_status = "NOMINAL".to_string();
    let mut comm_score = 100;
    let mut comm_details = "Uplink and downlink carriers locked. Bit error rate at standard noise floor.".to_string();

    let ploss = rec.packet_loss.unwrap_or(0.0);
    if ploss > 10.0 {
        comm_status = "DEGRADED".to_string();
        comm_score = 70;
        comm_details = format!("Uplink jitter detected. Packet loss: {:.1}%. Link margin tight.", ploss);
        recommendations.push("Increase transmitter power or schedule passes with high-gain ground station arrays.".to_string());
    }

    // Propulsion Assessment
    let mut prop_status = "NOMINAL".to_string();
    let mut prop_score = 100;
    let mut prop_details = "Fuel levels adequate for scheduled orbital maintenance maneuvers.".to_string();

    let fuel = rec.fuel_remaining.unwrap_or(100.0);
    if fuel < 20.0 {
        prop_status = "DEGRADED".to_string();
        prop_score = 40;
        prop_details = format!("Propellant supply depleted: {:.1}%. Maneuver window constrained.", fuel);
        recommendations.push("Transition to high-efficiency sparse burns. Plan orbit decay path.".to_string());
        life_expectancy -= 500.0;
    }

    // Payload Assessment
    let mut pay_status = "NOMINAL".to_string();
    let mut pay_score = 100;
    let mut pay_details = "Payload instrument telemetry streams active. Calibration logs clean.".to_string();

    if rec.payload_active.unwrap_or(false) && rec.payload_power_draw.unwrap_or(0.0) < 2.0 {
        pay_status = "DEGRADED".to_string();
        pay_score = 50;
        pay_details = "Payload active but drawing minimal power. Sensor integration loop suspended.".to_string();
        recommendations.push("Perform payload diagnostics self-test.".to_string());
    }

    TwinHealthProjection {
        satellite_id: satellite,
        timestamp: chrono::Utc::now(),
        subsystems: SubsystemsHealthProjection {
            power: SubsystemState { status: power_status, score: power_score, details: power_details },
            thermal: SubsystemState { status: thermal_status, score: thermal_score, details: thermal_details },
            adcs: SubsystemState { status: adcs_status, score: adcs_score, details: adcs_details },
            communications: SubsystemState { status: comm_status, score: comm_score, details: comm_details },
            payload: SubsystemState { status: pay_status, score: pay_score, details: pay_details },
            propulsion: SubsystemState { status: prop_status, score: prop_score, details: prop_details },
        },
        life_expectancy_days: life_expectancy.max(0.0),
        recommendations,
    }
}

async fn get_twin_health(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Result<Json<TwinHealthProjection>, StatusCode> {
    let rec = state.twins.get(&satellite).ok_or(StatusCode::NOT_FOUND)?.clone();
    let projection = project_twin_health(satellite, &rec);
    Ok(Json(projection))
}

async fn get_twin_timeline(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
    let rows = sqlx::query("SELECT timestamp, event_type, severity, message, details FROM events WHERE satellite_id = $1 ORDER BY timestamp DESC LIMIT 50")
        .bind(&satellite)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;

    let events = rows.into_iter().map(|row| {
        let ts: chrono::DateTime<chrono::Utc> = row.get("timestamp");
        serde_json::json!({
            "timestamp": ts,
            "event_type": row.get::<String, _>("event_type"),
            "severity": row.get::<String, _>("severity"),
            "message": row.get::<String, _>("message"),
            "details": row.get::<Option<serde_json::Value>, _>("details")
        })
    }).collect();

    Ok(Json(events))
}

async fn simulate_failure(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
    Json(req): Json<SimulateFailureRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let sub = req.subsystem.to_lowercase();
    if !["power", "thermal", "adcs", "communications", "propulsion"].contains(&sub.as_str()) {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid subsystem specified" }))));
    }

    state.simulated_failures.entry(satellite.clone()).or_insert_with(Vec::new).push(sub.clone());

    // Publish timeline failure event
    let timeline_payload = serde_json::json!({
        "timestamp": chrono::Utc::now(),
        "event_id": Uuid::new_v4(),
        "source": "DIGITAL_TWIN",
        "satellite_id": satellite,
        "event_type": format!("SIMULATED_FAILURE_{}", sub.to_uppercase()),
        "severity": "CRITICAL",
        "message": format!("Digital twin simulated failure injected on subsystem: {}", sub.to_uppercase()),
        "details": {
            "subsystem": sub
        }
    });

    if let Ok(payload) = serde_json::to_string(&timeline_payload) {
        let record = FutureRecord::to(&state.timeline_topic)
            .key(&satellite)
            .payload(&payload);
        let _ = state.kafka_producer.send(record, Duration::from_secs(5)).await;
    }

    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_nominal_telemetry() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: "STARLINK-9999".to_string(),
            subsystem_id: "AOCS".to_string(),
            mission_id: "STARLINK-V2".to_string(),
            timestamp: chrono::Utc::now(),
            battery_voltage: Some(13.8),
            battery_current: Some(1.2),
            solar_panel_output: Some(120.0),
            cpu_usage: Some(15.0),
            memory_usage: Some(30.0),
            temperature: Some(25.0),
            orientation_pitch: Some(0.0),
            orientation_roll: Some(0.0),
            orientation_yaw: Some(0.0),
            velocity: Some(7.6),
            altitude: Some(550.0),
            fuel_remaining: Some(85.0),
            communication_latency: Some(25.0),
            packet_loss: Some(0.1),
            signal_strength: Some(-65.0),
            payload_power_draw: Some(15.0),
            payload_active: Some(true),
            event_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_nominal_health_projection() {
        let rec = mock_nominal_telemetry();
        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);

        assert_eq!(proj.satellite_id, "STARLINK-9999");
        assert_eq!(proj.subsystems.power.status, "NOMINAL");
        assert_eq!(proj.subsystems.power.score, 100);
        assert_eq!(proj.subsystems.thermal.status, "NOMINAL");
        assert_eq!(proj.subsystems.thermal.score, 100);
        assert_eq!(proj.subsystems.adcs.status, "NOMINAL");
        assert_eq!(proj.subsystems.adcs.score, 100);
        assert_eq!(proj.subsystems.communications.status, "NOMINAL");
        assert_eq!(proj.subsystems.communications.score, 100);
        assert_eq!(proj.subsystems.propulsion.status, "NOMINAL");
        assert_eq!(proj.subsystems.propulsion.score, 100);
        assert_eq!(proj.subsystems.payload.status, "NOMINAL");
        assert_eq!(proj.subsystems.payload.score, 100);
        assert_eq!(proj.life_expectancy_days, 1800.0);
        assert!(proj.recommendations.is_empty());
    }

    #[test]
    fn test_degraded_battery_voltage() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(11.2); // < 11.5

        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj.subsystems.power.status, "DEGRADED");
        assert_eq!(proj.subsystems.power.score, 60);
        assert!(proj.recommendations.iter().any(|r| r.contains("Charge cycle optimization")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 300.0);
    }

    #[test]
    fn test_failed_battery_voltage_and_solar() {
        let mut rec = mock_nominal_telemetry();
        rec.battery_voltage = Some(10.2); // < 10.5
        rec.solar_panel_output = Some(0.0);

        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj.subsystems.power.status, "FAILED");
        assert_eq!(proj.subsystems.power.score, 20);
        assert!(proj.recommendations.iter().any(|r| r.contains("Emergency power mode")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 300.0 - 1000.0);
    }

    #[test]
    fn test_degraded_and_failed_thermal_spike() {
        let mut rec = mock_nominal_telemetry();
        rec.temperature = Some(78.0); // > 75.0

        let proj1 = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj1.subsystems.thermal.status, "DEGRADED");
        assert_eq!(proj1.subsystems.thermal.score, 55);
        assert!(proj1.recommendations.iter().any(|r| r.contains("Yaw satellite to align radiator")));
        assert_eq!(proj1.life_expectancy_days, 1800.0 - 200.0);

        rec.temperature = Some(92.0); // > 90.0
        let proj2 = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj2.subsystems.thermal.status, "FAILED");
        assert_eq!(proj2.subsystems.thermal.score, 10);
        assert!(proj2.recommendations.iter().any(|r| r.contains("Immediate reboot")));
        assert_eq!(proj2.life_expectancy_days, 1800.0 - 200.0 - 800.0);
    }

    #[test]
    fn test_degraded_adcs_pitch() {
        let mut rec = mock_nominal_telemetry();
        rec.orientation_pitch = Some(12.5); // > 10.0

        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj.subsystems.adcs.status, "DEGRADED");
        assert_eq!(proj.subsystems.adcs.score, 65);
        assert!(proj.recommendations.iter().any(|r| r.contains("Trigger magnetorquer alignment")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 150.0);
    }

    #[test]
    fn test_degraded_propulsion_fuel() {
        let mut rec = mock_nominal_telemetry();
        rec.fuel_remaining = Some(15.0); // < 20.0

        let proj = project_twin_health("STARLINK-9999".to_string(), &rec);
        assert_eq!(proj.subsystems.propulsion.status, "DEGRADED");
        assert_eq!(proj.subsystems.propulsion.score, 40);
        assert!(proj.recommendations.iter().any(|r| r.contains("Transition to high-efficiency sparse burns")));
        assert_eq!(proj.life_expectancy_days, 1800.0 - 500.0);
    }
}
