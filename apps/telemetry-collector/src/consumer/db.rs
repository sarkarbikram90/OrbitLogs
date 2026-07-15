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
use crate::models::TelemetryRecord;
use sqlx::{Error, PgPool, Row};
use tracing::{debug, info};

#[allow(async_fn_in_trait)]
pub trait TelemetryStore: Send + Sync {
    async fn ensure_satellite_exists(&self, satellite_id: &str) -> Result<(), Error>;
    async fn get_latest_telemetry(
        &self,
        satellite_id: &str,
    ) -> Result<Option<TelemetryRecord>, Error>;
    async fn insert_telemetry(&self, record: &TelemetryRecord) -> Result<(), Error>;
    async fn insert_event(
        &self,
        satellite_id: &str,
        event_type: &str,
        severity: &str,
        message: &str,
        details: serde_json::Value,
    ) -> Result<(), Error>;
}

pub struct PostgresTelemetryStore {
    pool: PgPool,
}

impl PostgresTelemetryStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TelemetryStore for PostgresTelemetryStore {
    async fn ensure_satellite_exists(&self, satellite_id: &str) -> Result<(), Error> {
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
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_latest_telemetry(
        &self,
        satellite_id: &str,
    ) -> Result<Option<TelemetryRecord>, Error> {
        let row = sqlx::query(
            r#"
            SELECT 
                timestamp, satellite_id, subsystem_id, mission_id, battery_voltage, battery_current, solar_panel_output,
                cpu_usage, memory_usage, temperature, orientation_pitch, orientation_roll,
                orientation_yaw, velocity, altitude, fuel_remaining, communication_latency,
                packet_loss, signal_strength, payload_power_draw, payload_active, event_id
            FROM telemetry
            WHERE satellite_id = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#
        )
        .bind(satellite_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(TelemetryRecord {
                satellite_id: row.get("satellite_id"),
                subsystem_id: row.get("subsystem_id"),
                mission_id: row.get("mission_id"),
                timestamp: row.get("timestamp"),
                battery_voltage: row.get("battery_voltage"),
                battery_current: row.get("battery_current"),
                solar_panel_output: row.get("solar_panel_output"),
                cpu_usage: row.get("cpu_usage"),
                memory_usage: row.get("memory_usage"),
                temperature: row.get("temperature"),
                orientation_pitch: row.get("orientation_pitch"),
                orientation_roll: row.get("orientation_roll"),
                orientation_yaw: row.get("orientation_yaw"),
                velocity: row.get("velocity"),
                altitude: row.get("altitude"),
                fuel_remaining: row.get("fuel_remaining"),
                communication_latency: row.get("communication_latency"),
                packet_loss: row.get("packet_loss"),
                signal_strength: row.get("signal_strength"),
                payload_power_draw: row.get("payload_power_draw"),
                payload_active: row.get("payload_active"),
                event_id: row.get("event_id"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn insert_telemetry(&self, record: &TelemetryRecord) -> Result<(), Error> {
        self.ensure_satellite_exists(&record.satellite_id).await?;

        sqlx::query(
            r#"
            INSERT INTO telemetry (
                timestamp, satellite_id, subsystem_id, mission_id, battery_voltage, battery_current, solar_panel_output,
                cpu_usage, memory_usage, temperature, orientation_pitch, orientation_roll,
                orientation_yaw, velocity, altitude, fuel_remaining, communication_latency,
                packet_loss, signal_strength, payload_power_draw, payload_active, event_id
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22
            )
            "#,
        )
        .bind(record.timestamp)
        .bind(&record.satellite_id)
        .bind(record.subsystem_id)
        .bind(&record.mission_id)
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
        .bind(record.payload_power_draw)
        .bind(record.payload_active)
        .bind(record.event_id)
        .execute(&self.pool)
        .await?;

        debug!(
            "Telemetry inserted into database for {}",
            record.satellite_id
        );
        Ok(())
    }

    async fn insert_event(
        &self,
        satellite_id: &str,
        event_type: &str,
        severity: &str,
        message: &str,
        details: serde_json::Value,
    ) -> Result<(), Error> {
        self.ensure_satellite_exists(satellite_id).await?;

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
        .execute(&self.pool)
        .await?;

        info!(
            "Logged space event for satellite {}: [{}] {}",
            satellite_id, severity, message
        );
        Ok(())
    }
}
