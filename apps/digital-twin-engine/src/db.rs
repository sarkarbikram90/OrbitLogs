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

//! Database access layer for the Digital Twin Engine.
//!
//! Provides typed queries against the TimescaleDB `events` table using
//! explicit column lists (no `SELECT *`), parameterised limits, and
//! structured result types.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};

/// Default maximum number of timeline events returned per query.
pub const DEFAULT_TIMELINE_LIMIT: i64 = 50;

/// Maximum allowed limit to prevent unbounded result sets.
pub const MAX_TIMELINE_LIMIT: i64 = 500;

/// A single timeline event retrieved from the `events` table.
///
/// Matches the schema:
/// ```sql
/// CREATE TABLE events (
///     timestamp  TIMESTAMPTZ NOT NULL,
///     satellite_id VARCHAR(50) NOT NULL,
///     event_type   VARCHAR(50) NOT NULL,
///     severity     VARCHAR(20) NOT NULL,
///     message      TEXT NOT NULL,
///     details      JSONB
/// );
/// ```
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    pub satellite_id: String,
    pub event_type: String,
    pub severity: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Fetches the most recent timeline events for a satellite.
///
/// # Arguments
/// * `pool` — Database connection pool.
/// * `satellite` — Satellite ID to filter by.
/// * `limit` — Maximum number of events to return. Clamped to
///   `[1, MAX_TIMELINE_LIMIT]`.
pub async fn get_twin_timeline(
    pool: &PgPool,
    satellite: &str,
    limit: Option<i64>,
) -> Result<Vec<TimelineEvent>, sqlx::Error> {
    let effective_limit = limit
        .unwrap_or(DEFAULT_TIMELINE_LIMIT)
        .clamp(1, MAX_TIMELINE_LIMIT);

    let events = sqlx::query_as::<_, TimelineEvent>(
        r#"
        SELECT timestamp,
               satellite_id,
               event_type,
               severity,
               message,
               details
        FROM events
        WHERE satellite_id = $1
        ORDER BY timestamp DESC
        LIMIT $2
        "#,
    )
    .bind(satellite)
    .bind(effective_limit)
    .fetch_all(pool)
    .await?;

    Ok(events)
}

/// Persists a snapshot of the digital twin state for historical analysis.
///
/// This is intended to be called periodically (e.g., every N telemetry
/// updates) to build a degradation history that can be queried for trend
/// analysis or replay.
pub async fn persist_twin_snapshot(
    pool: &PgPool,
    satellite_id: &str,
    twin_state: &crate::simulation::DigitalTwinState,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO events (timestamp, satellite_id, event_type, severity, message, details)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(Utc::now())
    .bind(satellite_id)
    .bind("TWIN_STATE_SNAPSHOT")
    .bind("INFO")
    .bind(format!("Digital twin state snapshot for {}", satellite_id))
    .bind(serde_json::to_value(twin_state).unwrap_or(serde_json::Value::Null))
    .execute(pool)
    .await?;

    Ok(())
}
