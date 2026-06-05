use sqlx::{Error, PgPool};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundStationHealth {
    pub timestamp: DateTime<Utc>,
    pub station_id: String,
    pub station_name: String,
    pub status: String,
    pub antenna_health: f32,
    pub weather_state: String,
    pub congestion: f32,
    pub latency: f32,
    pub bandwidth: f32,
    pub explanation: String,
}

pub async fn insert_ground_station_health(
    pool: &PgPool,
    gs: &GroundStationHealth,
) -> Result<(), Error> {
    sqlx::query(
        r#"
        INSERT INTO ground_station_health (
            timestamp, station_id, station_name, status, antenna_health,
            weather_state, congestion, latency, bandwidth, explanation
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
        )
        "#,
    )
    .bind(gs.timestamp)
    .bind(&gs.station_id)
    .bind(&gs.station_name)
    .bind(&gs.status)
    .bind(gs.antenna_health)
    .bind(&gs.weather_state)
    .bind(gs.congestion)
    .bind(gs.latency)
    .bind(gs.bandwidth)
    .bind(&gs.explanation)
    .execute(pool)
    .await?;

    Ok(())
}
