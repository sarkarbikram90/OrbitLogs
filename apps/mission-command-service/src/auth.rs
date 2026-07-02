use crate::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

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
    #[allow(clippy::should_implement_trait)]
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

impl std::str::FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s).ok_or(())
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
            Json(
                serde_json::json!({ "error": format!("Insufficient permissions: required {:?}, got {:?}", required, role) }),
            ),
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
        .or_else(|| connect_info.map(|axum::extract::ConnectInfo(addr)| addr.ip().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let now = std::time::Instant::now();
    let limit_exceeded = {
        let entry = state
            .login_rate_limiter
            .buckets
            .entry(ip)
            .or_insert_with(|| std::sync::Mutex::new((5.0, now)));
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
            Json(
                serde_json::json!({ "error": "Too many login attempts. Please try again later." }),
            ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use rdkafka::producer::FutureProducer;

    fn mock_state(secret: &str) -> AppState {
        // Create mock AppState without real DB or Kafka connection
        let mut producer_config = rdkafka::config::ClientConfig::new();
        producer_config.set("bootstrap.servers", "localhost:9092");
        let producer: FutureProducer = producer_config.create().unwrap();

        AppState {
            db_pool: sqlx::Pool::connect_lazy("postgres://localhost/dummy").unwrap(),
            kafka_producer: producer,
            command_topic: "command.events".to_string(),
            timeline_topic: "timeline.events".to_string(),
            api_key: "test_api_key".to_string(),
            jwt_secret: secret.to_string(),
            login_rate_limiter: crate::LoginRateLimiter {
                buckets: dashmap::DashMap::new(),
            },
        }
    }

    fn create_token(secret: &str, role_str: &str, exp_hours: i64) -> String {
        let exp = (chrono::Utc::now() + chrono::Duration::hours(exp_hours)).timestamp() as usize;
        let claims = Claims {
            sub: "test-user".to_string(),
            role: role_str.to_string(),
            exp,
        };
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_check_role_success() {
        let state = mock_state("secret123");
        let token = create_token("secret123", "FlightDirector", 1);

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        );

        let res = check_role(&state, &headers, Role::Operator);
        assert!(res.is_ok());

        let res2 = check_role(&state, &headers, Role::FlightDirector);
        assert!(res2.is_ok());
    }

    #[tokio::test]
    async fn test_check_role_insufficient_permissions() {
        let state = mock_state("secret123");
        let token = create_token("secret123", "Operator", 1);

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        );

        let res = check_role(&state, &headers, Role::FlightDirector);
        assert!(res.is_err());
        let (status, err) = res.unwrap_err();
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(err.to_string().contains("Insufficient permissions"));
    }

    #[tokio::test]
    async fn test_check_role_missing_header() {
        let state = mock_state("secret123");
        let headers = HeaderMap::new();

        let res = check_role(&state, &headers, Role::Operator);
        assert!(res.is_err());
        let (status, _) = res.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_check_role_expired_token() {
        let state = mock_state("secret123");
        let token = create_token("secret123", "Administrator", -1); // expired 1h ago

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        );

        let res = check_role(&state, &headers, Role::Operator);
        assert!(res.is_err());
        let (status, _) = res.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_check_role_invalid_secret() {
        let state = mock_state("correct_secret");
        let token = create_token("wrong_secret", "Administrator", 1);

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        );

        let res = check_role(&state, &headers, Role::Operator);
        assert!(res.is_err());
        let (status, _) = res.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
