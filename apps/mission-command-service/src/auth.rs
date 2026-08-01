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
use crate::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    #[serde(default = "default_tenant_id")]
    pub tenant_id: String,
    #[serde(default = "default_tenant_slug")]
    pub tenant_slug: String,
    pub role: String,
    pub exp: usize,
    #[serde(default)]
    pub iat: usize,
}

fn default_tenant_id() -> String {
    "00000000-0000-0000-0000-000000000001".to_string()
}

fn default_tenant_slug() -> String {
    "system".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub tenant_id: String,
    pub tenant_slug: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub tenant_id: String,
    pub tenant_slug: String,
    pub token: String,
}

// Simple RBAC roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Role {
    Viewer = 0,
    Operator = 1,
    FlightDirector = 2,
    MissionManager = 3,
    Administrator = 4,
}

impl Role {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "VIEWER" => Some(Role::Viewer),
            "OPERATOR" => Some(Role::Operator),
            "FLIGHTDIRECTOR" | "FLIGHT_DIRECTOR" | "FLIGHT DIRECTOR" => Some(Role::FlightDirector),
            "MISSIONMANAGER" | "MISSION_MANAGER" | "MISSION MANAGER" => Some(Role::MissionManager),
            "ADMINISTRATOR" | "ADMIN" | "TENANTADMIN" | "SUPERADMIN" => Some(Role::Administrator),
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

    let claims: Claims = orbitlogs_models::auth::decode_hs256_token(token, &state.jwt_secret)
        .map_err(|e| crate::error::AppError::Auth(format!("Invalid token: {}", e)))?;

    let role = Role::from_str(&claims.role)
        .ok_or_else(|| crate::error::AppError::Auth("Invalid role in token claims".to_string()))?;

    if role >= required {
        Ok(claims.role)
    } else {
        Err(crate::error::AppError::Forbidden(format!(
            "Insufficient permissions: required {:?}, got {:?}",
            required, role
        )))
    }
}

pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<orbitlogs_models::auth::RegisterRequest>,
) -> Result<(axum::http::StatusCode, Json<RegisterResponse>), crate::error::AppError> {
    if req.tenant_name.trim().is_empty() || req.tenant_slug.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "Tenant name and slug are required".to_string(),
        ));
    }
    if req.email.trim().is_empty() || req.password.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "Email and password are required".to_string(),
        ));
    }

    let tenant_id = uuid::Uuid::new_v4();
    let plan = req.plan_tier.as_deref().unwrap_or("developer");

    sqlx::query(
        "INSERT INTO tenants (id, tenant_name, tenant_slug, status, plan_tier) VALUES ($1, $2, $3, 'ACTIVE', $4)"
    )
    .bind(tenant_id)
    .bind(&req.tenant_name)
    .bind(&req.tenant_slug)
    .bind(plan)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate key") {
            crate::error::AppError::Validation(format!("Tenant slug '{}' is already taken", req.tenant_slug))
        } else {
            crate::error::AppError::Internal(format!("Failed to create tenant: {}", e))
        }
    })?;

    let user_id = uuid::Uuid::new_v4();
    let pass_hash = orbitlogs_models::auth::hash_password(&req.password)
        .map_err(|e| crate::error::AppError::Internal(e))?;

    sqlx::query(
        "INSERT INTO users (id, tenant_id, email, display_name, password_hash, role, status) VALUES ($1, $2, $3, $4, $5, 'Administrator', 'ACTIVE')"
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(&req.email)
    .bind(&req.tenant_name)
    .bind(pass_hash)
    .execute(&state.db_pool)
    .await
    .map_err(|e| crate::error::AppError::Internal(format!("Failed to create user: {}", e)))?;

    let token = orbitlogs_models::auth::create_jwt(
        &state.jwt_secret,
        &req.email,
        tenant_id,
        &req.tenant_slug,
        orbitlogs_models::auth::Role::TenantAdmin,
        state.jwt_expiration_hours,
    )
    .map_err(|e| crate::error::AppError::Internal(e))?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(RegisterResponse {
            tenant_id: tenant_id.to_string(),
            tenant_slug: req.tenant_slug,
            token,
        }),
    ))
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

    // Check if legacy API Key login
    if !req.api_key.is_empty() {
        if !orbitlogs_models::auth::secure_compare(&req.api_key, &state.api_key) {
            return Err(crate::error::AppError::Auth("Invalid API key".to_string()));
        }

        let role_str = if req.role.is_empty() {
            "Operator"
        } else {
            &req.role
        };
        let Some(role) = Role::from_str(role_str) else {
            return Err(crate::error::AppError::Validation(
                "Invalid role specified".to_string(),
            ));
        };

        let default_tenant = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        let token = orbitlogs_models::auth::create_jwt(
            &state.jwt_secret,
            "orbitlogs-operator",
            default_tenant,
            "system",
            orbitlogs_models::auth::Role::from_str_opt(role_str)
                .unwrap_or(orbitlogs_models::auth::Role::Operator),
            state.jwt_expiration_hours,
        )
        .map_err(|e| crate::error::AppError::Internal(format!("Token generation failed: {}", e)))?;

        return Ok(Json(LoginResponse {
            token,
            tenant_id: default_tenant.to_string(),
            tenant_slug: "system".to_string(),
        }));
    }

    // Email/password login flow
    if !req.email.is_empty() && !req.password.is_empty() {
        let row = sqlx::query(
            "SELECT u.tenant_id, u.role, u.password_hash, t.tenant_slug \
             FROM users u JOIN tenants t ON u.tenant_id = t.id \
             WHERE u.email = $1 AND u.status = 'ACTIVE'",
        )
        .bind(&req.email)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("Database query error: {}", e)))?;

        if let Some(row) = row {
            use sqlx::Row;
            let stored_hash: String = row.get("password_hash");

            if !orbitlogs_models::auth::verify_password(&req.password, &stored_hash) {
                return Err(crate::error::AppError::Auth(
                    "Invalid email or password".to_string(),
                ));
            }

            let tenant_id: uuid::Uuid = row.get("tenant_id");
            let role_str: String = row.get("role");
            let tenant_slug: String = row.get("tenant_slug");

            let role_enum = orbitlogs_models::auth::Role::from_str_opt(&role_str)
                .unwrap_or(orbitlogs_models::auth::Role::Operator);

            let token = orbitlogs_models::auth::create_jwt(
                &state.jwt_secret,
                &req.email,
                tenant_id,
                &tenant_slug,
                role_enum,
                state.jwt_expiration_hours,
            )
            .map_err(|e| crate::error::AppError::Internal(e))?;

            return Ok(Json(LoginResponse {
                token,
                tenant_id: tenant_id.to_string(),
                tenant_slug,
            }));
        } else {
            return Err(crate::error::AppError::Auth(
                "Invalid email or password".to_string(),
            ));
        }
    }

    Err(crate::error::AppError::Validation(
        "Either api_key or email+password must be provided".to_string(),
    ))
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
        orbitlogs_models::auth::create_hs256_token(&claims, secret).unwrap()
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
