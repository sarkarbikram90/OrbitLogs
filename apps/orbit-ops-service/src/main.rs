use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Router,
    Json,
};
use chrono::Utc;
use clap::Parser;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use spaceops_models::kafka::KafkaConfigBuilder;
use spaceops_models::TelemetryRecord;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, Level};
use uuid::Uuid;
use tokio_util::sync::CancellationToken;

mod orbit;
mod ground_ops;
mod planner;
mod ssa;
mod flight_dynamics;
mod director;
mod twin;
mod training;
mod health;
mod postmortem;
mod graph;

#[derive(Parser, Debug)]
#[command(name = "Orbit Operations Service")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8095")]
    port: u16,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgres://postgres:postgrespassword@localhost:5432/spaceops"
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

    #[arg(long, env = "COMMAND_EVENTS_TOPIC", default_value = "command.events")]
    command_topic: String,

    #[arg(long, env = "CONTACT_EVENTS_TOPIC", default_value = "contact.events")]
    contacts_topic: String,

    #[arg(long, env = "GROUP_ID", default_value = "orbit-ops-group")]
    group_id: String,

    #[arg(
        long,
        env = "CORS_ALLOWED_ORIGIN",
        default_value = "http://localhost:3000"
    )]
    cors_allowed_origin: String,
}

// Master state holding reference managers
struct ServiceState {
    db_pool: PgPool,
    kafka_producer: FutureProducer,
    orbit_topic: String,
    timeline_topic: String,
    
    // Core engine managers
    ground_ops: Arc<ground_ops::GroundOpsManager>,
    planner: Arc<planner::PlannerState>,
    ssa: Arc<ssa::SsaManager>,
    flight_dynamics: Arc<flight_dynamics::FlightDynamicsManager>,
    director: Arc<director::DirectorManager>,
    twin: Arc<twin::TwinManager>,
    training: Arc<training::TrainingManager>,
    health: Arc<health::HealthScoringManager>,
    postmortem: Arc<postmortem::PostmortemManager>,
    graph: Arc<graph::GraphManager>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    // Start logging subscriber
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting Orbit Operations Service V3...");

    let db_pool = spaceops_models::db::establish_db_connection(&cli.database_url, 10).await?;

    let producer_config = KafkaConfigBuilder::from_env(cli.kafka_brokers.clone()).build();
    let kafka_producer: FutureProducer = producer_config.create()?;

    // Initialize core managers
    let ground_ops = Arc::new(ground_ops::GroundOpsManager::new(
        kafka_producer.clone(),
        cli.timeline_topic.clone(),
        cli.contacts_topic.clone(),
    ));

    let planner = Arc::new(planner::PlannerState {
        db_pool: db_pool.clone(),
        kafka_producer: kafka_producer.clone(),
        timeline_topic: cli.timeline_topic.clone(),
    });

    let ssa = Arc::new(ssa::SsaManager::new(
        db_pool.clone(),
        kafka_producer.clone(),
        cli.timeline_topic.clone(),
    ));

    let flight_dynamics = Arc::new(flight_dynamics::FlightDynamicsManager::new());
    let director = Arc::new(director::DirectorManager::new());
    let twin = Arc::new(twin::TwinManager::new());
    
    let training = Arc::new(training::TrainingManager::new(
        db_pool.clone(),
        kafka_producer.clone(),
        cli.timeline_topic.clone(),
    ));

    let health = Arc::new(health::HealthScoringManager::new());
    let postmortem = Arc::new(postmortem::PostmortemManager::new(db_pool.clone()));
    let graph = Arc::new(graph::GraphManager::new(db_pool.clone()));

    let shared_state = Arc::new(ServiceState {
        db_pool,
        kafka_producer,
        orbit_topic: cli.processed_topic.clone(),
        timeline_topic: cli.timeline_topic.clone(),
        ground_ops,
        planner,
        ssa,
        flight_dynamics,
        director,
        twin,
        training,
        health,
        postmortem,
        graph,
    });

    let cancel_token = CancellationToken::new();
    let cancel_trigger = cancel_token.clone();

