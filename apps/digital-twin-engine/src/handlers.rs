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

//! HTTP request handlers for the Digital Twin Engine API.
//!
//! All handlers return `Result<Json<T>, AppError>` for consistent error
//! envelopes. The auth middleware validates bearer tokens and returns
//! structured `AppError::Unauthorized` on failure.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use rdkafka::producer::FutureRecord;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::db::TimelineEvent;
use crate::error::AppError;
use crate::metrics;
use crate::AppState;
use orbitlogs_models::TelemetryRecord;

// =========================================================================
// Request / Response Types
// =========================================================================

/// Request body for injecting a simulated subsystem failure.
#[derive(Debug, Deserialize)]
pub struct SimulateFailureRequest {
    pub subsystem: String,
}

/// Query parameters for the timeline endpoint.
#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    /// Maximum number of events to return (default: 50, max: 500).
    pub limit: Option<i64>,
}

/// Response body listing active simulated failures for a satellite.
#[derive(Debug, Serialize)]
pub struct SimulatedFailuresResponse {
    pub satellite_id: String,
    pub active_failures: Vec<String>,
}

/// The set of valid subsystem names that can be used for failure injection.
const VALID_SUBSYSTEMS: &[&str] = &["power", "thermal", "adcs", "communications", "propulsion"];

// =========================================================================
// Authentication Middleware
// =========================================================================

/// Bearer token authentication middleware.
///
/// Extracts the token from the `Authorization: Bearer <token>` header or
/// the `?token=<token>` query parameter. Uses constant-time comparison
/// to mitigate timing side-channel attacks.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, AppError> {
    let mut token_opt = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    // Fallback: check query parameter for WebSocket or browser-based clients.
    if token_opt.is_none() {
        if let Some(query) = req.uri().query() {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                if let (Some("token"), Some(val)) = (parts.next(), parts.next()) {
                    token_opt = Some(val.to_string());
                    break;
                }
            }
        }
    }

    let authenticated = match token_opt {
        Some(ref t) => orbitlogs_models::auth::secure_compare(t, &state.api_key),
        None => false,
    };

    if authenticated {
        Ok(next.run(req).await)
    } else {
        Err(AppError::Unauthorized {
            detail: "Missing or invalid authorization token".to_string(),
        })
    }
}

// =========================================================================
// Handlers
// =========================================================================

