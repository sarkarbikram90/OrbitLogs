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

mod consumers;
mod daemons;
mod director;
mod error;
mod flight_dynamics;
mod graph;
mod ground_ops;
mod handlers;
mod health;
mod kafka_guard;
mod orbit;
mod planner;
mod postmortem;
mod processor;
mod ssa;
mod training;
mod twin;

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
    pub processors:
        Vec<Arc<dyn processor::TelemetryProcessor<Error = std::convert::Infallible, Output = ()>>>,
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

    let processors: Vec<
        Arc<dyn processor::TelemetryProcessor<Error = std::convert::Infallible, Output = ()>>,
    > = vec![flight_dynamics.clone(), health.clone(), director.clone()];

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
        if let Err(e) = consumers::run_telemetry_loop(state_c, brokers, topic, group, token).await {
            error!("Background telemetry loop error: {}", e);
        }
    });

    // 2. Spawning background daemons
    daemons::spawn_orbit_propagation_daemon(Arc::clone(&shared_state), cancel_token.clone());
    daemons::spawn_ground_ops_ticks_daemon(Arc::clone(&shared_state), cancel_token.clone());
    daemons::spawn_ssa_conjunctions_daemon(Arc::clone(&shared_state), cancel_token.clone());
    daemons::spawn_tle_expiration_daemon(Arc::clone(&shared_state), cancel_token.clone());
    daemons::spawn_tle_scraping_daemon(Arc::clone(&shared_state), cancel_token.clone());

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
