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
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rdkafka::producer::FutureRecord;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::AppState;
use orbitlogs_models::TelemetryRecord;

#[derive(Debug, Deserialize)]
pub struct SimulateFailureRequest {
    pub subsystem: String,
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, std::convert::Infallible> {
    let mut token_opt = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

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
        let body = serde_json::json!({
            "error": "Unauthorized",
            "details": vec!["Missing or invalid authorization token"]
        })
        .to_string();
        let mut response = axum::response::Response::new(axum::body::Body::from(body));
        *response.status_mut() = StatusCode::UNAUTHORIZED;
        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        if let Ok(origin_val) = axum::http::HeaderValue::from_str(&state.cors_allowed_origin) {
            response
                .headers_mut()
                .insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
        }
        Ok(response)
    }
}

pub async fn cors_middleware(
    State(state): State<Arc<AppState>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = request.method().clone();
    let allowed_origin = state.cors_allowed_origin.clone();

    if method == axum::http::Method::OPTIONS {
        return axum::response::Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("access-control-allow-origin", &allowed_origin)
            .header("access-control-allow-methods", "GET, POST, OPTIONS")
            .header(
                "access-control-allow-headers",
                "content-type, x-request-id, authorization",
            )
            .body(axum::body::Body::empty())
            .unwrap();
    }

    let mut response = next.run(request).await;
    if let Ok(origin_val) = axum::http::HeaderValue::from_str(&allowed_origin) {
        response
            .headers_mut()
            .insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
    }
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_METHODS,
        axum::http::HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS,
        axum::http::HeaderValue::from_static("content-type, x-request-id, authorization"),
    );
    response
}

pub async fn get_twin(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Result<Json<TelemetryRecord>, StatusCode> {
    if let Some(rec) = state.twins.get(&satellite) {
        Ok(Json(rec.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn get_twin_health(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Result<Json<crate::simulation::TwinHealthProjection>, StatusCode> {
    let rec = state
        .twins
        .get(&satellite)
        .ok_or(StatusCode::NOT_FOUND)?
        .clone();
    let projection = crate::simulation::project_twin_health(satellite, &rec);
    Ok(Json(projection))
}

pub async fn get_twin_timeline(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
    let events = crate::db::get_twin_timeline(&state.db_pool, &satellite)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;
    Ok(Json(events))
}

pub async fn simulate_failure(
    State(state): State<Arc<AppState>>,
    Path(satellite): Path<String>,
    Json(payload): Json<SimulateFailureRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let sub = payload.subsystem.to_lowercase();
    let mut failures = state
        .simulated_failures
        .entry(satellite.clone())
        .or_default();
    if !failures.contains(&sub) {
        failures.push(sub.clone());
    }

    info!(
        satellite = %satellite,
        sub = %sub,
        message = "Digital twin simulated failure injected on subsystem: {}",
        sub.to_uppercase()
    );

    let timeline_payload = serde_json::json!({
        "timestamp": chrono::Utc::now(),
        "satellite_id": satellite,
        "event_type": "SIMULATED_FAILURE_INJECTED",
        "severity": "CRITICAL",
        "message": format!("The Digital twin simulated failure injected on subsystem: {}", sub.to_uppercase()),
        "details": {
            "subsystem": sub
        }
    });

    if let Ok(payload) = serde_json::to_string(&timeline_payload) {
        let record = FutureRecord::to(&state.timeline_topic)
            .key(&satellite)
            .payload(&payload);
        let _ = state
            .kafka_producer
            .send(record, Duration::from_secs(5))
            .await;
    }

    Ok(StatusCode::OK)
}
