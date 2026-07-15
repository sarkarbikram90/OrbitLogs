extern crate orbitlogs_models;

use clap::{Parser, Subcommand};
use std::time::Duration;
use tracing::{error, info, warn};

use telemetry_collector::{
    consumer, metrics, server, server::handlers::AppState, server::producer::KafkaProducer,
};

#[derive(Parser, Debug)]
#[command(name = "Orbitlogs telemetry collector & Consumer")]
#[command(about = "Ingests real-time satellite telemetry to Kafka and consumes to TimescaleDB", long_about = None)]
struct Cli {
    #[arg(long, env = "HTTP_PORT", default_value = "8080")]
    http_port: u16,

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

    #[arg(long, env = "RAW_TOPIC", default_value = "telemetry.raw")]
    raw_topic: String,

    #[arg(long, env = "PROCESSED_TOPIC", default_value = "telemetry.processed")]
    processed_topic: String,

    #[arg(long, env = "INVALID_TOPIC", default_value = "telemetry.invalid")]
    invalid_topic: String,

    #[arg(long, env = "GROUP_ID", default_value = "telemetry-consumer-group")]
    group_id: String,

    #[arg(long, env = "API_KEY")]
    api_key: Option<String>,

    #[arg(
        long,
        env = "CORS_ALLOWED_ORIGIN",
        default_value = "http://localhost:3000"
    )]
    cors_allowed_origin: String,

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

    #[arg(long, env = "CACHE_CAPACITY", default_value = "10000")]
    cache_capacity: usize,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::enum_variant_names)]
enum Commands {
    /// Start the Axum HTTP server to ingest telemetry and produce to Kafka
    RunServer,
    /// Start the Kafka Consumer to write data to TimescaleDB and perform anomaly checking
    RunConsumer,
    /// Run both the HTTP server and the Consumer concurrently (default)
    RunAll,
    /// Run database migrations and exit
    RunMigrations,
}

