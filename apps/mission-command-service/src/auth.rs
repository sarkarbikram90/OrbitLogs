use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub api_key: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

// Simple RBAC roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Role {
    Operator = 1,
    FlightDirector = 2,
    MissionManager = 3,
    Administrator = 4,
}

impl Role {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "OPERATOR" => Some(Role::Operator),
            "FLIGHTDIRECTOR" | "FLIGHT_DIRECTOR" | "FLIGHT DIRECTOR" => Some(Role::FlightDirector),
            "MISSIONMANAGER" | "MISSION_MANAGER" | "MISSION MANAGER" => Some(Role::MissionManager),
            "ADMINISTRATOR" | "ADMIN" => Some(Role::Administrator),
            _ => None,
        }
    }
}

pub fn check_role(
    state: &AppState,
    headers: &HeaderMap,
    required: Role,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Missing Authorization Bearer token" })),
            )
        })?;

    let mut validation = jsonwebtoken::Validation::default();
    validation.validate_exp = true;

    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": format!("Invalid token: {}", e) })),
        )
    })?;

    let role = Role::from_str(&token_data.claims.role).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid role in token claims" })),
        )
    })?;

    if role >= required {
        Ok(token_data.claims.role)
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": format!("Insufficient permissions: required {:?}, got {:?}", required, role) })),
        ))
    }
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    connect_info: Option<axum::extract::ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            connect_info.map(|axum::extract::ConnectInfo(addr)| addr.ip().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let now = std::time::Instant::now();
    let limit_exceeded = {
        let entry = state.login_rate_limiter.buckets.entry(ip).or_insert_with(|| {
            std::sync::Mutex::new((5.0, now))
        });
        let mut bucket = entry.lock().unwrap();
        let (ref mut tokens, ref mut last_refill) = *bucket;

        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        *tokens = (*tokens + elapsed * (5.0 / 60.0)).min(5.0);
        *last_refill = now;

        if *tokens >= 1.0 {
            *tokens -= 1.0;
            false
        } else {
            true
        }
    };

    if limit_exceeded {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": "Too many login attempts. Please try again later." })),
        ));
    }

    if !orbitlogs_models::auth::secure_compare(&req.api_key, &state.api_key) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid API key" })),
        ));
    }

    let Some(role) = Role::from_str(&req.role) else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid role specified" })),
        ));
    };

    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: "orbitlogs-operator".to_string(),
        role: format!("{:?}", role),
        exp,
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Token generation failed: {}", e) })),
        )
    })?;

    Ok(Json(LoginResponse { token }))
}
