use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReplaySession {
    pub session_id: uuid::Uuid,
    pub satellite_id: Option<String>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub current_position: chrono::DateTime<chrono::Utc>,
    pub speed: f32,
    pub status: String, // 'PLAYING', 'PAUSED', 'STOPPED'
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbTelemetry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub satellite_id: String,
    pub battery_voltage: Option<f64>,
    pub battery_current: Option<f64>,
    pub solar_panel_output: Option<f64>,
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub temperature: Option<f64>,
    pub orientation_pitch: Option<f32>,
    pub orientation_roll: Option<f32>,
    pub orientation_yaw: Option<f32>,
    pub velocity: Option<f64>,
    pub altitude: Option<f64>,
    pub fuel_remaining: Option<f32>,
    pub communication_latency: Option<f32>,
    pub packet_loss: Option<f32>,
    pub signal_strength: Option<f32>,
    pub event_id: uuid::Uuid,
    pub subsystem_id: String,
    pub mission_id: String,
    pub payload_power_draw: Option<f64>,
    pub payload_active: Option<bool>,
}

impl From<DbTelemetry> for spaceops_models::TelemetryRecord {
    fn from(t: DbTelemetry) -> Self {
        spaceops_models::TelemetryRecord {
            satellite_id: t.satellite_id,
            subsystem_id: t.subsystem_id,
            mission_id: t.mission_id,
            timestamp: t.timestamp,
            battery_voltage: t.battery_voltage,
            battery_current: t.battery_current,
            solar_panel_output: t.solar_panel_output,
            cpu_usage: t.cpu_usage,
            memory_usage: t.memory_usage,
            temperature: t.temperature,
            orientation_pitch: t.orientation_pitch,
            orientation_roll: t.orientation_roll,
            orientation_yaw: t.orientation_yaw,
            velocity: t.velocity,
            altitude: t.altitude,
            fuel_remaining: t.fuel_remaining,
            communication_latency: t.communication_latency,
            packet_loss: t.packet_loss,
            signal_strength: t.signal_strength,
            payload_power_draw: t.payload_power_draw,
            payload_active: t.payload_active,
            event_id: t.event_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub satellite_id: String,
    pub event_type: String,
    pub severity: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbSatelliteHealth {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub satellite_id: String,
    pub mission_id: String,
    pub overall_state: String,
    pub overall_score: i32,
    pub power_state: String,
    pub thermal_state: String,
    pub comm_state: String,
    pub propulsion_state: String,
    pub adcs_state: String,
    pub payload_state: String,
    pub fc_state: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySubsystemHealth {
    pub state: String,
    pub score: i32,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySatelliteHealth {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub satellite_id: String,
    pub mission_id: String,
    pub overall_state: String,
    pub overall_score: i32,
    pub power: ReplaySubsystemHealth,
    pub thermal: ReplaySubsystemHealth,
    pub communications: ReplaySubsystemHealth,
    pub propulsion: ReplaySubsystemHealth,
    pub adcs: ReplaySubsystemHealth,
    pub payload: ReplaySubsystemHealth,
    pub flight_computer: ReplaySubsystemHealth,
    pub explanation: String,
}

impl From<DbSatelliteHealth> for ReplaySatelliteHealth {
    fn from(h: DbSatelliteHealth) -> Self {
        let state_to_score = |s: &str| match s {
            "GREEN" => 100,
            "YELLOW" => 50,
            "RED" => 0,
            _ => 100,
        };

        ReplaySatelliteHealth {
            timestamp: h.timestamp,
            satellite_id: h.satellite_id,
            mission_id: h.mission_id,
            overall_state: h.overall_state,
            overall_score: h.overall_score,
            power: ReplaySubsystemHealth {
                state: h.power_state.clone(),
                score: state_to_score(&h.power_state),
                explanation: format!("Power subsystem is {}", h.power_state),
            },
            thermal: ReplaySubsystemHealth {
                state: h.thermal_state.clone(),
                score: state_to_score(&h.thermal_state),
                explanation: format!("Thermal subsystem is {}", h.thermal_state),
            },
            communications: ReplaySubsystemHealth {
                state: h.comm_state.clone(),
                score: state_to_score(&h.comm_state),
                explanation: format!("Communications subsystem is {}", h.comm_state),
            },
            propulsion: ReplaySubsystemHealth {
                state: h.propulsion_state.clone(),
                score: state_to_score(&h.propulsion_state),
                explanation: format!("Propulsion subsystem is {}", h.propulsion_state),
            },
            adcs: ReplaySubsystemHealth {
                state: h.adcs_state.clone(),
                score: state_to_score(&h.adcs_state),
                explanation: format!("ADCS subsystem is {}", h.adcs_state),
            },
            payload: ReplaySubsystemHealth {
                state: h.payload_state.clone(),
                score: state_to_score(&h.payload_state),
                explanation: format!("Payload subsystem is {}", h.payload_state),
            },
            flight_computer: ReplaySubsystemHealth {
                state: h.fc_state.clone(),
                score: state_to_score(&h.fc_state),
                explanation: format!("Flight computer subsystem is {}", h.fc_state),
            },
            explanation: h.explanation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbGroundStationHealth {
    pub timestamp: chrono::DateTime<chrono::Utc>,
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

pub async fn create_replay_session(
    pool: &PgPool,
    session: &ReplaySession,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO replay_sessions (
            session_id, satellite_id, start_time, end_time, current_position, speed, status, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, NOW(), NOW()
        )
        "#,
    )
    .bind(session.session_id)
    .bind(&session.satellite_id)
    .bind(session.start_time)
    .bind(session.end_time)
    .bind(session.current_position)
    .bind(session.speed)
    .bind(&session.status)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_replay_session(
    pool: &PgPool,
    session_id: uuid::Uuid,
) -> Result<Option<ReplaySession>, sqlx::Error> {
    sqlx::query_as::<_, ReplaySession>(
        r#"
        SELECT session_id, satellite_id, start_time, end_time, current_position, speed, status, created_at, updated_at
        FROM replay_sessions
        WHERE session_id = $1
        "#,
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await
}

pub async fn update_replay_session(
    pool: &PgPool,
    session_id: uuid::Uuid,
    current_position: chrono::DateTime<chrono::Utc>,
    status: &str,
    speed: f32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE replay_sessions
        SET current_position = $1, status = $2, speed = $3, updated_at = NOW()
        WHERE session_id = $4
        "#,
    )
    .bind(current_position)
    .bind(status)
    .bind(speed)
    .bind(session_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn query_telemetry_range(
    pool: &PgPool,
    satellite_id: Option<&str>,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<DbTelemetry>, sqlx::Error> {
    if let Some(sat_id) = satellite_id {
        sqlx::query_as::<_, DbTelemetry>(
            r#"
            SELECT timestamp, satellite_id, battery_voltage, battery_current, solar_panel_output, cpu_usage, memory_usage,
                   temperature, orientation_pitch, orientation_roll, orientation_yaw, velocity, altitude, fuel_remaining,
                   communication_latency, packet_loss, signal_strength, event_id, subsystem_id, mission_id,
                   payload_power_draw, payload_active
            FROM telemetry
            WHERE satellite_id = $1 AND timestamp > $2 AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
        )
        .bind(sat_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, DbTelemetry>(
            r#"
            SELECT timestamp, satellite_id, battery_voltage, battery_current, solar_panel_output, cpu_usage, memory_usage,
                   temperature, orientation_pitch, orientation_roll, orientation_yaw, velocity, altitude, fuel_remaining,
                   communication_latency, packet_loss, signal_strength, event_id, subsystem_id, mission_id,
                   payload_power_draw, payload_active
            FROM telemetry
            WHERE timestamp > $1 AND timestamp <= $2
            ORDER BY timestamp ASC
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
    }
}

pub async fn query_events_range(
    pool: &PgPool,
    satellite_id: Option<&str>,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<DbEvent>, sqlx::Error> {
    if let Some(sat_id) = satellite_id {
        sqlx::query_as::<_, DbEvent>(
            r#"
            SELECT timestamp, satellite_id, event_type, severity, message, details
            FROM events
            WHERE satellite_id = $1 AND timestamp > $2 AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
        )
        .bind(sat_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, DbEvent>(
            r#"
            SELECT timestamp, satellite_id, event_type, severity, message, details
            FROM events
            WHERE timestamp > $1 AND timestamp <= $2
            ORDER BY timestamp ASC
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
    }
}

pub async fn query_satellite_health_range(
    pool: &PgPool,
    satellite_id: Option<&str>,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<DbSatelliteHealth>, sqlx::Error> {
    if let Some(sat_id) = satellite_id {
        sqlx::query_as::<_, DbSatelliteHealth>(
            r#"
            SELECT timestamp, satellite_id, mission_id, overall_state, overall_score, power_state, thermal_state, comm_state,
                   propulsion_state, adcs_state, payload_state, fc_state, explanation
            FROM satellite_health
            WHERE satellite_id = $1 AND timestamp > $2 AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
        )
        .bind(sat_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, DbSatelliteHealth>(
            r#"
            SELECT timestamp, satellite_id, mission_id, overall_state, overall_score, power_state, thermal_state, comm_state,
                   propulsion_state, adcs_state, payload_state, fc_state, explanation
            FROM satellite_health
            WHERE timestamp > $1 AND timestamp <= $2
            ORDER BY timestamp ASC
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
    }
}

pub async fn query_ground_station_health_range(
    pool: &PgPool,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<DbGroundStationHealth>, sqlx::Error> {
    sqlx::query_as::<_, DbGroundStationHealth>(
        r#"
        SELECT timestamp, station_id, station_name, status, antenna_health, weather_state, congestion, latency, bandwidth, explanation
        FROM ground_station_health
        WHERE timestamp > $1 AND timestamp <= $2
        ORDER BY timestamp ASC
        "#,
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
}

pub async fn query_latest_satellites(pool: &PgPool) -> Result<Vec<DbSatelliteHealth>, sqlx::Error> {
    sqlx::query_as::<_, DbSatelliteHealth>(
        r#"
        SELECT DISTINCT ON (satellite_id) timestamp, satellite_id, mission_id, overall_state, overall_score, power_state, thermal_state, comm_state,
               propulsion_state, adcs_state, payload_state, fc_state, explanation
        FROM satellite_health
        ORDER BY satellite_id, timestamp DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn query_latest_ground_stations(
    pool: &PgPool,
) -> Result<Vec<DbGroundStationHealth>, sqlx::Error> {
    sqlx::query_as::<_, DbGroundStationHealth>(
        r#"
        SELECT DISTINCT ON (station_id) timestamp, station_id, station_name, status, antenna_health, weather_state, congestion, latency, bandwidth, explanation
        FROM ground_station_health
        ORDER BY station_id, timestamp DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn query_recent_events(pool: &PgPool) -> Result<Vec<DbEvent>, sqlx::Error> {
    sqlx::query_as::<_, DbEvent>(
        r#"
        SELECT timestamp, satellite_id, event_type, severity, message, details
        FROM events
        ORDER BY timestamp DESC
        LIMIT 50
        "#,
    )
    .fetch_all(pool)
    .await
}
