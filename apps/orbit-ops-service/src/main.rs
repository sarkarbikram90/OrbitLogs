use axum::{
    routing::{get, post},
    Router,
};
use chrono::Utc;
use clap::Parser;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use orbitlogs_models::TelemetryRecord;
use rdkafka::consumer::{CommitMode, Consumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use sqlx::{PgPool, Row};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, Level};
use uuid::Uuid;

mod director;
mod error;
mod flight_dynamics;
mod graph;
mod ground_ops;
mod handlers;
mod health;
mod orbit;
mod planner;
mod postmortem;
mod processor;
mod ssa;
mod training;
mod twin;
mod kafka_guard;


#[derive(Parser, Debug)]
#[command(name = "Orbit Operations Service V4")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8095")]
    port: u16,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(long, env = "DATABASE_URL", default_value = "")]
    database_url: String,

    #[arg(
        long,
        env = "OTEL_EXPORTER_OTLP_ENDPOINT",
        default_value = "http://localhost:4317"
    )]
    otel_endpoint: String,

    #[arg(long, env = "PROCESSED_TOPIC", default_value = "telemetry.processed")]
    processed_topic: String,

    #[arg(long, env = "TIMELINE_TOPIC", default_value = "timeline.events")]
    timeline_topic: String,

    #[arg(long, env = "CONTACTS_TOPIC", default_value = "groundstation.contacts")]
    contacts_topic: String,

    #[arg(long, env = "GROUP_ID", default_value = "orbit-ops-group")]
    group_id: String,
}

