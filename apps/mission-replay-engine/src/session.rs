use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateReplayRequest {
    pub satellite_id: Option<String>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub speed: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlReplayRequest {
    pub action: String, // "play", "pause", "stop", "seek", "speed"
    pub position: Option<chrono::DateTime<chrono::Utc>>,
    pub speed: Option<f32>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ReplayData {
    #[serde(rename = "telemetry")]
    Telemetry(orbitlogs_models::TelemetryRecord),
    #[serde(rename = "event")]
    Event(crate::db::DbEvent),
    #[serde(rename = "satellite_health")]
    SatelliteHealth(crate::db::ReplaySatelliteHealth),
    #[serde(rename = "ground_station_health")]
    GroundStationHealth(crate::db::DbGroundStationHealth),
}

pub struct TimedReplayData {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: ReplayData,
}

pub fn api_error(status: StatusCode, message: &str) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

pub fn is_valid_speed(speed: f32) -> bool {
    let allowed = [1.0, 5.0, 10.0, 50.0, 100.0];
    allowed.iter().any(|&s| (s - speed).abs() < f32::EPSILON)
}
