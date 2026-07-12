use crate::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
}

#[derive(Debug, Clone, Deserialize)]
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
) -> Result<String, crate::error::AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| {
            crate::error::AppError::Auth("Missing Authorization Bearer token".to_string())
        })?;

    let mut validation = jsonwebtoken::Validation::default();
    validation.validate_exp = true;

    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| crate::error::AppError::Auth(format!("Invalid token: {}", e)))?;

    let role = Role::from_str(&token_data.claims.role)
        .ok_or_else(|| crate::error::AppError::Auth("Invalid role in token claims".to_string()))?;

    if role >= required {
        Ok(token_data.claims.role)
    } else {
        Err(crate::error::AppError::Forbidden(format!(
            "Insufficient permissions: required {:?}, got {:?}",
            required, role
        )))
    }
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    connect_info: Option<axum::extract::ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, crate::error::AppError> {
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
        return Err(crate::error::AppError::TooManyRequests);
    }

    if !orbitlogs_models::auth::secure_compare(&req.api_key, &state.api_key) {
        return Err(crate::error::AppError::Auth("Invalid API key".to_string()));
    }

    let Some(role) = Role::from_str(&req.role) else {
        return Err(crate::error::AppError::Validation(
            "Invalid role specified".to_string(),
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
    .map_err(|e| crate::error::AppError::Internal(format!("Token generation failed: {}", e)))?;

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
        let err = res.unwrap_err();
        assert!(matches!(err, crate::error::AppError::Forbidden(_)));
        assert!(err.to_string().contains("Insufficient permissions"));
    }

    #[tokio::test]
    async fn test_check_role_missing_header() {
        let state = mock_state("secret123");
        let headers = HeaderMap::new();

        let res = check_role(&state, &headers, Role::Operator);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(matches!(err, crate::error::AppError::Auth(_)));
    }

    #[tokio::test]
    async fn test_check_role_expired_token() {
        let state = mock_state("secret123");
        let token = create_token("secret123", "Administrator", -1);
        // expired 1h ago

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        );

        let res = check_role(&state, &headers, Role::Operator);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(matches!(err, crate::error::AppError::Auth(_)));
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
        let err = res.unwrap_err();
        assert!(matches!(err, crate::error::AppError::Auth(_)));
    }

    #[tokio::test]
    async fn test_login_rate_limiting() {
        let state = Arc::new(mock_state("secret123"));
        let headers = HeaderMap::new();
        let req = LoginRequest {
            api_key: "test_api_key".to_string(),
            role: "Operator".to_string(),
        };

        // We have a bucket capacity of 5.0 tokens. Make 5 successful calls.
        for _ in 0..5 {
            let res = login_handler(
                State(state.clone()),
                None,
                headers.clone(),
                Json(req.clone()),
            )
            .await;
            assert!(res.is_ok());
        }

        // The 6th call should immediately exhaust the bucket and fail with TooManyRequests.
        let res = login_handler(
            State(state.clone()),
            None,
            headers.clone(),
            Json(req.clone()),
        )
        .await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(matches!(err, crate::error::AppError::TooManyRequests));
    }
}
