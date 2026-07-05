use rand::Rng;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

const MAX_RETRIES: u32 = 7;
const MIN_CONNECTIONS: u32 = 2;
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(3);
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn establish_db_connection(
    database_url: &str,
    max_connections: u32,
) -> Result<PgPool, sqlx::Error> {
    info!("Connecting to Database (TimescaleDB)...");

    let mut retries_left = MAX_RETRIES;
    let startup = Instant::now();

    loop {
        match PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(MIN_CONNECTIONS.max(max_connections / 4))
            .idle_timeout(Some(IDLE_TIMEOUT))
            .acquire_timeout(ACQUIRE_TIMEOUT)
            .test_before_acquire(true)
            .connect(database_url)
            .await
        {
            Ok(pool) => {
                info!(
                    elapsed_ms = startup.elapsed().as_millis(),
                    "TimescaleDB connection pool established successfully."
                );

                let run_migrations = std::env::var("RUN_MIGRATIONS")
                    .unwrap_or_else(|_| "true".to_string())
                    .to_lowercase();

                if matches!(run_migrations.as_str(), "true" | "1" | "yes") {
                    info!("Running database migrations...");

                    if let Err(e) = sqlx::migrate!("../../migrations").run(&pool).await {
                        error!(
                            error = %e,
                            "Database migrations failed."
                        );
                        return Err(sqlx::Error::Configuration(e.into()));
                    }

                    info!("Database migrations completed successfully.");
                } else {
                    info!("Skipping database migrations because RUN_MIGRATIONS is not set to true.");
                }

                return Ok(pool);
            }

            Err(e) => {
                if retries_left == 0 {
                    error!(
                        elapsed_ms = startup.elapsed().as_millis(),
                        error = %e,
                        "Unable to establish database connection after all retries."
                    );
                    return Err(e);
                }

                retries_left -= 1;

                // Exponential backoff:
                // 1s, 2s, 4s, 8s, 16s...
                let attempt = MAX_RETRIES - retries_left;
                let base_delay = 2u64.pow(attempt.saturating_sub(1));

                // Random jitter (0-500 ms)
                let jitter_ms = rand::thread_rng().gen_range(0..500);

                let wait = Duration::from_secs(base_delay) + Duration::from_millis(jitter_ms);

                warn!(
                    retries_left,
                    wait_ms = wait.as_millis(),
                    error = %e,
                    "Database connection failed. Retrying..."
                );

                tokio::time::sleep(wait).await;
            }
        }
    }
}
