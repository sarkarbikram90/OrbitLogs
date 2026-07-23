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
use chrono::{DateTime, Utc};
use clap::Parser;
use rand::Rng;
use sqlx::{PgPool, Row};
use std::time::Duration;
use tracing::{error, info, Level};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "Constellation Scheduler")]
struct Cli {
    #[arg(long, env = "DATABASE_URL", default_value = "")]
    database_url: String,

    #[arg(long, env = "SCHEDULER_INTERVAL_SECS", default_value = "60")]
    interval_secs: u64,
}

pub struct GroundStationAntenna {
    pub id: String,
    pub min_elevation_angle: f32,
    pub max_slew_rate: f32,
    pub operational_status: String,
}

impl GroundStationAntenna {
    pub fn calculate_path_loss(
        &self,
        distance_km: f64,
        frequency_ghz: f64,
        rain_rate_mm_h: f64,
    ) -> f64 {
        let free_space_loss = 20.0 * distance_km.log10() + 20.0 * frequency_ghz.log10() + 92.44;
        // Simplified rain fade model
        let rain_loss = 0.05 * rain_rate_mm_h.powf(1.15) * distance_km;
        free_space_loss + rain_loss
    }
}

struct ContactWindow {
    satellite_id: String,
    ground_station_id: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    max_elevation: f32,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    info!("Starting Constellation Scheduler...");

    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) =
        orbitlogs_models::load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty())
    {
        val
    } else {
        return Err(anyhow::anyhow!(
            "DATABASE_URL is required. Set DATABASE_URL env var or pass --database-url."
        ));
    };

    let db_pool = orbitlogs_models::db::establish_db_connection(&db_url, 3).await?;

    let antennas = vec![
        GroundStationAntenna {
            id: "ANT-A".to_string(),
            min_elevation_angle: 15.0,
            max_slew_rate: 2.5,
            operational_status: "ACTIVE".to_string(),
        },
        GroundStationAntenna {
            id: "ANT-B".to_string(),
            min_elevation_angle: 15.0,
            max_slew_rate: 2.5,
            operational_status: "ACTIVE".to_string(),
        },
    ];

    loop {
        info!("Running scheduling optimization solver...");
        let now = Utc::now();

        // 1. Fetch upcoming contact windows for the next 2 hours
        let windows_query = sqlx::query(
            "SELECT satellite_id, ground_station_id, start_time, end_time, max_elevation 
             FROM contact_windows 
             WHERE start_time >= $1 AND start_time <= $2",
        )
        .bind(now)
        .bind(now + chrono::Duration::hours(2))
        .fetch_all(&db_pool)
        .await;

        match windows_query {
            Ok(rows) => {
                let mut windows = Vec::new();
                for row in rows {
                    windows.push(ContactWindow {
                        satellite_id: row.get("satellite_id"),
                        ground_station_id: row.get("ground_station_id"),
                        start_time: row.get("start_time"),
                        end_time: row.get("end_time"),
                        max_elevation: row.get("max_elevation"),
                    });
                }

                info!("Fetched {} upcoming contact windows.", windows.len());

                // 2. Simple Heuristic Solver: Schedule links based on elevation and rain rate
                for window in windows {
                    let mut rng = rand::thread_rng();
                    let rain_rate: f64 = rng.gen_range(0.0..15.0); // simulated rain rate in mm/h
                    let distance_km = 550.0; // typical low earth orbit altitude
                    let frequency_ghz = 28.0; // Ka-band downlink frequency

                    // Calculate path loss
                    let antenna = &antennas[0];
                    let path_loss =
                        antenna.calculate_path_loss(distance_km, frequency_ghz, rain_rate);

                    // If path loss is acceptable, schedule the contact
                    if path_loss < 160.0 {
                        // threshold for link lock
                        let id = Uuid::new_v4();
                        let priority = if rain_rate > 10.0 {
                            "CRITICAL"
                        } else {
                            "ROUTINE"
                        };

                        // Try to insert schedule
                        let insert_res = sqlx::query(
                            "INSERT INTO contact_schedules (id, satellite_id, station_id, window_type, start_time, end_time, priority, status)
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                             ON CONFLICT (id) DO NOTHING"
                        )
                        .bind(id)
                        .bind(&window.satellite_id)
                        .bind(&window.ground_station_id)
                        .bind("DOWNLINK")
                        .bind(window.start_time)
                        .bind(window.end_time)
                        .bind(priority)
                        .bind("RESERVED")
                        .execute(&db_pool)
                        .await;

                        if let Err(e) = insert_res {
                            error!(
                                "Failed to persist schedule for satellite {}: {}",
                                window.satellite_id, e
                            );
                        } else {
                            info!("Scheduled DOWNLINK for satellite {} at station {} (Path Loss: {:.2} dB)", 
                                  window.satellite_id, window.ground_station_id, path_loss);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch contact windows: {}", e);
            }
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received. Exiting Constellation Scheduler gracefully.");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(cli.interval_secs)) => {}
        }
    }
    Ok(())
}
