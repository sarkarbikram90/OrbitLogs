use crate::models::TelemetryRecord;
use sqlx::{Error, PgPool};
use tracing::{debug, info};

pub async fn ensure_satellite_exists(pool: &PgPool, satellite_id: &str) -> Result<(), Error> {
    // Check and upsert satellite metadata if it does not exist
    sqlx::query(
        r#"
        INSERT INTO satellites (id, name, model, launch_date, status)
        VALUES ($1, $2, $3, NOW(), 'ACTIVE')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(satellite_id)
    .bind(format!("Satellite {}", satellite_id))
    .bind("Simulated-V1")
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_telemetry(pool: &PgPool, record: &TelemetryRecord) -> Result<(), Error> {
    // Ensure satellite parent record exists
    ensure_satellite_exists(pool, &record.satellite_id).await?;

    // Insert record
    sqlx::query(
        r#"
        INSERT INTO telemetry (
            timestamp, satellite_id, battery_voltage, battery_current, solar_panel_output,
            cpu_usage, memory_usage, temperature, orientation_pitch, orientation_roll,
            orientation_yaw, velocity, altitude, fuel_remaining, communication_latency,
            packet_loss, signal_strength, event_id
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18
        )
        "#,
    )
    .bind(record.timestamp)
    .bind(&record.satellite_id)
    .bind(record.battery_voltage)
    .bind(record.battery_current)
    .bind(record.solar_panel_output)
    .bind(record.cpu_usage)
    .bind(record.memory_usage)
    .bind(record.temperature)
    .bind(record.orientation_pitch)
    .bind(record.orientation_roll)
    .bind(record.orientation_yaw)
    .bind(record.velocity)
    .bind(record.altitude)
    .bind(record.fuel_remaining)
    .bind(record.communication_latency)
    .bind(record.packet_loss)
    .bind(record.signal_strength)
    .bind(record.event_id)
    .execute(pool)
    .await?;

    debug!(
        "Telemetry inserted into database for {}",
        record.satellite_id
    );
    Ok(())
}

pub async fn insert_event(
    pool: &PgPool,
    satellite_id: &str,
    event_type: &str,
    severity: &str,
    message: &str,
    details: serde_json::Value,
) -> Result<(), Error> {
    ensure_satellite_exists(pool, satellite_id).await?;

    sqlx::query(
        r#"
        INSERT INTO events (
            timestamp, satellite_id, event_type, severity, message, details
        ) VALUES (
            NOW(), $1, $2, $3, $4, $5
        )
        "#,
    )
    .bind(satellite_id)
    .bind(event_type)
    .bind(severity)
    .bind(message)
    .bind(details)
    .execute(pool)
    .await?;

    info!(
        "Logged space event for satellite {}: [{}] {}",
        satellite_id, severity, message
    );
    Ok(())
}
