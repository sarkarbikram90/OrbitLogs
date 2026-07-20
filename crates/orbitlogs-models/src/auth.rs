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
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

// =========================================================================
// Constant-Time Comparison
// =========================================================================

/// Hashes both inputs with SHA-256 and compares the resulting digests
/// in constant time to reduce timing side channels during equality checks.
///
/// Note: hashing itself is not constant-time with respect to input length.
pub fn secure_compare(provided: &str, expected: &str) -> bool {
    let provided_hash = Sha256::digest(provided.as_bytes());
    let expected_hash = Sha256::digest(expected.as_bytes());

    bool::from(provided_hash.ct_eq(&expected_hash))
}

// =========================================================================
// RBAC Roles
// =========================================================================

/// Role hierarchy for multi-tenant RBAC. Higher numeric value = more permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Role {
    Viewer = 0,
    Operator = 1,
    FlightDirector = 2,
    MissionManager = 3,
    TenantAdmin = 4,
    SuperAdmin = 5,
}

impl Role {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_uppercase().replace(' ', "_").as_str() {
            "VIEWER" => Some(Role::Viewer),
            "OPERATOR" => Some(Role::Operator),
            "FLIGHTDIRECTOR" | "FLIGHT_DIRECTOR" => Some(Role::FlightDirector),
            "MISSIONMANAGER" | "MISSION_MANAGER" => Some(Role::MissionManager),
            "TENANTADMIN" | "TENANT_ADMIN" => Some(Role::TenantAdmin),
            "SUPERADMIN" | "SUPER_ADMIN" | "ADMINISTRATOR" | "ADMIN" => Some(Role::SuperAdmin),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Viewer => "Viewer",
            Role::Operator => "Operator",
            Role::FlightDirector => "FlightDirector",
            Role::MissionManager => "MissionManager",
            Role::TenantAdmin => "TenantAdmin",
            Role::SuperAdmin => "SuperAdmin",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_opt(s).ok_or_else(|| format!("Unknown role: {}", s))
    }
}

// =========================================================================
// JWT Claims with Tenant Context
// =========================================================================

/// JWT claims structure for multi-tenant authentication.
/// Embedded in every Bearer token issued by the auth service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user email or service identifier
    pub sub: String,
    /// Tenant UUID — scopes all data access
    pub tenant_id: String,
    /// Tenant slug — human-readable org identifier
    pub tenant_slug: String,
    /// User's assigned RBAC role
    pub role: String,
    /// Token expiration (Unix timestamp)
    pub exp: usize,
    /// Token issued at (Unix timestamp)
    #[serde(default)]
    pub iat: usize,
}

// =========================================================================
// Tenant Context — Extracted from validated JWT
// =========================================================================

/// Carrier for authenticated request context. Extracted from JWT by middleware
/// and made available to all request handlers via Axum extensions.
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub tenant_slug: String,
    pub user_email: String,
    pub role: Role,
}

impl TenantContext {
    /// Check if the user has at least the required role level.
    pub fn require_role(&self, required: Role) -> Result<(), String> {
        if self.role >= required {
            Ok(())
        } else {
            Err(format!(
                "Insufficient permissions: required {:?}, got {:?}",
                required, self.role
            ))
        }
    }

    /// Returns the tenant_id as a string for SQL query binding.
    pub fn tenant_id_str(&self) -> String {
        self.tenant_id.to_string()
    }
}

/// Validates a JWT token string and extracts the TenantContext.
///
/// # Arguments
/// * `token` - The raw JWT token string (without "Bearer " prefix)
/// * `secret` - The JWT signing secret
///
/// # Returns
/// `Ok(TenantContext)` on success, `Err(String)` on validation failure.
pub fn validate_jwt(token: &str, secret: &str) -> Result<TenantContext, String> {
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| format!("Invalid token: {}", e))?;

    let claims = token_data.claims;

    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|e| format!("Invalid tenant_id in token: {}", e))?;

    let role = Role::from_str_opt(&claims.role)
        .ok_or_else(|| format!("Invalid role in token: {}", claims.role))?;

    Ok(TenantContext {
        tenant_id,
        tenant_slug: claims.tenant_slug,
        user_email: claims.sub,
        role,
    })
}

/// Creates a signed JWT token for a user.
pub fn create_jwt(
    secret: &str,
    sub: &str,
    tenant_id: Uuid,
    tenant_slug: &str,
    role: Role,
    expiration_hours: i64,
) -> Result<String, String> {
    let now = Utc::now();
    let exp = now
        .checked_add_signed(chrono::Duration::hours(expiration_hours))
        .ok_or("Failed to compute expiration")?
        .timestamp() as usize;

    let claims = Claims {
        sub: sub.to_string(),
        tenant_id: tenant_id.to_string(),
        tenant_slug: tenant_slug.to_string(),
        role: role.as_str().to_string(),
        exp,
        iat: now.timestamp() as usize,
    };

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| format!("Token generation failed: {}", e))
}