/// `GET /api/twin/:satellite`
///
/// Returns the current telemetry snapshot of the satellite's digital twin.
pub async fn get_twin(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Result<Json<TelemetryRecord>, AppError> {
    let rec = state
        .twins
        .get(&satellite)
        .ok_or_else(|| AppError::NotFound {
            entity: "digital twin",
            id: satellite.clone(),
        })?;

    Ok(Json(rec.clone()))
}

/// `GET /api/twin/:satellite/health`
///
/// Computes and returns a full health projection for the satellite's
/// digital twin, including subsystem scores, life expectancy, and
/// operator recommendations.
pub async fn get_twin_health(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Result<Json<crate::simulation::TwinHealthProjection>, AppError> {
    let rec = state
        .twins
        .get(&satellite)
        .ok_or_else(|| AppError::NotFound {
            entity: "digital twin",
            id: satellite.clone(),
        })?
        .clone();

    // Retrieve the twin state for degradation model integration.
    let twin_state_ref = state.twin_states.get(&satellite);
    let twin_state_opt = twin_state_ref.as_ref().map(|r| r.value());

    let timer = metrics::HEALTH_PROJECTION_DURATION.start_timer();
    let projection = crate::simulation::project_twin_health(satellite, &rec, twin_state_opt);
    timer.observe_duration();

    Ok(Json(projection))
}

/// `POST /api/twin/:satellite/monte-carlo`
///
/// Executes a 48-hour Monte Carlo predictive projection for the satellite's digital twin
/// under simulated solar flare or thruster burn scenarios.
pub async fn post_monte_carlo(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
    Json(payload): Json<crate::simulation::MonteCarloRequest>,
) -> Result<Json<crate::simulation::MonteCarloResult>, AppError> {
    let rec = state
        .twins
        .get(&satellite)
        .ok_or_else(|| AppError::NotFound {
            entity: "digital twin",
            id: satellite.clone(),
        })?
        .clone();

    let result = crate::simulation::run_monte_carlo_projection(satellite, &rec, payload);
    Ok(Json(result))
}

/// `GET /api/twin/:satellite/timeline`
///
/// Fetches the most recent events for a satellite from the database.
/// Accepts an optional `?limit=N` query parameter (default 50, max 500).
pub async fn get_twin_timeline(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
    Query(params): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineEvent>>, AppError> {
    let events = crate::db::get_twin_timeline(&state.db_pool, &satellite, params.limit)
        .await
        .map_err(AppError::Database)?;
    Ok(Json(events))
}

/// `POST /api/twin/:satellite/simulate-failure`
///
/// Injects an artificial subsystem failure for the specified satellite.
/// The failure persists until explicitly cleared and affects all subsequent
/// telemetry processing. Requires bearer token authentication.
///
/// Valid subsystems: `power`, `thermal`, `adcs`, `communications`, `propulsion`.
pub async fn simulate_failure(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
    Json(payload): Json<SimulateFailureRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let sub = payload.subsystem.to_lowercase();

    // Validate subsystem name against allowed values.
    if !VALID_SUBSYSTEMS.contains(&sub.as_str()) {
        return Err(AppError::InvalidInput {
            field: "subsystem".to_string(),
            reason: format!(
                "Unknown subsystem '{}'. Valid options: {}",
                sub,
                VALID_SUBSYSTEMS.join(", ")
            ),
        });
    }

    let mut failures = state
        .simulated_failures
        .entry(satellite.clone())
        .or_default();

    if !failures.contains(&sub) {
        failures.push(sub.clone());
        metrics::SIMULATED_FAILURES_ACTIVE
            .with_label_values(&[&satellite, &sub])
            .set(1.0);
    }

    info!(
        satellite = %satellite,
        subsystem = %sub,
        "Digital twin simulated failure injected"
    );

    // Emit timeline event via Kafka
    let timeline_payload = serde_json::json!({
        "timestamp": chrono::Utc::now(),
        "satellite_id": satellite,
        "event_type": "SIMULATED_FAILURE_INJECTED",
        "severity": "CRITICAL",
        "message": format!(
            "Digital twin simulated failure injected on subsystem: {}",
            sub.to_uppercase()
        ),
        "details": {
            "subsystem": sub
        }
    });

    if let Ok(payload_str) = serde_json::to_string(&timeline_payload) {
        let record = FutureRecord::to(&state.timeline_topic)
            .key(&satellite)
            .payload(&payload_str);
        if let Err((e, _)) = state
            .kafka_producer
            .send(record, Duration::from_secs(5))
            .await
        {
            tracing::warn!(error = %e, "Failed to publish timeline event for failure injection");
        }
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "injected",
            "satellite_id": satellite,
            "subsystem": sub
        })),
    ))
}

/// `GET /api/twin/:satellite/simulated-failures`
///
/// Lists all currently active simulated failures for a satellite.
pub async fn list_simulated_failures(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Json<SimulatedFailuresResponse> {
    let active = state
        .simulated_failures
        .get(&satellite)
        .map(|f| f.value().clone())
        .unwrap_or_default();

    Json(SimulatedFailuresResponse {
        satellite_id: satellite,
        active_failures: active,
    })
}

/// `DELETE /api/twin/:satellite/simulated-failures`
///
/// Clears all active simulated failures for a satellite, restoring
/// normal telemetry processing. Requires bearer token authentication.
pub async fn clear_simulated_failures(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    // Remove the entry and collect the cleared subsystems for metrics.
    let cleared = state
        .simulated_failures
        .remove(&satellite)
        .map(|(_, v)| v)
        .unwrap_or_default();

    for sub in &cleared {
        metrics::SIMULATED_FAILURES_ACTIVE
            .with_label_values(&[&satellite, sub])
            .set(0.0);
    }

    info!(
        satellite = %satellite,
        cleared_count = cleared.len(),
        "Digital twin simulated failures cleared"
    );

    // Emit timeline event via Kafka
    let timeline_payload = serde_json::json!({
        "timestamp": chrono::Utc::now(),
        "satellite_id": satellite,
        "event_type": "SIMULATED_FAILURES_CLEARED",
        "severity": "INFO",
        "message": format!(
            "All simulated failures cleared for {}. Subsystems restored: {}",
            satellite,
            cleared.join(", ")
        ),
        "details": {
            "cleared_subsystems": cleared
        }
    });

    if let Ok(payload_str) = serde_json::to_string(&timeline_payload) {
        let record = FutureRecord::to(&state.timeline_topic)
            .key(&satellite)
            .payload(&payload_str);
        if let Err((e, _)) = state
            .kafka_producer
            .send(record, Duration::from_secs(5))
            .await
        {
            tracing::warn!(error = %e, "Failed to publish timeline event for failure clearing");
        }
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "cleared",
            "satellite_id": satellite,
            "cleared_subsystems": cleared
        })),
    ))
}