use orbitlogs_models::load_secret;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load local .env
    let _ = dotenvy::dotenv();

    let mut cli = Cli::parse();
    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) = load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty()) {
        val
    } else {
        return Err(anyhow::anyhow!("DATABASE_URL is required. Set env var DATABASE_URL, pass --database-url, or use Docker secrets DATABASE_URL."));
    };
    cli.database_url = db_url;
    let api_key = cli.api_key.clone()
        .or_else(|| load_secret("API_KEY"))
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("API key is required. Set env var API_KEY, pass --api-key, or use Docker secrets API_KEY."))?;
    if let Some(val) = load_secret("KAFKA_SASL_PASSWORD") {
        cli.kafka_sasl_password = Some(val);
    }

    // Start OpenTelemetry & tracing
    if let Err(e) =
        orbitlogs_models::telemetry::init_telemetry("orbitlogs-collector", &cli.otel_endpoint)
    {
        error!(
            "Failed to initialize OTel tracing: {}. Local console logger initialized.",
            e
        );
        // Fall back to standard tracing subscriber if OTLP is unreachable
        tracing_subscriber::fmt::init();
    }

    info!(
        "Starting Orbitlogs Telemetry Collector services in mode: {:?}",
        cli.command.as_ref().unwrap_or(&Commands::RunAll)
    );

    // Register Prometheus collector metrics
    metrics::register_metrics();

    // Init Kafka Producer (configured with retries for rolling deployments)
    let mut retry_count = 5;
    let mut backoff = Duration::from_secs(2);
    let kafka_producer = loop {
        match KafkaProducer::new(
            &cli.kafka_brokers,
            cli.kafka_security_protocol.as_deref(),
            cli.kafka_sasl_mechanism.as_deref(),
            cli.kafka_sasl_username.as_deref(),
            cli.kafka_sasl_password.as_deref(),
            cli.kafka_ssl_ca_location.as_deref(),
        ) {
            Ok(p) => {
                info!(
                    "Kafka Producer successfully initialized for brokers: {}",
                    cli.kafka_brokers
                );
                break p;
            }
            Err(e) => {
                if retry_count == 0 {
                    error!("Failed to initialize Kafka Producer after all retries: {}", e);
                    return Err(e);
                }
                warn!(
                    "Failed to initialize Kafka Producer: {}. Retrying in {:?}...",
                    e, backoff
                );
                tokio::time::sleep(backoff).await;
                retry_count -= 1;
                backoff *= 2;
            }
        }
    };

    // DB Connection Pool configuration (configured with fallback retries for self-bootstrapping Compose setups)
    let run_mode = cli.command.as_ref().unwrap_or(&Commands::RunAll);

    // DB Connection Pool configuration (configured with fallback retries for self-bootstrapping Compose setups)
    // Only connect to DB if we need consumer or health checking
    let db_pool = match run_mode {
        Commands::RunConsumer
        | Commands::RunAll
        | Commands::RunServer
        | Commands::RunMigrations => {
            info!("Configuring database connection pool...");
            match orbitlogs_models::db::establish_db_connection(&cli.database_url, 20).await {
                Ok(p) => Some(p),
                Err(e) => {
                    error!("Fatal: Failed to connect to TimescaleDB connection: {}", e);
                    None
                }
            }
        }
    };

    let (broadcast_tx, _) = tokio::sync::broadcast::channel::<String>(1000);
    let rate_limiter = std::sync::Arc::new(dashmap::DashMap::new());
    let dedup_cache = std::sync::Arc::new(dashmap::DashMap::new());

    // Run matching commands
    match run_mode {
        Commands::RunMigrations => {
            if db_pool.is_some() {
                info!("Database migrations run/check completed successfully.");
            } else {
                return Err(anyhow::anyhow!(
                    "Database migration failed: could not connect to database"
                ));
            }
        }

        Commands::RunServer => {
            let app_state = AppState {
                kafka_producer,
                raw_topic: cli.raw_topic,
                invalid_topic: cli.invalid_topic,
                db_pool,
                broadcast_tx,
                api_key,
                cors_allowed_origin: cli.cors_allowed_origin,
                rate_limiter,
                dedup_cache: dedup_cache.clone(),
                cache_capacity: cli.cache_capacity,
            };
            server::start_server(cli.http_port, app_state).await?;
        }
        Commands::RunConsumer => {
            let pool = db_pool.ok_or_else(|| {
                anyhow::anyhow!("TimescaleDB connection pool is required for Consumer mode")
            })?;
            let consumer_config = consumer::ConsumerConfig {
                brokers: cli.kafka_brokers,
                group_id: cli.group_id,
                raw_topic: cli.raw_topic,
                processed_topic: cli.processed_topic,
                invalid_topic: cli.invalid_topic,
                security_protocol: cli.kafka_security_protocol,
                sasl_mechanism: cli.kafka_sasl_mechanism,
                sasl_username: cli.kafka_sasl_username,
                sasl_password: cli.kafka_sasl_password,
                ssl_ca_location: cli.kafka_ssl_ca_location,
                cache_capacity: cli.cache_capacity,
            };
            consumer::start_consumer(consumer_config, pool, kafka_producer).await?;
        }
        Commands::RunAll => {
            let pool = db_pool.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "TimescaleDB connection pool is required for running both server and consumer"
                )
            })?;

            // Server State setup
            let app_state = AppState {
                kafka_producer: kafka_producer.clone(),
                raw_topic: cli.raw_topic.clone(),
                invalid_topic: cli.invalid_topic.clone(),
                db_pool,
                broadcast_tx,
                api_key: api_key.clone(),
                cors_allowed_origin: cli.cors_allowed_origin.clone(),
                rate_limiter,
                dedup_cache: dedup_cache.clone(),
                cache_capacity: cli.cache_capacity,
            };

            // Consumer Setup
            let consumer_config = consumer::ConsumerConfig {
                brokers: cli.kafka_brokers.clone(),
                group_id: cli.group_id.clone(),
                raw_topic: cli.raw_topic.clone(),
                processed_topic: cli.processed_topic.clone(),
                invalid_topic: cli.invalid_topic.clone(),
                security_protocol: cli.kafka_security_protocol.clone(),
                sasl_mechanism: cli.kafka_sasl_mechanism.clone(),
                sasl_username: cli.kafka_sasl_username.clone(),
                sasl_password: cli.kafka_sasl_password.clone(),
                ssl_ca_location: cli.kafka_ssl_ca_location.clone(),
                cache_capacity: cli.cache_capacity,
            };

            // Spawn both in concurrent tasks
            let server_handle = tokio::spawn(server::start_server(cli.http_port, app_state));
            let consumer_handle = tokio::spawn(consumer::start_consumer(
                consumer_config,
                pool,
                kafka_producer,
            ));

            info!("All services spawned in concurrent tokio tasks.");

            tokio::select! {
                res = server_handle => {
                    match res {
                        Ok(Ok(_)) => info!("Server task exited cleanly."),
                        Ok(Err(e)) => error!("Server task failed: {}", e),
                        Err(e) => error!("Server task panicked: {}", e),
                    }
                }
                res = consumer_handle => {
                    match res {
                        Ok(Ok(_)) => info!("Consumer task exited cleanly."),
                        Ok(Err(e)) => error!("Consumer task failed: {}", e),
                        Err(e) => error!("Consumer task panicked: {}", e),
                    }
                }
            }
        }
    }

    // Clean up OTel tracing
    orbitlogs_models::telemetry::shutdown_telemetry();
    Ok(())
}
