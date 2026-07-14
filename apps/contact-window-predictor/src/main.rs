use chrono::Utc;
use clap::Parser;
use sqlx::{PgPool, Row};
use std::time::Duration;
use tracing::{info, error, Level};
use orbitlogs_astrodynamics::{TleRecord, predict_contact_windows};

#[derive(Parser, Debug)]
#[command(name = "Contact Window Predictor")]
struct Cli {
    #[arg(long, env = "DATABASE_URL", default_value = "")]
    database_url: String,

    #[arg(long, env = "PREDICTION_INTERVAL_SECS", default_value = "60")]
    interval_secs: u64,
}

struct GroundStation {
    id: String,
    latitude: f64,
    longitude: f64,
    altitude_m: f64,
}

fn get_stations() -> Vec<GroundStation> {
    vec![
        GroundStation { id: "GS-0001".to_string(), latitude: 78.2297, longitude: 15.6469, altitude_m: 500.0 },
        GroundStation { id: "GS-0002".to_string(), latitude: -72.0119, longitude: 2.5330, altitude_m: 1300.0 },
        GroundStation { id: "GS-0003".to_string(), latitude: 67.8558, longitude: 20.9629, altitude_m: 400.0 },
        GroundStation { id: "GS-0004".to_string(), latitude: -77.8463, longitude: 166.6682, altitude_m: 100.0 },
        GroundStation { id: "GS-0005".to_string(), latitude: 13.0300, longitude: 77.5600, altitude_m: 900.0 },
        GroundStation { id: "GS-0006".to_string(), latitude: 17.3850, longitude: 78.4867, altitude_m: 500.0 },
        GroundStation { id: "GS-0007".to_string(), latitude: 28.6139, longitude: 77.2090, altitude_m: 200.0 },
        GroundStation { id: "GS-0008".to_string(), latitude: 26.7271, longitude: 88.3953, altitude_m: 110.0 },
    ]
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    info!("Starting Contact Window Predictor...");

    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) = orbitlogs_models::load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty()) {
        val
    } else {
        return Err(anyhow::anyhow!("DATABASE_URL is required. Set DATABASE_URL env var or pass --database-url."));
    };

    let db_pool = orbitlogs_models::db::establish_db_connection(&db_url, 3).await?;
    let stations = get_stations();

    loop {
        info!("Calculating upcoming contact windows...");
        let active_sats: Vec<String> = (1..=10).map(|i| format!("SAT-{:04}", i)).collect();
        let now = Utc::now();
        let end_time = now + chrono::Duration::hours(24);

        for sat_id in &active_sats {
            // Load latest TLE for satellite
            let row = sqlx::query(
                "SELECT line1, line2, epoch FROM satellite_tles WHERE satellite_id = $1 ORDER BY epoch DESC LIMIT 1"
            )
            .bind(sat_id)
            .fetch_optional(&db_pool)
            .await;

            match row {
                Ok(Some(r)) => {
                    let line1: String = r.get("line1");
                    let line2: String = r.get("line2");
                    let epoch: chrono::DateTime<Utc> = r.get("epoch");
                    let tle = TleRecord {
                        satellite_id: sat_id.clone(),
                        epoch,
                        line1,
                        line2,
                    };

                    for station in &stations {
                        let windows = predict_contact_windows(
                            &tle,
                            &station.id,
                            station.latitude,
                            station.longitude,
                            station.altitude_m,
                            now,
                            end_time,
                        );

                        for window in windows {
                            // Insert window or ignore on duplicate
                            let insert_res = sqlx::query(
                                "INSERT INTO contact_windows (satellite_id, ground_station_id, start_time, end_time, max_elevation)
                                 VALUES ($1, $2, $3, $4, $5)"
                            )
                            .bind(&window.satellite_id)
                            .bind(&window.ground_station_id)
                            .bind(window.start_time)
                            .bind(window.end_time)
                            .bind(window.max_elevation)
                            .execute(&db_pool)
                            .await;

                            if let Err(e) = insert_res {
                                error!("Failed to insert contact window for {} at {}: {}", sat_id, station.id, e);
                            }
                        }
                    }
                }
                Ok(None) => {
                    // No custom TLE, generate default or skip
                }
                Err(e) => {
                    error!("Error querying TLE for {}: {}", sat_id, e);
                }
            }
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received. Exiting Contact Window Predictor gracefully.");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(cli.interval_secs)) => {}
        }
    }
    Ok(())
}
