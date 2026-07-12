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