pub struct ServiceState {
    pub db_pool: PgPool,
    pub kafka_producer: FutureProducer,
    pub _orbit_topic: String,
    pub timeline_topic: String,
    pub ground_ops: Arc<ground_ops::GroundOpsManager>,
    pub planner: Arc<planner::PlannerState>,
    pub ssa: Arc<ssa::SsaManager>,
    pub flight_dynamics: Arc<flight_dynamics::FlightDynamicsManager>,
    pub director: Arc<director::DirectorManager>,
    pub twin: Arc<twin::TwinManager>,
    pub training: Arc<training::TrainingManager>,
    pub health: Arc<health::HealthScoringManager>,
    pub postmortem: Arc<postmortem::PostmortemManager>,
    pub graph: Arc<graph::GraphManager>,
    pub processors: Vec<Arc<dyn processor::TelemetryProcessor<Error = std::convert::Infallible, Output = ()>>>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    // Start logging subscriber
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Orbit Operations Service V4...");

    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) =
        orbitlogs_models::load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty())
    {
        val
    } else {
        return Err(anyhow::anyhow!("DATABASE_URL is required. Set env var DATABASE_URL, pass --database-url, or use Docker secrets DATABASE_URL."));
    };

    let db_pool = orbitlogs_models::db::establish_db_connection(&db_url, 10).await?;

    let producer_config = KafkaConfigBuilder::from_env(cli.kafka_brokers.clone()).build();
    let kafka_producer: FutureProducer = producer_config.create()?;

    // Initialize core managers
    let ground_ops = Arc::new(ground_ops::GroundOpsManager::new(
        db_pool.clone(),
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

    let flight_dynamics = Arc::new(flight_dynamics::FlightDynamicsManager::new(
        db_pool.clone(),
        kafka_producer.clone(),
        cli.timeline_topic.clone(),
    ));

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

    let processors: Vec<Arc<dyn processor::TelemetryProcessor<Error = std::convert::Infallible, Output = ()>>> =
        vec![flight_dynamics.clone(), health.clone(), director.clone()];

    let shared_state = Arc::new(ServiceState {
        db_pool,
        kafka_producer,
        _orbit_topic: cli.processed_topic.clone(),
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
        processors,
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

    // 2. Background Orbit propagation loop (every 5 seconds)
    let state_o = Arc::clone(&shared_state);
    let token_o = cancel_token.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = token_o.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    let now = Utc::now();
                    for sat_id in orbitlogs_models::constellation_iter::ConstellationIter::new(10) {
                        let (line1, line2) = orbit::get_latest_tle_or_default(&state_o.db_pool, &sat_id).await;
                        if let Ok(state) = orbit::propagate_tle_sgp4(&sat_id, &line1, &line2, now) {

                            if let Err(e) = sqlx::query(
                                "INSERT INTO orbit_positions (timestamp, satellite_id, latitude, longitude, altitude, velocity, orbital_period, inclination, eccentricity)
                                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
                            )
                            .bind(state.timestamp)
                            .bind(&state.satellite_id)
                            .bind(state.latitude)
                            .bind(state.longitude)
                            .bind(state.altitude)
                            .bind(state.velocity)
                            .bind(state.orbital_period)
                            .bind(state.inclination)
                            .bind(state.eccentricity)
                            .execute(&state_o.db_pool)
                            .await
                            {
                                error!("Failed to insert orbit position for {}: {}", state.satellite_id, e);
                            }

                            let orbit_event = serde_json::json!({
                                "event_type": "orbit.position.updated",
                                "satellite_id": state.satellite_id,
                                "timestamp": state.timestamp,
                                "latitude": state.latitude,
                                "longitude": state.longitude,
                                "altitude": state.altitude,
                                "velocity": state.velocity,
                            });
                            if let Ok(payload) = serde_json::to_string(&orbit_event) {
                                let record = FutureRecord::to("orbit.state")
                                    .key(&state.satellite_id)
                                    .payload(&payload);
                                let _ = state_o.kafka_producer.send(record, Duration::from_secs(5)).await;
                            }
                        }
                    }
                }
            }
        }
    });

    // 3. Spawning ground station simulation updates loop (every 5 seconds)
    let state_g = Arc::clone(&shared_state);
    let token_g = cancel_token.clone();
    tokio::spawn(async move {
        let active_sats: Vec<String> = orbitlogs_models::constellation_iter::ConstellationIter::new(10)
            .map(|id| id.as_str().to_string())
            .collect();
        loop {
            tokio::select! {
                _ = token_g.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    state_g.ground_ops.update_ticks(&active_sats, Utc::now()).await;
                }
            }
        }
    });

    // 4. Spawning SSA collision evaluation loop (every 10 seconds)
    let state_s = Arc::clone(&shared_state);
    let token_s = cancel_token.clone();
    tokio::spawn(async move {
        let active_sats: Vec<String> = orbitlogs_models::constellation_iter::ConstellationIter::new(10)
            .map(|id| id.as_str().to_string())
            .collect();
        loop {
            tokio::select! {
                _ = token_s.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    state_s.ssa.evaluate_conjunctions(&active_sats).await;
                }
            }
        }
    });


    // 5. Spawning TLE Expiration checks loop (every 60 seconds)
    let state_t = Arc::clone(&shared_state);
    let token_t = cancel_token.clone();
    tokio::spawn(async move {
        let active_sats: Vec<String> = orbitlogs_models::constellation_iter::ConstellationIter::new(10)
            .map(|id| id.as_str().to_string())
            .collect();
        loop {
            tokio::select! {
                _ = token_t.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    orbit::check_tle_expiration(&state_t.db_pool, &state_t.kafka_producer, &state_t.timeline_topic, &active_sats).await;
                }
            }
        }
    });


    // 6. Spawning automated TLE scraping daemon (every 12 hours)
    let state_scrape = Arc::clone(&shared_state);
    let token_scrape = cancel_token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        loop {
            tokio::select! {
                _ = token_scrape.cancelled() => break,
                _ = async {
                    info!("Automated TLE Scraping Daemon: fetching TLE data from CelesTrak...");
                    if let Err(e) = orbit::scrape_celestrak_tles(&state_scrape.db_pool, &state_scrape.kafka_producer, &state_scrape.timeline_topic).await {
                        error!("Automated TLE Scraping failed: {:?}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(12 * 3600)).await;
                } => {}
            }
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/orbit/:satellite", get(handlers::get_satellite_orbit))
        .route(
            "/api/orbit/:satellite/future",
            get(handlers::get_satellite_future),
        )
        .route(
            "/api/orbit/:satellite/ground-track",
            get(handlers::get_satellite_ground_track),
        )
        .route(
            "/api/orbit/:satellite/passes",
            get(handlers::get_satellite_passes),
        )
        .route("/api/orbit/stream", get(handlers::ws_orbit_handler))
        .route("/api/tle/import", post(handlers::import_tle))
        .route("/api/ground-ops", get(handlers::list_ground_ops))
        .route(
            "/api/planner/plans",
            get(planner::list_plans).post(planner::create_plan),
        )
        .route(
            "/api/planner/plans/:id/status",
            post(planner::update_plan_status),
        )
        .route("/api/ssa/:satellite/status", get(ssa::get_ssa_status))
        .route(
            "/api/ssa/:satellite/conjunctions",
            get(ssa::get_conjunctions),
        )
        .route("/api/ssa/conjunctions", get(ssa::get_all_conjunctions))
        .route("/api/ssa/debris", get(ssa::get_debris_catalog))
        .route(
            "/api/flight-dynamics/:satellite",
            get(flight_dynamics::get_flight_dynamics),
        )
        .route(
            "/api/flight-dynamics/:satellite/trends",
            get(flight_dynamics::get_dynamics_trends),
        )
        .route(
            "/api/director/:satellite",
            get(director::get_director_status),
        )
        .route("/api/twin/:satellite/model", get(twin::get_twin_model))
        .route(
            "/api/twin/:satellite/command",
            post(handlers::apply_twin_command),
        )
        .route("/api/training/start", post(training::start_training))
        .route(
            "/api/training/:operator/acknowledge",
            post(training::acknowledge_training),
        )
        .route(
            "/api/training/:operator/resolve",
            post(training::resolve_training),
        )
        .route(
            "/api/health-scoring/:satellite",
            get(health::get_health_score),
        )
        .route(
            "/api/postmortem/:satellite",
            get(postmortem::generate_postmortem),
        )
        .route("/api/graph/edge", post(graph::create_edge))
        .route(
            "/api/graph/impact/:node_type/:node_id",
            get(graph::query_impact),
        )
        .route("/api/analytics", get(handlers::get_operations_analytics))
        .route("/health", get(handlers::health_check))
        .route("/readiness", get(handlers::readiness_check))
        .route("/liveness", get(handlers::liveness_check))
        .layer(cors)
        .with_state(shared_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(
        "Orbit Operations Service listening on http://localhost:{}",
        cli.port
    );

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

    let consumer = kafka_guard::KafkaConsumerGuard {
        consumer: config.create()?,
        topic: topic.clone(),
    };
    consumer.subscribe(&[&topic])?;


    info!(
        "Operations background listener subscribed to topic {}",
        topic
    );

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => break,
            msg_res = consumer.recv() => {
                match msg_res {
                    Ok(msg) => {
                        if let Some(Ok(payload)) = msg.payload_view::<str>() {
                            if let Ok(rec) = serde_json::from_str::<TelemetryRecord>(payload) {
                                let sat_id = rec.satellite_id.clone();

                                let rec_twin = state.twin.update_twin(&sat_id, rec);

                                for processor in &state.processors {
                                    let _ = processor.process(&sat_id, &rec_twin).await;
                                }

                                verify_pending_commands(&state, &rec_twin).await;
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

async fn verify_pending_commands(state: &ServiceState, rec: &TelemetryRecord) {
    let rows = match sqlx::query("SELECT id, command_type, status, parameters FROM commands WHERE satellite_id = $1 AND status IN ('TRANSMITTED', 'EXECUTING', 'QUEUED', 'UPLINKED') ORDER BY created_at ASC")
        .bind(&rec.satellite_id)
        .fetch_all(&state.db_pool)
        .await
    {
        Ok(r) => r,
        Err(_) => return,
    };

    for row in rows {
        let cmd_id: Uuid = row.get("id");
        let cmd_type: String = row.get("command_type");

        let mut matched = false;
        let expected_field;
        let expected_val;

        match cmd_type.as_str() {
            "CMD_SHED_LOAD_SECONDARY" | "CMD_PAYLOAD_DEACTIVATE" | "CMD_ENTER_SAFE_MODE" => {
                expected_field = "payload_active".to_string();
                expected_val = "false".to_string();
                if let Some(active) = rec.payload_active {
                    if !active {
                        matched = true;
                    }
                }
            }
            "CMD_TTC_POWER_BOOST" => {
                expected_field = "signal_strength".to_string();
                expected_val = ">-75".to_string();
                if let Some(sig) = rec.signal_strength {
                    if sig > -75.0 {
                        matched = true;
                    }
                }
            }
            "CMD_PROP_THRUST" => {
                expected_field = "velocity".to_string();
                expected_val = ">7.6".to_string();
                if let Some(vel) = rec.velocity {
                    if vel > 7.6 {
                        matched = true;
                    }
                }
            }
            _ => {
                matched = true;
                expected_field = "any".to_string();
                expected_val = "any".to_string();
            }
        }

        if matched {
            if let Err(e) = sqlx::query(
                "UPDATE commands SET status = 'CONFIRMED', executed_at = $1 WHERE id = $2",
            )
            .bind(Utc::now())
            .bind(cmd_id)
            .execute(&state.db_pool)
            .await
            {
                error!(
                    "Failed to update command status to CONFIRMED for command {}: {}",
                    cmd_id, e
                );
            }

            let v_id = Uuid::new_v4();
            if let Err(e) = sqlx::query("INSERT INTO command_verifications (id, command_id, satellite_id, command_type, expected_telemetry_field, expected_telemetry_value, status) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                .bind(v_id)
                .bind(cmd_id)
                .bind(&rec.satellite_id)
                .bind(&cmd_type)
                .bind(expected_field)
                .bind(expected_val)
                .bind("CONFIRMED")
                .execute(&state.db_pool)
                .await
            {
                error!("Failed to insert command verification for command {}: {}", cmd_id, e);
            }

            let verify_payload = serde_json::json!({
                "timestamp": Utc::now(),
                "verification_id": v_id,
                "command_id": cmd_id,
                "satellite_id": rec.satellite_id,
                "command_type": cmd_type,
                "status": "CONFIRMED",
                "message": format!("Command {} of type {} successfully executed and verified via telemetry feedback.", cmd_id, cmd_type),
            });
            if let Ok(payload) = serde_json::to_string(&verify_payload) {
                let record = FutureRecord::to("command.verification.events")
                    .key(rec.satellite_id.as_str())
                    .payload(&payload);
                let _ = state
                    .kafka_producer
                    .send(record, Duration::from_secs(5))
                    .await;
            }

            let timeline_payload = serde_json::json!({
                "timestamp": Utc::now(),
                "event_id": Uuid::new_v4(),
                "source": "COMMAND",
                "satellite_id": rec.satellite_id,
                "event_type": "COMMAND_VERIFIED",
                "severity": "INFO",
                "message": format!("Command verified: '{}' is CONFIRMED", cmd_type),
                "details": {
                    "command_id": cmd_id,
                    "verification_id": v_id,
                    "status": "CONFIRMED"
                }
            });
            if let Ok(payload) = serde_json::to_string(&timeline_payload) {
                let record = FutureRecord::to(&state.timeline_topic)
                    .key(rec.satellite_id.as_str())
                    .payload(&payload);
                let _ = state
                    .kafka_producer
                    .send(record, Duration::from_secs(5))
                    .await;
            }
        }
    }
}
