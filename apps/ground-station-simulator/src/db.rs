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
use sqlx::{Error, PgPool};

pub use orbitlogs_models::GroundStationHealth;

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
    .bind(gs.status)
    .bind(gs.antenna_health)
    .bind(gs.weather_state)
    .bind(gs.congestion)
    .bind(gs.latency)
    .bind(gs.bandwidth)
    .bind(&gs.explanation)
    .execute(pool)
    .await?;

    Ok(())
}