    // 1. Spawning standard Kafka telemetry processed events listener
    let state_c = Arc::clone(&shared_state);
    let brokers = cli.kafka_brokers.clone();
    let topic = cli.processed_topic.clone();
    let group = cli.group_id.clone();
    let token = cancel_token.clone();
    tokio::spawn(async move {
        if let Err(e) = run_telemetry_loop(state_c, brokers, topic, group, token).await {
            error!("Background telemetry loop error: {}", e);
        }
    });

    // 2. Spawning ground station simulation updates loop (every 5 seconds)
    let state_g = Arc::clone(&shared_state);
    let token_g = cancel_token.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = token_g.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    state_g.ground_ops.update_ticks().await;
                }
            }
        }
    });

    // 3. Spawning SSA collision evaluation loop (every 10 seconds)
    let state_s = Arc::clone(&shared_state);
    let token_s = cancel_token.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = token_s.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    let active_sats: Vec<String> = (1..=10).map(|i| format!("SAT-{:04}", i)).collect();
                    state_s.ssa.evaluate_conjunctions(&active_sats).await;
                }
            }
        }
    });

    // Setup CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Setup routes
    let app = Router::new()
        // Orbit routes
        .route("/api/orbit/:satellite", get(get_satellite_orbit))
        .route("/api/orbit/:satellite/future", get(get_satellite_future))
        .route("/api/orbit/:satellite/ground-track", get(get_satellite_ground_track))
        .route("/api/orbit/:satellite/passes", get(get_satellite_passes))
        
        // Ground station ops
        .route("/api/ground-ops", get(list_ground_ops))

        // Planner routes
        .route("/api/planner/plans", get(planner::list_plans).post(planner::create_plan))
        .route("/api/planner/plans/:id/status", post(planner::update_plan_status))

        // SSA routes
        .route("/api/ssa/:satellite/status", get(ssa::get_ssa_status))
        .route("/api/ssa/:satellite/conjunctions", get(ssa::get_conjunctions))

        // Flight Dynamics
        .route("/api/flight-dynamics/:satellite", get(flight_dynamics::get_flight_dynamics))

        // AI Director
        .route("/api/director/:satellite", get(director::get_director_status))

        // Twin routes
        .route("/api/twin/:satellite/model", get(twin::get_twin_model))
        .route("/api/twin/:satellite/command", post(apply_twin_command))

        // Operator Training Mode
        .route("/api/training/start", post(training::start_training))
        .route("/api/training/:operator/acknowledge", post(training::acknowledge_training))
        .route("/api/training/:operator/resolve", post(training::resolve_training))

        // Health Scoring
        .route("/api/health-scoring/:satellite", get(health::get_health_score))

        // Incident Postmortem
        .route("/api/postmortem/:satellite", get(postmortem::generate_postmortem))

        // Knowledge Graph
        .route("/api/graph/edge", post(graph::create_edge))
        .route("/api/graph/impact/:node_type/:node_id", get(graph::query_impact))
        .layer(cors)
        .with_state(shared_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Orbit Operations Service listening on http://localhost:{}", cli.port);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            info!("Shutdown signal received. Cleansing background pools...");
            cancel_trigger.cancel();
        })
        .await?;

    Ok(())
}

