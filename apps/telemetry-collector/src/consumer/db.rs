use crate::models::TelemetryRecord;
use sqlx::{Error, PgPool};
use tracing::{debug, error, info};

pub async fn ensure_satellite_exists(pool: &PgPool, satellite_id: &str) -> Result<(), Error> {
    // Check and upsert satellite metadata if it does not exist
    sqlx::query!(
        r#"
        INSERT INTO satellites (id, name, model, launch_date, status)
        VALUES ($1, $2, $3, NOW(), 'ACTIVE')
        ON CONFLICT (id) DO NOTHING
        "#,
        satellite_id,
        format!("Satellite {}", satellite_id),
        "Simulated-V1"
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_telemetry(pool: &PgPool, record: &TelemetryRecord) -> Result<(), Error> {
    // Ensure satellite parent record exists
    ensure_satellite_exists(pool, &record.satellite_id).await?;

    // Insert record
    sqlx::query!(
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
        record.timestamp,
        record.satellite_id,
        record.battery_voltage,
        record.battery_current,
        record.solar_panel_output,
        record.cpu_usage,
        record.memory_usage,
        record.temperature,
        record.orientation_pitch,
        record.orientation_roll,
        record.orientation_yaw,
        record.velocity,
        record.altitude,
        record.fuel_remaining,
        record.communication_latency,
        record.packet_loss,
        record.signal_strength,
        record.event_id
    )
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

    sqlx::query!(
        r#"
        INSERT INTO events (
            timestamp, satellite_id, event_type, severity, message, details
        ) VALUES (
            NOW(), $1, $2, $3, $4, $5
        )
        "#,
        satellite_id,
        event_type,
        severity,
        message,
        details
    )
    .execute(pool)
    .await?;

    info!(
        "Logged space event for satellite {}: [{}] {}",
        satellite_id, severity, message
    );
    Ok(())
}
