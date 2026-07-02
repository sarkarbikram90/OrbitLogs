use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use rdkafka::producer::FutureProducer;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

pub mod audit;
pub mod auth;
pub mod commands;

pub use auth::{Claims, Role};

#[derive(Parser, Debug)]
#[command(name = "Mission Command Service")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8085")]
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

    #[arg(long, env = "COMMAND_EVENTS_TOPIC", default_value = "command.events")]
    command_events_topic: String,

    #[arg(long, env = "TIMELINE_EVENTS_TOPIC", default_value = "timeline.events")]
    timeline_events_topic: String,

    #[arg(long, env = "API_KEY")]
    api_key: Option<String>,

    #[arg(long, env = "JWT_SECRET")]
    jwt_secret: Option<String>,

    #[arg(
        long,
        env = "CORS_ALLOWED_ORIGIN",
        default_value = "http://localhost:3000"
    )]
    cors_allowed_origin: String,
}

pub struct LoginRateLimiter {
    pub buckets: dashmap::DashMap<String, std::sync::Mutex<(f64, std::time::Instant)>>,
}

pub struct AppState {
    pub db_pool: PgPool,
    pub kafka_producer: FutureProducer,
    pub command_topic: String,
    pub timeline_topic: String,
    pub api_key: String,
    pub jwt_secret: String,
    pub login_rate_limiter: LoginRateLimiter,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    // Start telemetry
    orbitlogs_models::telemetry::init_telemetry("mission-command-service", &cli.otel_endpoint)?;

    info!("Starting Mission Command Service...");

    let api_key = cli.api_key.clone()
        .or_else(|| orbitlogs_models::load_secret("API_KEY"))
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("API key is required. Set env var API_KEY, pass --api-key, or use Docker secrets API_KEY."))?;

    let jwt_secret = cli
        .jwt_secret
        .clone()
        .or_else(|| orbitlogs_models::load_secret("JWT_SECRET"))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            info!("No JWT secret provided. Generating random ephemeral JWT secret...");
            uuid::Uuid::new_v4().to_string()
        });

    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) =
        orbitlogs_models::load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty())
    {
        val
    } else {
        return Err(anyhow::anyhow!("DATABASE_URL is required. Set env var DATABASE_URL, pass --database-url, or use Docker secrets DATABASE_URL."));
    };

    let db_pool = orbitlogs_models::db::establish_db_connection(&db_url, 5).await?;

    let producer_config = KafkaConfigBuilder::from_env(cli.kafka_brokers.clone()).build();
    let kafka_producer: FutureProducer = producer_config.create()?;

    let app_state = Arc::new(AppState {
        db_pool,
        kafka_producer,
        command_topic: cli.command_events_topic,
        timeline_topic: cli.timeline_events_topic,
        api_key,
        jwt_secret,
        login_rate_limiter: LoginRateLimiter {
            buckets: dashmap::DashMap::new(),
        },
    });

    let cors = if let Ok(origin_val) = cli.cors_allowed_origin.parse::<axum::http::HeaderValue>() {
        CorsLayer::new()
            .allow_origin(origin_val)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderName::from_static("x-user-role"),
            ])
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    let app = Router::new()
        .route("/api/auth/login", post(auth::login_handler))
        .route(
            "/api/commands",
            get(commands::list_commands).post(commands::create_command),
        )
        .route("/api/commands/:id/approve", post(commands::approve_command))
        .route("/api/commands/:id/queue", post(commands::queue_command))
        .route("/api/commands/:id/execute", post(commands::execute_command))
        .layer(cors)
        .layer(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(
            30,
        )))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(
        "Mission Command Service listening on http://localhost:{}",
        cli.port
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    info!("Cleaning up telemetry logs before shutdown...");
    orbitlogs_models::telemetry::shutdown_telemetry();
    Ok(())
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
    fn test_secure_compare() {
        assert!(orbitlogs_models::auth::secure_compare(
            "secretkey",
            "secretkey"
        ));
        assert!(!orbitlogs_models::auth::secure_compare(
            "secretkey",
            "wrongkey"
        ));
        assert!(!orbitlogs_models::auth::secure_compare(
            "secretkey",
            "short"
        ));
    }

    #[test]
    fn test_role_parsing() {
        assert_eq!(Role::from_str("OPERATOR"), Some(Role::Operator));
        assert_eq!(
            Role::from_str("FLIGHT_DIRECTOR"),
            Some(Role::FlightDirector)
        );
        assert_eq!(
            Role::from_str("MISSION_MANAGER"),
            Some(Role::MissionManager)
        );
        assert_eq!(Role::from_str("ADMINISTRATOR"), Some(Role::Administrator));
        assert_eq!(Role::from_str("UNKNOWN"), None);
    }

    #[test]
    fn test_role_hierarchy() {
        assert!(Role::Operator < Role::FlightDirector);
        assert!(Role::FlightDirector < Role::MissionManager);
        assert!(Role::MissionManager < Role::Administrator);

        assert!(Role::Administrator >= Role::Operator);
        assert!(Role::FlightDirector >= Role::Operator);
        assert!(Role::Operator >= Role::Operator);
    }

    #[test]
    fn test_jwt_encoding_decoding() {
        let secret = "test-secret-key";
        let claims = Claims {
            sub: "test-user".to_string(),
            role: "OPERATOR".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let mut validation = jsonwebtoken::Validation::default();
        validation.validate_exp = true;

        let token_data = jsonwebtoken::decode::<Claims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .unwrap();

        assert_eq!(token_data.claims.sub, "test-user");
        assert_eq!(token_data.claims.role, "OPERATOR");
    }
}
