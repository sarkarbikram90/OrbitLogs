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
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use tracing::error;

use crate::db;
use crate::metrics;
use crate::session::{api_error, is_valid_speed, ControlReplayRequest, CreateReplayRequest};
use crate::AppState;

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "UP" }))).into_response()
}

pub async fn handle_metrics() -> impl IntoResponse {
    use prometheus::{Encoder, TextEncoder};
    let encoder = TextEncoder::new();
    let metric_families = metrics::REGISTRY.gather();
    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode metrics: {}", e),
        )
            .into_response();
    }

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, encoder.format_type())
        .body(axum::body::Body::from(buffer))
        .unwrap()
        .into_response()
}

pub async fn create_replay(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateReplayRequest>,
) -> impl IntoResponse {
    if payload.start_time >= payload.end_time {
        return api_error(
            StatusCode::BAD_REQUEST,
            "start_time must be before end_time",
        );
    }

    let speed = payload.speed.unwrap_or(1.0);
    if !is_valid_speed(speed) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "Invalid replay speed. Must be 1.0, 5.0, 10.0, 50.0, or 100.0",
        );
    }

    let session_id = uuid::Uuid::new_v4();
    let session = db::ReplaySession {
        session_id,
        satellite_id: payload.satellite_id.clone(),
        start_time: payload.start_time,
        end_time: payload.end_time,
        current_position: payload.start_time,
        speed,
        status: "PAUSED".to_string(),
        created_at: None,
        updated_at: None,
    };

    if let Err(e) = db::create_replay_session(&state.db_pool, &session).await {
        error!("Failed to create replay session in DB: {}", e);
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database failure");
    }

    state.sessions.insert(session_id, session.clone());

    (StatusCode::CREATED, Json(session)).into_response()
}

pub async fn get_replay(
    State(state): State<Arc<AppState>>,
    Path(id_param): Path<String>,
) -> impl IntoResponse {
    let session_id = match uuid::Uuid::parse_str(&id_param) {
        Ok(uid) => uid,
        Err(_) => return api_error(StatusCode::BAD_REQUEST, "Invalid session ID"),
    };

    match db::get_replay_session(&state.db_pool, session_id).await {
        Ok(Some(session)) => (StatusCode::OK, Json(session)).into_response(),
        Ok(None) => api_error(StatusCode::NOT_FOUND, "Replay session not found"),
        Err(e) => {
            error!("Database error fetching session: {}", e);
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        }
    }
}

pub async fn control_replay(
    State(state): State<Arc<AppState>>,
    Path(id_param): Path<String>,
    Json(payload): Json<ControlReplayRequest>,
) -> impl IntoResponse {
    let session_id = match uuid::Uuid::parse_str(&id_param) {
        Ok(uid) => uid,
        Err(_) => return api_error(StatusCode::BAD_REQUEST, "Invalid session ID"),
    };

    let action = payload.action.to_lowercase();
    let valid_actions = ["play", "pause", "stop", "seek", "speed"];
    if !valid_actions.contains(&action.as_str()) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "Invalid action. Must be play, pause, stop, seek, or speed",
        );
    }

    let mut session = match db::get_replay_session(&state.db_pool, session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return api_error(StatusCode::NOT_FOUND, "Replay session not found"),
        Err(e) => {
            error!("Database error fetching session: {}", e);
            return api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error");
        }
    };

    metrics::REPLAY_CONTROLS.with_label_values(&[&action]).inc();

    match action.as_str() {
        "play" => {
            session.status = "PLAYING".to_string();
        }
        "pause" => {
            session.status = "PAUSED".to_string();
        }
        "stop" => {
            session.status = "STOPPED".to_string();
        }
        "seek" => {
            let pos = match payload.position {
                Some(p) => p,
                None => {
                    return api_error(
                        StatusCode::BAD_REQUEST,
                        "position is required for seek action",
                    )
                }
            };
            if pos < session.start_time || pos > session.end_time {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "seek position must be within session start and end times",
                );
            }
            session.current_position = pos;
        }
        "speed" => {
            let speed = match payload.speed {
                Some(s) => s,
                None => {
                    return api_error(
                        StatusCode::BAD_REQUEST,
                        "speed is required for speed action",
                    )
                }
            };
            if !is_valid_speed(speed) {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "Invalid speed. Must be 1.0, 5.0, 10.0, 50.0, or 100.0",
                );
            }
            session.speed = speed;
        }
        _ => unreachable!(),
    }

    if let Err(e) = db::update_replay_session(
        &state.db_pool,
        session.session_id,
        session.current_position,
        &session.status,
        session.speed,
    )
    .await
    {
        error!("Failed to update replay session status in DB: {}", e);
        return api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error updating state",
        );
    }

    state.sessions.insert(session.session_id, session.clone());

    let playing_count: i64 =
        match sqlx::query_scalar("SELECT COUNT(*) FROM replay_sessions WHERE status = 'PLAYING'")
            .fetch_one(&state.db_pool)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                error!("Database error counting playing sessions: {}", e);
                0
            }
        };

    metrics::ACTIVE_REPLAYS.set(playing_count as f64);

    (StatusCode::OK, Json(session)).into_response()
}

pub async fn list_satellites(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match db::query_latest_satellites(&state.db_pool).await {
        Ok(health_records) => {
            let mapped: Vec<db::ReplaySatelliteHealth> = health_records
                .into_iter()
                .map(db::ReplaySatelliteHealth::from)
                .collect();
            (StatusCode::OK, Json(mapped)).into_response()
        }
        Err(e) => {
            error!("Failed to query latest satellite health: {}", e);
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database query failure")
        }
    }
}

pub async fn list_ground_stations(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match db::query_latest_ground_stations(&state.db_pool).await {
        Ok(gs_records) => (StatusCode::OK, Json(gs_records)).into_response(),
        Err(e) => {
            error!("Failed to query latest ground station health: {}", e);
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database query failure")
        }
    }
}

pub async fn list_events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match db::query_recent_events(&state.db_pool).await {
        Ok(event_records) => (StatusCode::OK, Json(event_records)).into_response(),
        Err(e) => {
            error!("Failed to query recent events: {}", e);
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "Database query failure")
        }
    }
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

pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
