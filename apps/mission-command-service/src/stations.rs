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

//! Ground station management API — CRUD endpoints for tenant-scoped ground station registration.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use orbitlogs_models::auth::{
    validate_jwt, CreateGroundStationRequest, RegisteredGroundStation, Role, TenantContext,
};

use crate::AppState;

#[derive(Serialize)]
pub struct StationListResponse {
    pub ground_stations: Vec<RegisteredGroundStation>,
    pub total: i64,
}

fn extract_tenant_context(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<TenantContext, crate::error::AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| {
            crate::error::AppError::Auth("Missing Authorization Bearer token".to_string())
        })?;

    validate_jwt(token, &state.jwt_secret)
        .map_err(|e| crate::error::AppError::Auth(e))
}

// --- LIST GROUND STATIONS ---

pub async fn list_ground_stations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<StationListResponse>, crate::error::AppError> {
    let ctx = extract_tenant_context(&state, &headers)?;

    let rows = sqlx::query(
        "SELECT id, tenant_id, station_id, station_name, latitude, longitude, \
         altitude_m, antenna_type, frequency_band, min_elevation_deg, \
         status, created_at, updated_at \
         FROM registered_ground_stations WHERE tenant_id = $1 \
         ORDER BY station_name ASC",
    )
    .bind(ctx.tenant_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to query ground stations");
        crate::error::AppError::Internal("Database query failed".to_string())
    })?;

    let ground_stations: Vec<RegisteredGroundStation> = rows
        .iter()
        .map(|row| RegisteredGroundStation {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            station_id: row.get("station_id"),
            station_name: row.get("station_name"),
            latitude: row.get("latitude"),
            longitude: row.get("longitude"),
            altitude_m: row.get("altitude_m"),
            antenna_type: row.get("antenna_type"),
            frequency_band: row.get("frequency_band"),
            min_elevation_deg: row.get("min_elevation_deg"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    let total = ground_stations.len() as i64;

    Ok(Json(StationListResponse {
        ground_stations,
        total,
    }))
}

// --- CREATE GROUND STATION ---

pub async fn create_ground_station(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateGroundStationRequest>,
) -> Result<(StatusCode, Json<RegisteredGroundStation>), crate::error::AppError> {
    let ctx = extract_tenant_context(&state, &headers)?;
    ctx.require_role(Role::Operator)
        .map_err(|e| crate::error::AppError::Forbidden(e))?;

    // Validate coordinates
    if !(-90.0..=90.0).contains(&req.latitude) {
        return Err(crate::error::AppError::Validation(
            "Latitude must be between -90 and 90 degrees".to_string(),
        ));
    }
    if !(-180.0..=180.0).contains(&req.longitude) {
        return Err(crate::error::AppError::Validation(
            "Longitude must be between -180 and 180 degrees".to_string(),
        ));
    }

    // Check tenant limits
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM registered_ground_stations WHERE tenant_id = $1 AND status != 'DECOMMISSIONED'",
    )
    .bind(ctx.tenant_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to count ground stations");
        crate::error::AppError::Internal("Database query failed".to_string())
    })?;

    let max_stations: i32 = sqlx::query_scalar(
        "SELECT max_ground_stations FROM tenants WHERE id = $1",
    )
    .bind(ctx.tenant_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to query tenant limits");
        crate::error::AppError::Internal("Database query failed".to_string())
    })?;

    if count >= max_stations as i64 {
        return Err(crate::error::AppError::Validation(format!(
            "Ground station limit reached ({}/{}). Upgrade your plan to add more.",
            count, max_stations
        )));
    }

    let now = Utc::now();
    let id = Uuid::new_v4();
    let altitude_m = req.altitude_m.unwrap_or(0.0);
    let min_elevation = req.min_elevation_deg.unwrap_or(10.0);

    sqlx::query(
        "INSERT INTO registered_ground_stations \
         (id, tenant_id, station_id, station_name, latitude, longitude, altitude_m, \
          antenna_type, frequency_band, min_elevation_deg, status, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'ONLINE', $11, $11)",
    )
    .bind(id)
    .bind(ctx.tenant_id)
    .bind(&req.station_id)
    .bind(&req.station_name)
    .bind(req.latitude)
    .bind(req.longitude)
    .bind(altitude_m)
    .bind(&req.antenna_type)
    .bind(&req.frequency_band)
    .bind(min_elevation)
    .bind(now)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to insert ground station");
        if e.to_string().contains("duplicate key") {
            crate::error::AppError::Validation(format!(
                "Ground station '{}' already registered for this tenant",
                req.station_id
            ))
        } else {
            crate::error::AppError::Internal("Database insert failed".to_string())
        }
    })?;

    info!(
        station_id = %req.station_id,
        tenant = %ctx.tenant_slug,
        lat = req.latitude,
        lon = req.longitude,
        "Ground station registered"
    );

    let station = RegisteredGroundStation {
        id,
        tenant_id: ctx.tenant_id,
        station_id: req.station_id,
        station_name: req.station_name,
        latitude: req.latitude,
        longitude: req.longitude,
        altitude_m,
        antenna_type: req.antenna_type,
        frequency_band: req.frequency_band,
        min_elevation_deg: min_elevation,
        status: "ONLINE".to_string(),
        created_at: now,
        updated_at: now,
    };

    Ok((StatusCode::CREATED, Json(station)))
}

// --- DELETE GROUND STATION ---

pub async fn delete_ground_station(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(station_id): Path<String>,
) -> Result<StatusCode, crate::error::AppError> {
    let ctx = extract_tenant_context(&state, &headers)?;
    ctx.require_role(Role::MissionManager)
        .map_err(|e| crate::error::AppError::Forbidden(e))?;

    let result = sqlx::query(
        "UPDATE registered_ground_stations SET status = 'DECOMMISSIONED', updated_at = NOW() \
         WHERE station_id = $1 AND tenant_id = $2",
    )
    .bind(&station_id)
    .bind(ctx.tenant_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to decommission ground station");
        crate::error::AppError::Internal("Database update failed".to_string())
    })?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound(format!(
            "Ground station '{}' not found",
            station_id
        )));
    }

    info!(
        station_id = %station_id,
        tenant = %ctx.tenant_slug,
        "Ground station decommissioned"
    );

    Ok(StatusCode::NO_CONTENT)
}