// =========================================================================
// Tenant & User Models (for DB queries)
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub tenant_name: String,
    pub tenant_slug: String,
    pub status: String,
    pub plan_tier: String,
    pub max_satellites: i32,
    pub max_ground_stations: i32,
    pub contact_email: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Registration request for creating a new tenant + admin user.
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub tenant_name: String,
    pub tenant_slug: String,
    pub email: String,
    pub password: String,
    pub plan_tier: Option<String>,
}

/// Login request supporting email/password or API key authentication.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
    /// Legacy API key field for backward compatibility
    #[serde(default)]
    pub api_key: String,
    /// Role context for legacy API key login
    #[serde(default)]
    pub role: String,
}

// =========================================================================
// Registered Fleet Models
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredSatellite {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub satellite_id: String,
    pub satellite_name: String,
    pub norad_id: Option<i32>,
    pub orbit_type: String,
    pub tle_line1: Option<String>,
    pub tle_line2: Option<String>,
    pub tle_epoch: Option<DateTime<Utc>>,
    pub launch_date: Option<DateTime<Utc>>,
    pub mission_type: Option<String>,
    pub mass_kg: Option<f32>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSatelliteRequest {
    pub satellite_id: String,
    pub satellite_name: String,
    pub norad_id: Option<i32>,
    pub orbit_type: Option<String>,
    pub tle_line1: Option<String>,
    pub tle_line2: Option<String>,
    pub launch_date: Option<String>,
    pub mission_type: Option<String>,
    pub mass_kg: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredGroundStation {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub station_id: String,
    pub station_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub antenna_type: Option<String>,
    pub frequency_band: Option<String>,
    pub min_elevation_deg: f64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateGroundStationRequest {
    pub station_id: String,
    pub station_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: Option<f64>,
    pub antenna_type: Option<String>,
    pub frequency_band: Option<String>,
    pub min_elevation_deg: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_compare_equal() {
        assert!(secure_compare("test_key", "test_key"));
    }

    #[test]
    fn test_secure_compare_not_equal() {
        assert!(!secure_compare("test_key", "wrong_key"));
    }

    #[test]
    fn test_role_hierarchy() {
        assert!(Role::SuperAdmin > Role::TenantAdmin);
        assert!(Role::TenantAdmin > Role::MissionManager);
        assert!(Role::MissionManager > Role::FlightDirector);
        assert!(Role::FlightDirector > Role::Operator);
        assert!(Role::Operator > Role::Viewer);
    }

    #[test]
    fn test_role_parsing() {
        assert_eq!(Role::from_str_opt("Operator"), Some(Role::Operator));
        assert_eq!(Role::from_str_opt("FLIGHT_DIRECTOR"), Some(Role::FlightDirector));
        assert_eq!(Role::from_str_opt("ADMIN"), Some(Role::SuperAdmin));
        assert_eq!(Role::from_str_opt("TenantAdmin"), Some(Role::TenantAdmin));
        assert_eq!(Role::from_str_opt("invalid"), None);
    }

    #[test]
    fn test_tenant_context_require_role() {
        let ctx = TenantContext {
            tenant_id: Uuid::new_v4(),
            tenant_slug: "test-org".to_string(),
            user_email: "user@test.com".to_string(),
            role: Role::FlightDirector,
        };

        assert!(ctx.require_role(Role::Viewer).is_ok());
        assert!(ctx.require_role(Role::Operator).is_ok());
        assert!(ctx.require_role(Role::FlightDirector).is_ok());
        assert!(ctx.require_role(Role::MissionManager).is_err());
        assert!(ctx.require_role(Role::SuperAdmin).is_err());
    }

    #[test]
    fn test_jwt_roundtrip() {
        let secret = "test-jwt-secret-key-for-unit-testing";
        let tenant_id = Uuid::new_v4();

        let token = create_jwt(
            secret,
            "test@pixxel.space",
            tenant_id,
            "pixxel",
            Role::FlightDirector,
            1,
        )
        .expect("Token creation should succeed");

        let ctx = validate_jwt(&token, secret).expect("Token validation should succeed");

        assert_eq!(ctx.tenant_id, tenant_id);
        assert_eq!(ctx.tenant_slug, "pixxel");
        assert_eq!(ctx.user_email, "test@pixxel.space");
        assert_eq!(ctx.role, Role::FlightDirector);
    }

    #[test]
    fn test_jwt_expired_token() {
        let secret = "test-secret";
        let tenant_id = Uuid::new_v4();

        // Create a token that expired 1 hour ago
        let token = create_jwt(secret, "user@test.com", tenant_id, "test", Role::Operator, -1)
            .expect("Token creation should succeed");

        let result = validate_jwt(&token, secret);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid token"));
    }

    #[test]
    fn test_jwt_wrong_secret() {
        let tenant_id = Uuid::new_v4();
        let token = create_jwt("correct-secret", "user@test.com", tenant_id, "test", Role::Operator, 1)
            .expect("Token creation should succeed");

        let result = validate_jwt(&token, "wrong-secret");
        assert!(result.is_err());
    }
}
