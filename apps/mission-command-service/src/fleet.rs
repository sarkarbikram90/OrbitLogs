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

//! Fleet management API — CRUD endpoints for tenant-scoped satellite registration.

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
    validate_jwt, CreateSatelliteRequest, RegisteredSatellite, Role, TenantContext,
};

use crate::AppState;

#[derive(Serialize)]
pub struct FleetListResponse {
    pub satellites: Vec<RegisteredSatellite>,
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

// --- LIST SATELLITES ---

pub async fn list_satellites(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<FleetListResponse>, crate::error::AppError> {
    let ctx = extract_tenant_context(&state, &headers)?;

    let rows = sqlx::query(
        "SELECT id, tenant_id, satellite_id, satellite_name, norad_id, orbit_type, \
         tle_line1, tle_line2, tle_epoch, launch_date, mission_type, mass_kg, \
         status, created_at, updated_at \
         FROM registered_satellites WHERE tenant_id = $1 \
         ORDER BY created_at DESC",
    )
    .bind(ctx.tenant_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to query satellites");
        crate::error::AppError::Internal("Database query failed".to_string())
    })?;

    let satellites: Vec<RegisteredSatellite> = rows
        .iter()
        .map(|row| RegisteredSatellite {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            satellite_id: row.get("satellite_id"),
            satellite_name: row.get("satellite_name"),
            norad_id: row.get("norad_id"),
            orbit_type: row.get("orbit_type"),
            tle_line1: row.get("tle_line1"),
            tle_line2: row.get("tle_line2"),
            tle_epoch: row.get("tle_epoch"),
            launch_date: row.get("launch_date"),
            mission_type: row.get("mission_type"),
            mass_kg: row.get("mass_kg"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    let total = satellites.len() as i64;

    Ok(Json(FleetListResponse { satellites, total }))
}

// --- CREATE SATELLITE ---

pub async fn create_satellite(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateSatelliteRequest>,
) -> Result<(StatusCode, Json<RegisteredSatellite>), crate::error::AppError> {
    let ctx = extract_tenant_context(&state, &headers)?;
    ctx.require_role(Role::Operator).map_err(|e| crate::error::AppError::Forbidden(e))?;

    // Validate satellite_id format
    if req.satellite_id.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "satellite_id cannot be empty".to_string(),
        ));
    }

    // Check tenant satellite limit
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM registered_satellites WHERE tenant_id = $1 AND status != 'DECOMMISSIONED'",
    )
    .bind(ctx.tenant_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to count satellites");
        crate::error::AppError::Internal("Database query failed".to_string())
    })?;

    // Get tenant limits
    let max_sats: i32 = sqlx::query_scalar(
        "SELECT max_satellites FROM tenants WHERE id = $1",
    )
    .bind(ctx.tenant_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to query tenant limits");
        crate::error::AppError::Internal("Database query failed".to_string())
    })?;

    if count >= max_sats as i64 {
        return Err(crate::error::AppError::Validation(format!(
            "Satellite limit reached ({}/{}). Upgrade your plan to add more.",
            count, max_sats
        )));
    }

    let now = Utc::now();
    let id = Uuid::new_v4();
    let orbit_type = req.orbit_type.as_deref().unwrap_or("LEO");

    let launch_date = req
        .launch_date
        .as_ref()
        .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
        .map(|d| d.with_timezone(&Utc));

    sqlx::query(
        "INSERT INTO registered_satellites \
         (id, tenant_id, satellite_id, satellite_name, norad_id, orbit_type, \
          tle_line1, tle_line2, launch_date, mission_type, mass_kg, status, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'ACTIVE', $12, $12)",
    )
    .bind(id)
    .bind(ctx.tenant_id)
    .bind(&req.satellite_id)
    .bind(&req.satellite_name)
    .bind(req.norad_id)
    .bind(orbit_type)
    .bind(&req.tle_line1)
    .bind(&req.tle_line2)
    .bind(launch_date)
    .bind(&req.mission_type)
    .bind(req.mass_kg)
    .bind(now)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to insert satellite");
        if e.to_string().contains("duplicate key") {
            crate::error::AppError::Validation(format!(
                "Satellite '{}' already registered for this tenant",
                req.satellite_id
            ))
        } else {
            crate::error::AppError::Internal("Database insert failed".to_string())
        }
    })?;

    // Also insert into the legacy `satellites` table for backward compatibility
    let _ = sqlx::query(
        "INSERT INTO satellites (id, name, model, launch_date, status) \
         VALUES ($1, $2, $3, $4, 'ACTIVE') ON CONFLICT (id) DO NOTHING",
    )
    .bind(&req.satellite_id)
    .bind(&req.satellite_name)
    .bind(orbit_type)
    .bind(launch_date.unwrap_or(now))
    .execute(&state.db_pool)
    .await;

    info!(
        satellite_id = %req.satellite_id,
        tenant = %ctx.tenant_slug,
        "Satellite registered"
    );

    let sat = RegisteredSatellite {
        id,
        tenant_id: ctx.tenant_id,
        satellite_id: req.satellite_id,
        satellite_name: req.satellite_name,
        norad_id: req.norad_id,
        orbit_type: orbit_type.to_string(),
        tle_line1: req.tle_line1,
        tle_line2: req.tle_line2,
        tle_epoch: None,
        launch_date,
        mission_type: req.mission_type,
        mass_kg: req.mass_kg,
        status: "ACTIVE".to_string(),
        created_at: now,
        updated_at: now,
    };

    Ok((StatusCode::CREATED, Json(sat)))
}

