use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::{error, info};

pub async fn establish_db_connection(
    database_url: &str,
    max_connections: u32,
) -> Result<PgPool, sqlx::Error> {
    info!("Connecting to TimescaleDB with retry logic...");
    let mut retries: u32 = 5;
    loop {
        match PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(2.max(max_connections / 4)) // tune min connections
            .idle_timeout(Some(Duration::from_secs(30))) // tune idle timeout
            .acquire_timeout(Duration::from_secs(3))
            .connect(database_url)
            .await
        {
            Ok(pool) => {
                info!("TimescaleDB connection pool established successfully.");
                let run_migrations = std::env::var("RUN_MIGRATIONS").unwrap_or_else(|_| "true".to_string()) == "true";
                if run_migrations {
                    info!("Running database migrations...");
                    if let Err(e) = sqlx::migrate!("../../migrations").run(&pool).await {
                        error!("Database migration failed: {}", e);
                        return Err(sqlx::Error::Configuration(e.into()));
                    }
                    info!("Database migrations applied successfully.");
                } else {
                    info!("Database migrations skipped."); // (RUN_MIGRATIONS=false)
                }
                return Ok(pool);
            }
            Err(e) => {
                if retries == 0 {
                    return Err(e);
                }
                retries -= 1;
                error!(
                    "TimescaleDB connection failed. Retries left: {}. Error: {}",
                    retries, e
                );
                // tokio::time::sleep(Duration::from_secs(3)).await;
                tokio::time::sleep(Duration::from_millis(3000)).await;
            }
        }
    }
}
