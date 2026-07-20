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

//! Multi-format data exporter for OrbitLogs telemetry (CSV, Cesium CZML, CCSDS binary).

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::Row;
use std::sync::Arc;
use crate::AppState;

#[derive(Deserialize)]
pub struct ExportParams {
    pub format: Option<String>, // "csv", "czml", "ccsds"
    pub limit: Option<i64>,
}

pub async fn export_satellite_data(
    State(state): State<Arc<AppState>>,
    Path(sat_id): Path<String>,
    Query(params): Query<ExportParams>,
) -> impl IntoResponse {
    let fmt = params.format.as_deref().unwrap_or("csv").to_lowercase();
    let limit = params.limit.unwrap_or(500).min(5000);

    let rows = match sqlx::query(
        "SELECT timestamp, battery_voltage, battery_current, solar_panel_output, \
         temperature, altitude, velocity, fuel_remaining \
         FROM telemetry WHERE satellite_id = $1 \
         ORDER BY timestamp DESC LIMIT $2",
    )
    .bind(&sat_id)
    .bind(limit)
    .fetch_all(&state.db_pool)
    .await
    {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    match fmt.as_str() {
        "czml" => {
            // Generate Cesium CZML document for STK/CesiumJS 3D rendering
            let mut czml = vec![serde_json::json!({
                "id": "document",
                "name": format!("OrbitLogs Trajectory: {}", sat_id),
                "version": "1.0"
            })];

            let mut positions: Vec<serde_json::Value> = Vec::new();

            for row in &rows {
                let ts: chrono::DateTime<chrono::Utc> = row.get("timestamp");
                let alt: f64 = row.get("altitude");
                // Simplified ECEF approximation for CZML visualization
                positions.push(serde_json::json!(ts.to_rfc3339()));
                positions.push(serde_json::json!(77.56)); // Longitude (m)
                positions.push(serde_json::json!(13.03)); // Latitude (m)
                positions.push(serde_json::json!(alt * 1000.0)); // Altitude (meters)
            }

            czml.push(serde_json::json!({
                "id": format!("Satellite/{}", sat_id),
                "name": sat_id,
                "position": {
                    "cartographicDegrees": positions
                }
            }));

            let json_str = serde_json::to_string_pretty(&czml).unwrap_or_default();
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}_trajectory.czml\"", sat_id).parse().unwrap(),
            );
            (StatusCode::OK, headers, json_str).into_response()
        }
        "ccsds" => {
            // Generate simulated CCSDS Transfer Stream
            let mut binary_data = Vec::new();
            for (idx, _row) in rows.iter().enumerate() {
                // CCSDS Primary Header (6 bytes)
                let pkt_num = (idx as u16) & 0x3FFF;
                let hdr = [0x08, 0x01, (pkt_num >> 8) as u8, pkt_num as u8, 0x00, 0x10];
                binary_data.extend_from_slice(&hdr);
                // Payload dummy bytes
                binary_data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x00, 0x11, 0x22, 0x33]);
            }
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
            headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}_telemetry.stream\"", sat_id).parse().unwrap(),
            );
            (StatusCode::OK, headers, binary_data).into_response()
        }
        _ => {
            // Default: CSV format
            let mut csv_out = String::from("timestamp,battery_voltage,battery_current,solar_panel_output,temperature,altitude,velocity,fuel_remaining\n");
            for row in rows {
                let ts: chrono::DateTime<chrono::Utc> = row.get("timestamp");
                let v: f64 = row.get("battery_voltage");
                let i: f64 = row.get("battery_current");
                let w: f64 = row.get("solar_panel_output");
                let t: f64 = row.get("temperature");
                let alt: f64 = row.get("altitude");
                let vel: f64 = row.get("velocity");
                let fuel: f32 = row.get("fuel_remaining");

                csv_out.push_str(&format!("{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.1}\n", ts.to_rfc3339(), v, i, w, t, alt, vel, fuel));
            }
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/csv".parse().unwrap());
            headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}_telemetry.csv\"", sat_id).parse().unwrap(),
            );
            (StatusCode::OK, headers, csv_out).into_response()
        }
    }
}