// --- GET SATELLITE ---

pub async fn get_satellite(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(satellite_id): Path<String>,
) -> Result<Json<RegisteredSatellite>, crate::error::AppError> {
    let ctx = extract_tenant_context(&state, &headers)?;

    let row = sqlx::query(
        "SELECT id, tenant_id, satellite_id, satellite_name, norad_id, orbit_type, \
         tle_line1, tle_line2, tle_epoch, launch_date, mission_type, mass_kg, \
         status, created_at, updated_at \
         FROM registered_satellites WHERE satellite_id = $1 AND tenant_id = $2",
    )
    .bind(&satellite_id)
    .bind(ctx.tenant_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to query satellite");
        crate::error::AppError::Internal("Database query failed".to_string())
    })?
    .ok_or_else(|| {
        crate::error::AppError::NotFound(format!("Satellite '{}' not found", satellite_id))
    })?;

    Ok(Json(RegisteredSatellite {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        satellite_id: row.get("satellite_id"),
        satellite_name: row.get("satellite_name"),
        norad_id: row.get("norad_id"),
        orbit_type: row.get("orbit_type"),
        tle_line1: row.get("tle_line1"),
        tle_line2: row.get("tle_line2"),
        tle_epoch: row.get("tle_epoch"),
        launch_date: row.get("launch_date"),
        mission_type: row.get("mission_type"),
        mass_kg: row.get("mass_kg"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }))
}

// --- DELETE SATELLITE ---

pub async fn delete_satellite(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(satellite_id): Path<String>,
) -> Result<StatusCode, crate::error::AppError> {
    let ctx = extract_tenant_context(&state, &headers)?;
    ctx.require_role(Role::MissionManager).map_err(|e| crate::error::AppError::Forbidden(e))?;

    let result = sqlx::query(
        "UPDATE registered_satellites SET status = 'DECOMMISSIONED', updated_at = NOW() \
         WHERE satellite_id = $1 AND tenant_id = $2",
    )
    .bind(&satellite_id)
    .bind(ctx.tenant_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to decommission satellite");
        crate::error::AppError::Internal("Database update failed".to_string())
    })?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound(format!(
            "Satellite '{}' not found",
            satellite_id
        )));
    }

    info!(
        satellite_id = %satellite_id,
        tenant = %ctx.tenant_slug,
        "Satellite decommissioned"
    );

    Ok(StatusCode::NO_CONTENT)
}

// --- UPDATE TLE ---

#[derive(serde::Deserialize)]
pub struct UpdateTleRequest {
    pub tle_line1: String,
    pub tle_line2: String,
}

pub async fn update_satellite_tle(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(satellite_id): Path<String>,
    Json(req): Json<UpdateTleRequest>,
) -> Result<StatusCode, crate::error::AppError> {
    let ctx = extract_tenant_context(&state, &headers)?;
    ctx.require_role(Role::Operator).map_err(|e| crate::error::AppError::Forbidden(e))?;

    // Basic TLE validation
    if req.tle_line1.len() < 69 || req.tle_line2.len() < 69 {
        return Err(crate::error::AppError::Validation(
            "TLE lines must be at least 69 characters".to_string(),
        ));
    }

    let result = sqlx::query(
        "UPDATE registered_satellites SET tle_line1 = $1, tle_line2 = $2, \
         tle_epoch = NOW(), updated_at = NOW() \
         WHERE satellite_id = $3 AND tenant_id = $4",
    )
    .bind(&req.tle_line1)
    .bind(&req.tle_line2)
    .bind(&satellite_id)
    .bind(ctx.tenant_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to update TLE");
        crate::error::AppError::Internal("Database update failed".to_string())
    })?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound(format!(
            "Satellite '{}' not found",
            satellite_id
        )));
    }

    info!(satellite_id = %satellite_id, "TLE updated");

    Ok(StatusCode::NO_CONTENT)
}
