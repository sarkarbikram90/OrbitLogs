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
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_id: Uuid,
    pub source: String, // "TELEMETRY", "COMMAND", "ALERT", "INCIDENT", "GROUND_STATION", "DIGITAL_TWIN"
    pub satellite_id: Option<String>,
    pub event_type: String,
    pub severity: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    pub satellite_id: Option<String>,
    pub source: Option<String>,
    pub severity: Option<String>,
    pub limit: Option<i64>,
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

pub async fn insert_event(pool: &PgPool, evt: &TimelineEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO timeline_events (timestamp, event_id, source, satellite_id, event_type, severity, message, details)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(evt.timestamp)
    .bind(evt.event_id)
    .bind(&evt.source)
    .bind(&evt.satellite_id)
    .bind(&evt.event_type)
    .bind(&evt.severity)
    .bind(&evt.message)
    .bind(&evt.details)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_timeline(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineEvent>>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(100);

    let mut sql = "SELECT * FROM timeline_events WHERE 1=1".to_string();
    let mut param_index = 1;
    let mut bindings = Vec::new();

    if let Some(ref sat_id) = params.satellite_id {
        sql.push_str(&format!(" AND satellite_id = ${}", param_index));
        bindings.push(sat_id.clone());
        param_index += 1;
    }

    if let Some(ref src) = params.source {
        sql.push_str(&format!(" AND source = ${}", param_index));
        bindings.push(src.clone());
        param_index += 1;
    }

    if let Some(ref sev) = params.severity {
        sql.push_str(&format!(" AND severity = ${}", param_index));
        bindings.push(sev.clone());
        param_index += 1;
    }

    sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT ${}", param_index));

    let mut query = sqlx::query(&sql);
    for bind_val in bindings {
        query = query.bind(bind_val);
    }
    query = query.bind(limit);

    let rows = query.fetch_all(&state.db_pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    let events = rows
        .into_iter()
        .map(|row| TimelineEvent {
            timestamp: row.get("timestamp"),
            event_id: row.get("event_id"),
            source: row.get("source"),
            satellite_id: row.get("satellite_id"),
            event_type: row.get("event_type"),
            severity: row.get("severity"),
            message: row.get("message"),
            details: row.get("details"),
        })
        .collect();

    Ok(Json(events))
}

pub async fn create_timeline_event(
    State(state): State<Arc<AppState>>,
    Json(evt): Json<TimelineEvent>,
) -> Result<(StatusCode, Json<TimelineEvent>), (StatusCode, Json<serde_json::Value>)> {
    insert_event(&state.db_pool, &evt).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok((StatusCode::CREATED, Json(evt)))
}
