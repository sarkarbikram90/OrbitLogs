use sqlx::{PgPool, Row};
use tracing::error;

pub async fn get_latest_telemetry(
    pool: &PgPool,
    sat_id: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let row_opt = sqlx::query(
        "SELECT
    satellite_id,
    timestamp,
    latitude,
    longitude,
    altitude,
    velocity,
    payload
    FROM telemetry
    WHERE satellite_id = $1
    ORDER BY timestamp DESC
    LIMIT 1",
    )
    .bind(sat_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        error!(
            "Failed to fetch latest satellite telemetry {}: {}",
            sat_id, e
        );
        e
    })?;

    Ok(row_opt.map(|row| {
        serde_json::json!({
            "timestamp": row.get::<chrono::DateTime<chrono::Utc>, _>("timestamp"),
            "battery_voltage": row.get::<Option<f64>, _>("battery_voltage"),
            "battery_current": row.get::<Option<f64>, _>("battery_current"),
            "solar_panel_output": row.get::<Option<f64>, _>("solar_panel_output"),
            "cpu_usage": row.get::<Option<f32>, _>("cpu_usage"),
            "memory_usage": row.get::<Option<f32>, _>("memory_usage"),
            "temperature": row.get::<Option<f64>, _>("temperature"),
            "fuel_remaining": row.get::<Option<f32>, _>("fuel_remaining"),
            "signal_strength": row.get::<Option<f32>, _>("signal_strength"),
            "packet_loss": row.get::<Option<f32>, _>("packet_loss"),
            "communication_latency": row.get::<Option<f32>, _>("communication_latency"),
        })
    }))
}

pub async fn get_recent_events(pool: &PgPool, sat_id: &str) -> Vec<serde_json::Value> {
    let rows = match sqlx::query(
        "SELECT
    satellite_id,
    timestamp,
    latitude,
    longitude,
    altitude,
    velocity,
    payload
    FROM telemetry
    WHERE satellite_id = $1
    ORDER BY timestamp DESC
    LIMIT 5",
    )
    .bind(sat_id)
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            error!(
                "Failed to fetch recent events for satellite {}: {}",
                sat_id, e
            );
            return Vec::new();
        }
    };

    rows.into_iter()
        .map(|row| {
            serde_json::json!({
                "timestamp": row.get::<chrono::DateTime<chrono::Utc>, _>("timestamp"),
                "event_type": row.get::<String, _>("event_type"),
                "severity": row.get::<String, _>("severity"),
                "message": row.get::<String, _>("message"),
            })
        })
        .collect()
}

pub async fn get_recent_commands(pool: &PgPool, sat_id: &str) -> Vec<serde_json::Value> {
    let rows = match sqlx::query(
        "SELECT * FROM commands WHERE satellite_id = $1 ORDER BY created_at DESC LIMIT 5",
    )
    .bind(sat_id)
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            error!(
                "Failed to fetch recent commands for satellite {}: {}",
                sat_id, e
            );
            return Vec::new();
        }
    };

    rows.into_iter()
        .map(|row| {
            serde_json::json!({
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                "command_type": row.get::<String, _>("command_type"),
                "status": row.get::<String, _>("status"),
                "created_by": row.get::<String, _>("created_by"),
            })
        })
        .collect()
}