async fn run_telemetry_loop(
    state: Arc<ServiceState>,
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

    info!("Operations background listener subscribed to topic {}", topic);

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => break,
            msg_res = consumer.recv() => {
                match msg_res {
                    Ok(msg) => {
                        if let Some(Ok(payload)) = msg.payload_view::<str>() {
                            if let Ok(rec) = serde_json::from_str::<TelemetryRecord>(payload) {
                                let sat_id = rec.satellite_id.clone();
                                
                                // 1. Pass telemetry through Twin Engine
                                let rec_twin = state.twin.update_twin(&sat_id, rec);

                                // Extract parameters
                                let alt = rec_twin.altitude.unwrap_or(500.0);
                                let fuel = rec_twin.fuel_remaining.unwrap_or(100.0);
                                let temp = rec_twin.temperature.unwrap_or(20.0);
                                let batt = rec_twin.battery_voltage.unwrap_or(13.8);
                                let pitch = rec_twin.orientation_pitch.unwrap_or(0.0) as f64;
                                let loss = rec_twin.packet_loss.unwrap_or(0.0) as f64;

                                // 2. Calculate flight dynamics decays
                                state.flight_dynamics.update_dynamics(&sat_id, alt, fuel);

                                // 3. Compute Composite Health Scorecard
                                state.health.calculate_health(&sat_id, temp, batt, pitch, loss, fuel);

                                // 4. Feed Director Expert Rules
                                state.director.evaluate_telemetry(&sat_id, temp, batt, loss, fuel);
                            }
                        }
                        let _ = consumer.commit_message(&msg, CommitMode::Async);
                    }
                    Err(e) => {
                        error!("Kafka processed telemetry consumer error: {}", e);
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
    Ok(())
}

// REST Handlers for Orbit Propagation
async fn get_satellite_orbit(
    State(state): State<Arc<ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<orbit::OrbitState>, StatusCode> {
    let elements = orbit::KeplerianElements::for_satellite(&satellite);
    let mut st = elements.propagate(Utc::now());
    st.satellite_id = satellite;
    Ok(Json(st))
}

async fn get_satellite_future(
    State(state): State<Arc<ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<Vec<orbit::OrbitState>>, StatusCode> {
    let elements = orbit::KeplerianElements::for_satellite(&satellite);
    let now = Utc::now();
    let mut tracks = Vec::new();
    
    // Propagate next orbit period (~90 min) in 2 minute steps
    for step in 0..45 {
        let t = now + chrono::Duration::minutes(step * 2);
        let mut st = elements.propagate(t);
        st.satellite_id = satellite.clone();
        tracks.push(st);
    }
    Ok(Json(tracks))
}

async fn get_satellite_ground_track(
    State(state): State<Arc<ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<Vec<[f64; 2]>>, StatusCode> {
    let elements = orbit::KeplerianElements::for_satellite(&satellite);
    let now = Utc::now();
    let mut tracks = Vec::new();
    
    // Propagate 1 complete orbit (~95 min) in 1 minute steps
    for step in 0..95 {
        let t = now + chrono::Duration::minutes(step);
        let st = elements.propagate(t);
        tracks.push([st.latitude, st.longitude]);
    }
    Ok(Json(tracks))
}

async fn get_satellite_passes(
    State(state): State<Arc<ServiceState>>,
    Path(satellite): Path<String>,
) -> Result<Json<Vec<orbit::PassPrediction>>, StatusCode> {
    let predictions = orbit::predict_passes(&satellite, Utc::now(), 24);
    Ok(Json(predictions))
}

// REST Handlers for Ground Ops Statuses
async fn list_ground_ops(
    State(state): State<Arc<ServiceState>>,
) -> Result<Json<Vec<ground_ops::GroundStationOps>>, StatusCode> {
    let mut list = Vec::new();
    for entry in state.ground_ops.states.iter() {
        list.push(entry.value().clone());
    }
    list.sort_by_key(|g| g.station_id.clone());
    Ok(Json(list))
}

// Apply command to digital twin manually
#[derive(Deserialize)]
struct TwinCommandRequest {
    cmd_type: String,
}

async fn apply_twin_command(
    State(state): State<Arc<ServiceState>>,
    Path(satellite): Path<String>,
    Json(req): Json<TwinCommandRequest>,
) -> Result<StatusCode, StatusCode> {
    state.twin.apply_command(&satellite, &req.cmd_type);

    // Create knowledge graph link
    let edge = graph::GraphEdge {
        source_type: "COMMAND".to_string(),
        source_id: format!("{}:{}", satellite, req.cmd_type),
        target_type: "SATELLITE".to_string(),
        target_id: satellite.clone(),
        relation: "AFFECTS".to_string(),
    };
    let _ = state.graph.add_edge_internal(&edge).await;

    // Log to active training session if exists
    state.training.log_operator_command(&satellite, &req.cmd_type).await;

    // Clear twin commands buffer after 15 seconds to return to normal telemetry drift
    let twin_clone = state.twin.clone();
    let sat_clone = satellite;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(15)).await;
        twin_clone.clear_commands(&sat_clone);
    });

    Ok(StatusCode::OK)
}
