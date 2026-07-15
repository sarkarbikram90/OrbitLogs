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
use sqlx::{PgPool, Row};

pub async fn get_twin_timeline(
    pool: &PgPool,
    satellite: &str,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT * FROM events WHERE satellite_id = $1 ORDER BY timestamp DESC LIMIT 50",
    )
    .bind(satellite)
    .fetch_all(pool)
    .await?;

    let events = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "timestamp": row.get::<chrono::DateTime<chrono::Utc>, _>("timestamp"),
                "satellite_id": row.get::<String, _>("satellite_id"),
                "event_type": row.get::<String, _>("event_type"),
                "severity": row.get::<String, _>("severity"),
                "message": row.get::<String, _>("message"),
                "details": row.get::<Option<serde_json::Value>, _>("details"),
            })
        })
        .collect();

    Ok(events)
}
