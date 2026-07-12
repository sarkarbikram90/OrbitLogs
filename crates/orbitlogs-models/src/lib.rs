use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod bounded;
pub mod command_state;
pub mod constellation_iter;
pub mod cow_log;
pub mod domain_error;
pub mod health_broadcast;
pub mod telemetry_view;

pub use domain_error::{DomainError, MissionIdError, SatelliteIdError, StationIdError};

// =========================================================================
// 1. Type-Safe Newtypes — Generated via `define_newtype_id!` (P0 #1, P1 #8)
// =========================================================================
//
// Each newtype:
// - Has a validated `parse()` method returning `Result<Self, Error>` (Parse, Don't Validate)
// - Keeps `new()` for backward compatibility (accepts any string)
// - Has `from_trusted()` as `pub(crate)` for internal/serde construction
// - Inner field is `pub(crate)` not `pub` (P1 #20)
// - Generates Display, Deref, From, sqlx::Type impls

/// Macro that generates validated newtype ID structs with full trait impls.
///
/// This replaces ~150 lines of repetitive code per newtype with a single
/// invocation. Each generated type includes:
/// - `parse()` → `Result<Self, Error>` with prefix validation
/// - `new()` for backward compatibility
/// - `from_trusted()` for `pub(crate)` internal use
/// - `Display`, `Deref`, `From<String>`, `From<&str>`, `Default`, `sqlx::Type`
macro_rules! define_newtype_id {
    ($name:ident, $prefix:literal, $error_name:ident, $default_val:expr) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, sqlx::Type,
        )]
        #[sqlx(transparent)]
        pub struct $name(pub(crate) String);

        impl $name {
            /// Validated constructor: ensures the ID starts with the expected prefix.
            ///
            /// Use this for all user-supplied or untrusted input.
            pub fn parse(raw: impl AsRef<str>) -> Result<Self, domain_error::$error_name> {
                let s = raw.as_ref();
                if !s.starts_with($prefix) {
                    return Err(domain_error::$error_name::InvalidPrefix(s.to_string()));
                }
                Ok(Self(s.to_string()))
            }

            /// Unvalidated constructor for backward compatibility.
            ///
            /// Prefer `parse()` for new code. This accepts any string.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// Internal constructor for trusted sources (serde, DB rows).
            #[allow(dead_code)]
            pub(crate) fn from_trusted(s: String) -> Self {
                Self(s)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self($default_val.to_string())
            }
        }
    };
}

define_newtype_id!(SatelliteId, "SAT-", SatelliteIdError, "");
define_newtype_id!(StationId, "GS-", StationIdError, "");
define_newtype_id!(MissionId, "MISSION-", MissionIdError, "unknown");

// --- SatelliteId-specific methods (outside the macro) ---

impl SatelliteId {
    /// Derives the linked ground station ID deterministically from the satellite ID.
    pub fn linked_ground_station(&self) -> StationId {
        if let Some(num_str) = self.0.strip_prefix("SAT-") {
            if let Ok(num) = num_str.parse::<usize>() {
                let gs_num = ((num - 1) % 4) + 1;
                return StationId::from_trusted(format!("GS-{:04}", gs_num));
            }
        }
        StationId::from_trusted("GS-0001".to_string())
    }
}

// =========================================================================
// 2. Domain Enums
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    Power,
    Thermal,
    Communications,
    Propulsion,
    Adcs,
    Payload,
    FlightComputer,
    Unified,
}

impl Default for Subsystem {
    fn default() -> Self {
        Subsystem::Unified
    }
}

impl std::fmt::Display for Subsystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Subsystem::Power => "power",
            Subsystem::Thermal => "thermal",
            Subsystem::Communications => "communications",
            Subsystem::Propulsion => "propulsion",
            Subsystem::Adcs => "adcs",
            Subsystem::Payload => "payload",
            Subsystem::FlightComputer => "flight_computer",
            Subsystem::Unified => "unified",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for Subsystem {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "power" => Ok(Subsystem::Power),
            "thermal" => Ok(Subsystem::Thermal),
            "communications" | "comms" => Ok(Subsystem::Communications),
            "propulsion" => Ok(Subsystem::Propulsion),
            "adcs" => Ok(Subsystem::Adcs),
            "payload" => Ok(Subsystem::Payload),
            "flight_computer" | "flightcomputer" | "flight computer" => Ok(Subsystem::FlightComputer),
            "unified" => Ok(Subsystem::Unified),
            _ => Err(format!("Unknown subsystem: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StationStatus {
    Online,
    Degraded,
    Outage,
    Maintenance,
}

impl std::fmt::Display for StationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            StationStatus::Online => "ONLINE",
            StationStatus::Degraded => "DEGRADED",
            StationStatus::Outage => "OUTAGE",
            StationStatus::Maintenance => "MAINTENANCE",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for StationStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ONLINE" => Ok(StationStatus::Online),
            "DEGRADED" => Ok(StationStatus::Degraded),
            "OUTAGE" => Ok(StationStatus::Outage),
            "MAINTENANCE" => Ok(StationStatus::Maintenance),
            _ => Err(format!("Unknown station status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeatherState {
    Clear,
    SevereStorm,
    HeavySnow,
}

impl std::fmt::Display for WeatherState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            WeatherState::Clear => "Clear",
            WeatherState::SevereStorm => "SevereStorm",
            WeatherState::HeavySnow => "HeavySnow",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for WeatherState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Clear" => Ok(WeatherState::Clear),
            "SevereStorm" => Ok(WeatherState::SevereStorm),
            "HeavySnow" => Ok(WeatherState::HeavySnow),
            _ => Err(format!("Unknown weather state: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandStatus {
    Created,
    Approved,
    Queued,
    Transmitted,
    Executed,
    Failed,
}

impl std::fmt::Display for CommandStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CommandStatus::Created => "CREATED",
            CommandStatus::Approved => "APPROVED",
            CommandStatus::Queued => "QUEUED",
            CommandStatus::Transmitted => "TRANSMITTED",
            CommandStatus::Executed => "EXECUTED",
            CommandStatus::Failed => "FAILED",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for CommandStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "CREATED" => Ok(CommandStatus::Created),
            "APPROVED" => Ok(CommandStatus::Approved),
            "QUEUED" => Ok(CommandStatus::Queued),
            "TRANSMITTED" => Ok(CommandStatus::Transmitted),
            "EXECUTED" => Ok(CommandStatus::Executed),
            "FAILED" => Ok(CommandStatus::Failed),
            _ => Err(format!("Unknown command status: {}", s)),
        }
    }
}

// Macro to implement sqlx Decode/Encode/Type for string-like enums
macro_rules! impl_sqlx_string_enum {
    ($t:ty) => {
        impl<'r> ::sqlx::Decode<'r, ::sqlx::Postgres> for $t {
            fn decode(value: ::sqlx::postgres::PgValueRef<'r>) -> Result<Self, ::sqlx::error::BoxDynError> {
                let s = <String as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?;
                s.parse::<Self>().map_err(|e| e.into())
            }
        }
        impl<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> for $t {
            fn encode_by_ref(&self, buf: &mut ::sqlx::postgres::PgArgumentBuffer) -> ::sqlx::encode::IsNull {
                let s = self.to_string();
                <String as ::sqlx::Encode<::sqlx::Postgres>>::encode(s, buf)
            }
        }
        impl ::sqlx::Type<::sqlx::Postgres> for $t {
            fn type_info() -> ::sqlx::postgres::PgTypeInfo {
                <String as ::sqlx::Type<::sqlx::Postgres>>::type_info()
            }
        }
    };
}

impl_sqlx_string_enum!(Subsystem);
impl_sqlx_string_enum!(StationStatus);
impl_sqlx_string_enum!(WeatherState);
impl_sqlx_string_enum!(CommandStatus);

// =========================================================================
// 3. Domain Model Structs
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryRecord {
    pub satellite_id: SatelliteId,

    #[serde(default)]
    pub subsystem_id: Subsystem,

    #[serde(default)]
    pub mission_id: MissionId,

    pub timestamp: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_voltage: Option<f64>, // Volts

    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_current: Option<f64>, // Amperes

    #[serde(skip_serializing_if = "Option::is_none")]
    pub solar_panel_output: Option<f64>, // Watts

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_usage: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_usage: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>, // °C

    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_pitch: Option<f32>, // Degrees

    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_roll: Option<f32>, // Degrees

    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation_yaw: Option<f32>, // Degrees

    #[serde(skip_serializing_if = "Option::is_none")]
    pub velocity: Option<f64>, // km/s

    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>, // km

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_remaining: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication_latency: Option<f32>, // ms

    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<f32>, // %

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_strength: Option<f32>, // dBm

    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_power_draw: Option<f64>, // Watts

    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_active: Option<bool>,

    pub event_id: Uuid,
}

impl TelemetryRecord {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.satellite_id.as_str().trim().is_empty() {
            errors.push("satellite_id cannot be empty".to_string());
        } else if !self
            .satellite_id
            .as_str()
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-')
        {
            errors.push("satellite_id must be alphanumeric or contain dashes".to_string());
        }

        if let Some(battery_voltage) = self.battery_voltage {
            if !(0.0..=100.0).contains(&battery_voltage) {
                errors.push(format!(
                    "battery_voltage must be between 0 and 100, got {}",
                    battery_voltage
                ));
            }
        }

        if let Some(cpu_usage) = self.cpu_usage {
            if !(0.0..=100.0).contains(&cpu_usage) {
                errors.push(format!(
                    "cpu_usage must be between 0 and 100, got {}",
                    cpu_usage
                ));
            }
        }

        if let Some(memory_usage) = self.memory_usage {
            if !(0.0..=100.0).contains(&memory_usage) {
                errors.push(format!(
                    "memory_usage must be between 0 and 100, got {}",
                    memory_usage
                ));
            }
        }

        if let Some(fuel_remaining) = self.fuel_remaining {
            if !(0.0..=100.0).contains(&fuel_remaining) {
                errors.push(format!(
                    "fuel_remaining must be between 0 and 100, got {}",
                    fuel_remaining
                ));
            }
        }

        if let Some(packet_loss) = self.packet_loss {
            if !(0.0..=100.0).contains(&packet_loss) {
                errors.push(format!(
                    "packet_loss must be between 0 and 100, got {}",
                    packet_loss
                ));
            }
        }

        if let Some(orientation_pitch) = self.orientation_pitch {
            if !(-360.0..=360.0).contains(&orientation_pitch) {
                errors.push(format!(
                    "orientation_pitch must be between -360 and 360, got {}",
                    orientation_pitch
                ));
            }
        }

        if let Some(orientation_roll) = self.orientation_roll {
            if !(-360.0..=360.0).contains(&orientation_roll) {
                errors.push(format!(
                    "orientation_roll must be between -360 and 360, got {}",
                    orientation_roll
                ));
            }
        }

        if let Some(orientation_yaw) = self.orientation_yaw {
            if !(-360.0..=360.0).contains(&orientation_yaw) {
                errors.push(format!(
                    "orientation_yaw must be between -360 and 360, got {}",
                    orientation_yaw
                ));
            }
        }

        if self.timestamp > Utc::now() + chrono::Duration::minutes(5) {
            errors.push("Timestamp cannot be ahead of current time".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundStationHealth {
    pub timestamp: DateTime<Utc>,
    pub station_id: StationId,
    pub station_name: String,
    pub status: StationStatus,
    pub antenna_health: f32,
    pub weather_state: WeatherState,
    pub congestion: f32,
    pub latency: f32,
    pub bandwidth: f32,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backhaul_health: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_quality: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthState {
    #[serde(rename = "GREEN")]
    Green,
    #[serde(rename = "YELLOW")]
    Yellow,
    #[serde(rename = "RED")]
    Red,
}

impl std::fmt::Display for HealthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthState::Green => write!(f, "GREEN"),
            HealthState::Yellow => write!(f, "YELLOW"),
            HealthState::Red => write!(f, "RED"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubsystemStatus {
    #[serde(rename = "NOMINAL")]
    Nominal,
    #[serde(rename = "DEGRADED")]
    Degraded,
    #[serde(rename = "FAILED")]
    Failed,
}

impl std::fmt::Display for SubsystemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubsystemStatus::Nominal => write!(f, "NOMINAL"),
            SubsystemStatus::Degraded => write!(f, "DEGRADED"),
            SubsystemStatus::Failed => write!(f, "FAILED"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    #[serde(rename = "ERR_UNAUTHORIZED")]
    Unauthorized,
    #[serde(rename = "ERR_FORBIDDEN")]
    Forbidden,
    #[serde(rename = "ERR_NOT_FOUND")]
    NotFound,
    #[serde(rename = "ERR_VALIDATION_FAILED")]
    ValidationFailed,
    #[serde(rename = "ERR_INTERNAL_ERROR")]
    InternalError,
    #[serde(rename = "ERR_RATE_LIMITED")]
    RateLimited,
    #[serde(rename = "ERR_COMMAND_INVALID_STATE")]
    CommandInvalidState,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::Unauthorized => write!(f, "ERR_UNAUTHORIZED"),
            ErrorCode::Forbidden => write!(f, "ERR_FORBIDDEN"),
            ErrorCode::NotFound => write!(f, "ERR_NOT_FOUND"),
            ErrorCode::ValidationFailed => write!(f, "ERR_VALIDATION_FAILED"),
            ErrorCode::InternalError => write!(f, "ERR_INTERNAL_ERROR"),
            ErrorCode::RateLimited => write!(f, "ERR_RATE_LIMITED"),
            ErrorCode::CommandInvalidState => write!(f, "ERR_COMMAND_INVALID_STATE"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            trace_id: None,
        }
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

pub fn load_secret(env_var: &str) -> Option<String> {
    if let Ok(val) = std::env::var(env_var) {
        if !val.trim().is_empty() {
            return Some(val.trim().to_string());
        }
    }
    if let Ok(file_path) = std::env::var(format!("{}_FILE", env_var)) {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            return Some(content.trim().to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecondaryObjectType {
    Debris,
    ActiveSatellite,
    RocketBody,
    Unknown,
}

impl std::fmt::Display for SecondaryObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SecondaryObjectType::Debris => "DEBRIS",
            SecondaryObjectType::ActiveSatellite => "SATELLITE",
            SecondaryObjectType::RocketBody => "ROCKET_BODY",
            SecondaryObjectType::Unknown => "UNKNOWN",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for SecondaryObjectType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "DEBRIS" | "DEB" => Ok(SecondaryObjectType::Debris),
            "SATELLITE" | "ACTIVE_SATELLITE" | "ACTIVESATELLITE" => Ok(SecondaryObjectType::ActiveSatellite),
            "ROCKET_BODY" | "ROCKETBODY" => Ok(SecondaryObjectType::RocketBody),
            "UNKNOWN" => Ok(SecondaryObjectType::Unknown),
            _ => Err(format!("Unknown secondary object type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConjunctionRiskTier {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ConjunctionRiskTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ConjunctionRiskTier::Low => "LOW",
            ConjunctionRiskTier::Medium => "MEDIUM",
            ConjunctionRiskTier::High => "HIGH",
            ConjunctionRiskTier::Critical => "CRITICAL",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for ConjunctionRiskTier {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "LOW" => Ok(ConjunctionRiskTier::Low),
            "MEDIUM" => Ok(ConjunctionRiskTier::Medium),
            "HIGH" => Ok(ConjunctionRiskTier::High),
            "CRITICAL" => Ok(ConjunctionRiskTier::Critical),
            _ => Err(format!("Unknown conjunction risk tier: {}", s)),
        }
    }
}

impl_sqlx_string_enum!(SecondaryObjectType);
impl_sqlx_string_enum!(ConjunctionRiskTier);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConjunctionEvent {
    pub conjunction_id: Uuid,
    pub primary_satellite_id: SatelliteId,
    pub secondary_object_id: String,
    pub secondary_object_type: SecondaryObjectType,
    pub tca: DateTime<Utc>,
    pub miss_distance_km: f64,
    pub radial_miss_m: f64,
    pub intrack_miss_m: f64,
    pub crosstrack_miss_m: f64,
    pub collision_probability: f64,
    pub relative_velocity_kms: f64,
    pub risk_tier: ConjunctionRiskTier,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionImpactSummary {
    pub propellant_cost_kg: f64,
    pub payload_downtime_minutes: u32,
    pub post_burn_orbit_apogee_km: f64,
    pub post_burn_orbit_perigee_km: f64,
    pub downlink_window_disrupted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvoidanceManeuverRecommendation {
    pub recommendation_id: Uuid,
    pub conjunction_id: Uuid,
    pub satellite_id: SatelliteId,
    pub burn_execution_window_start: DateTime<Utc>,
    pub delta_v_vector: [f64; 3],
    pub post_burn_miss_distance_km: f64,
    pub post_burn_pc: f64,
    pub mission_impact: MissionImpactSummary,
    pub recommended_action: String,
    pub rationale: String,
    pub timestamp: DateTime<Utc>,
}

// =========================================================================
// 4. Unified TLE Parsing
// =========================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TleElements {
    pub inclination: f64,
    pub raan: f64,
    pub eccentricity: f64,
    pub arg_of_perigee: f64,
    pub mean_anomaly: f64,
    pub mean_motion: f64,
}

impl TleElements {
    pub fn parse(line2: &str) -> Self {
        let inclination = if line2.len() >= 16 {
            line2[8..16].trim().parse::<f64>().unwrap_or(53.0)
        } else {
            53.0
        };
        let raan = if line2.len() >= 25 {
            line2[17..25].trim().parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        };
        let ecc = if line2.len() >= 33 {
            let ecc_str = line2[26..33].trim();
            format!("0.{}", ecc_str).parse::<f64>().unwrap_or(0.0001)
        } else {
            0.0001
        };
        let arg_of_perigee = if line2.len() >= 42 {
            line2[34..42].trim().parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        };
        let mean_anomaly = if line2.len() >= 51 {
            line2[43..51].trim().parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        };
        let mean_motion = if line2.len() >= 63 {
            line2[52..63].trim().parse::<f64>().unwrap_or(15.0)
        } else {
            15.0
        };

        Self {
            inclination,
            raan,
            eccentricity: ecc,
            arg_of_perigee,
            mean_anomaly,
            mean_motion,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MissionPlanStatus {
    Draft,
    Review,
    Approved,
    Scheduled,
    Executing,
    Completed,
    Cancelled,
}

impl std::fmt::Display for MissionPlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MissionPlanStatus::Draft => "DRAFT",
            MissionPlanStatus::Review => "REVIEW",
            MissionPlanStatus::Approved => "APPROVED",
            MissionPlanStatus::Scheduled => "SCHEDULED",
            MissionPlanStatus::Executing => "EXECUTING",
            MissionPlanStatus::Completed => "COMPLETED",
            MissionPlanStatus::Cancelled => "CANCELLED",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for MissionPlanStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "DRAFT" => Ok(MissionPlanStatus::Draft),
            "REVIEW" => Ok(MissionPlanStatus::Review),
            "APPROVED" => Ok(MissionPlanStatus::Approved),
            "SCHEDULED" => Ok(MissionPlanStatus::Scheduled),
            "EXECUTING" => Ok(MissionPlanStatus::Executing),
            "COMPLETED" => Ok(MissionPlanStatus::Completed),
            "CANCELLED" => Ok(MissionPlanStatus::Cancelled),
            _ => Err(format!("Unknown mission plan status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MissionPlanType {
    #[serde(rename = "Imaging")]
    Imaging,
    #[serde(rename = "Communications")]
    Communications,
    #[serde(rename = "Calibration")]
    Calibration,
    #[serde(rename = "Orbit Maneuver")]
    OrbitManeuver,
    #[serde(rename = "Payload Test")]
    PayloadTest,
    #[serde(rename = "Software Update")]
    SoftwareUpdate,
}

impl std::fmt::Display for MissionPlanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MissionPlanType::Imaging => "Imaging",
            MissionPlanType::Communications => "Communications",
            MissionPlanType::Calibration => "Calibration",
            MissionPlanType::OrbitManeuver => "Orbit Maneuver",
            MissionPlanType::PayloadTest => "Payload Test",
            MissionPlanType::SoftwareUpdate => "Software Update",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for MissionPlanType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Imaging" => Ok(MissionPlanType::Imaging),
            "Communications" => Ok(MissionPlanType::Communications),
            "Calibration" => Ok(MissionPlanType::Calibration),
            "Orbit Maneuver" => Ok(MissionPlanType::OrbitManeuver),
            "Payload Test" => Ok(MissionPlanType::PayloadTest),
            "Software Update" => Ok(MissionPlanType::SoftwareUpdate),
            _ => Err(format!("Unknown mission plan type: {}", s)),
        }
    }
}

impl_sqlx_string_enum!(MissionPlanStatus);
impl_sqlx_string_enum!(MissionPlanType);

pub mod auth;
pub mod db;
pub mod kafka;
pub mod telemetry;

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_valid_record() -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: SatelliteId::new("SAT-0001"),
            subsystem_id: Subsystem::Unified,
            mission_id: MissionId::new("unknown"),
            timestamp: Utc::now(),
            battery_voltage: Some(13.8),
            battery_current: Some(2.1),
            solar_panel_output: Some(110.0),
            cpu_usage: Some(25.4),
            memory_usage: Some(45.1),
            temperature: Some(28.5),
            orientation_pitch: Some(1.2),
            orientation_roll: Some(-2.3),
            orientation_yaw: Some(0.5),
            velocity: Some(7.67),
            altitude: Some(520.4),
            fuel_remaining: Some(92.5),
            communication_latency: Some(35.1),
            packet_loss: Some(0.1),
            signal_strength: Some(-68.0),
            payload_power_draw: Some(15.0),
            payload_active: Some(true),
            event_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_valid_payload() {
        let rec = mock_valid_record();
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_invalid_satellite_id() {
        let mut rec = mock_valid_record();
        rec.satellite_id = SatelliteId::new("");
        assert!(rec.validate().is_err());

        rec.satellite_id = SatelliteId::new("SAT#01");
        assert!(rec.validate().is_err());
    }

    #[test]
    fn test_invalid_voltages() {
        let mut rec = mock_valid_record();
        rec.battery_voltage = Some(-1.2);
        assert!(rec.validate().is_err());

        rec.battery_voltage = Some(101.5);
        assert!(rec.validate().is_err());
    }

    #[test]
    fn test_invalid_usages() {
        let mut rec = mock_valid_record();
        rec.cpu_usage = Some(105.0);
        assert!(rec.validate().is_err());

        rec.memory_usage = Some(-5.0);
        assert!(rec.validate().is_err());
    }

    #[test]
    fn test_serde_deny_unknown_fields() {
        let json_with_extra = r#"{
            "satellite_id": "SAT-0001",
            "subsystem_id": "unified",
            "mission_id": "unknown",
            "timestamp": "2026-01-01T00:00:00Z",
            "battery_voltage": 13.8,
            "battery_current": 2.1,
            "solar_panel_output": 110.0,
            "cpu_usage": 25.4,
            "memory_usage": 45.1,
            "temperature": 28.5,
            "orientation_pitch": 1.2,
            "orientation_roll": -2.3,
            "orientation_yaw": 0.5,
            "velocity": 7.67,
            "altitude": 520.4,
            "fuel_remaining": 92.5,
            "communication_latency": 35.1,
            "packet_loss": 0.1,
            "signal_strength": -68.0,
            "payload_power_draw": 15.0,
            "payload_active": true,
            "event_id": "550e8400-e29b-41d4-a716-446655440000",
            "unknown_field": "should_be_rejected"
        }"#;

        let result: Result<TelemetryRecord, _> = serde_json::from_str(json_with_extra);
        assert!(result.is_err(), "Unknown fields must be rejected");
    }

    #[test]
    fn test_serde_roundtrip() {
        let rec = mock_valid_record();
        let json = serde_json::to_string(&rec).unwrap();
        let deserialized: TelemetryRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.satellite_id, deserialized.satellite_id);
        assert_eq!(rec.battery_voltage, deserialized.battery_voltage);
        assert_eq!(rec.temperature, deserialized.temperature);
        assert_eq!(rec.subsystem_id, deserialized.subsystem_id);
        assert_eq!(rec.mission_id, deserialized.mission_id);
    }

    #[test]
    fn test_serde_defaults() {
        let json_minimal = r#"{
            "satellite_id": "SAT-0001",
            "timestamp": "2026-01-01T00:00:00Z",
            "event_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;

        let deserialized: TelemetryRecord = serde_json::from_str(json_minimal).unwrap();
        assert_eq!(deserialized.subsystem_id, Subsystem::Unified);
        assert_eq!(deserialized.mission_id, MissionId::new("unknown"));
        assert!(deserialized.battery_voltage.is_none());
    }

    #[test]
    fn test_anomaly_threshold_thermal_spike() {
        let mut rec = mock_valid_record();
        rec.temperature = Some(86.0);
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_anomaly_threshold_battery_degradation() {
        let mut rec = mock_valid_record();
        rec.battery_voltage = Some(10.5);
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_anomaly_threshold_fuel_low() {
        let mut rec = mock_valid_record();
        rec.fuel_remaining = Some(14.0);
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn test_anomaly_threshold_comm_degradation() {
        let mut rec = mock_valid_record();
        rec.packet_loss = Some(12.0);
        rec.communication_latency = Some(1200.0);
        assert!(rec.validate().is_ok());
    }

    // --- Newtype validation tests (P0 #1) ---

    #[test]
    fn satellite_id_parse_valid() {
        let id = SatelliteId::parse("SAT-0001").unwrap();
        assert_eq!(id.as_str(), "SAT-0001");
    }

    #[test]
    fn satellite_id_parse_rejects_bad_prefix() {
        assert!(SatelliteId::parse("").is_err());
        assert!(SatelliteId::parse("SAT0001").is_err());
        assert!(SatelliteId::parse("SATELLITE-0001").is_err());
        assert!(SatelliteId::parse("DROP TABLE satellites").is_err());
    }

    #[test]
    fn station_id_parse_valid() {
        let id = StationId::parse("GS-0001").unwrap();
        assert_eq!(id.as_str(), "GS-0001");
    }

    #[test]
    fn station_id_parse_rejects_bad_prefix() {
        assert!(StationId::parse("").is_err());
        assert!(StationId::parse("GROUND-01").is_err());
    }

    #[test]
    fn mission_id_parse_valid() {
        let id = MissionId::parse("MISSION-ALPHA").unwrap();
        assert_eq!(id.as_str(), "MISSION-ALPHA");
    }

    #[test]
    fn mission_id_parse_rejects_bad_prefix() {
        assert!(MissionId::parse("").is_err());
        assert!(MissionId::parse("TASK-1").is_err());
    }

    #[test]
    fn newtype_backward_compat_new() {
        // SatelliteId::new() still works for backward compatibility
        let id = SatelliteId::new("any-string");
        assert_eq!(id.as_str(), "any-string");
    }

    #[test]
    fn domain_error_conversion_chain() {
        // Demonstrates: SatelliteIdError -> DomainError via #[from]
        let result: Result<SatelliteId, DomainError> =
            SatelliteId::parse("INVALID").map_err(|e| e.into());
        assert!(result.is_err());
    }
}

// =========================================================================
// 10. Test Support — Mock Implementations (P1 #9)
// =========================================================================

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::sync::Mutex;

    /// A mock telemetry store that captures all inserted records for assertion.
    ///
    /// # Usage
    /// ```ignore
    /// let store = MockTelemetryStore::new();
    /// // ... run your processor with &store ...
    /// assert_eq!(store.inserted_count(), 5);
    /// let records = store.drain();
    /// assert_eq!(records[0].satellite_id.as_str(), "SAT-0001");
    /// ```
    pub struct MockTelemetryStore {
        pub inserted: Mutex<Vec<TelemetryRecord>>,
    }

    impl MockTelemetryStore {
        pub fn new() -> Self {
            Self {
                inserted: Mutex::new(Vec::new()),
            }
        }

        /// Record a telemetry insert (simulates TelemetryStore::insert_telemetry).
        pub fn record_insert(&self, record: &TelemetryRecord) {
            self.inserted.lock().unwrap().push(record.clone());
        }

        /// Return the number of inserts captured.
        pub fn inserted_count(&self) -> usize {
            self.inserted.lock().unwrap().len()
        }

        /// Drain all captured records for assertion.
        pub fn drain(&self) -> Vec<TelemetryRecord> {
            std::mem::take(&mut *self.inserted.lock().unwrap())
        }
    }

    /// Helper to create a valid TelemetryRecord for testing.
    pub fn mock_telemetry_record(sat_id: &str) -> TelemetryRecord {
        TelemetryRecord {
            satellite_id: SatelliteId::new(sat_id),
            subsystem_id: Subsystem::Unified,
            mission_id: MissionId::new("unknown"),
            timestamp: Utc::now(),
            battery_voltage: Some(13.8),
            battery_current: Some(2.1),
            solar_panel_output: Some(110.0),
            cpu_usage: Some(25.4),
            memory_usage: Some(45.1),
            temperature: Some(28.5),
            orientation_pitch: Some(1.2),
            orientation_roll: Some(-2.3),
            orientation_yaw: Some(0.5),
            velocity: Some(7.67),
            altitude: Some(520.4),
            fuel_remaining: Some(92.5),
            communication_latency: Some(35.1),
            packet_loss: Some(0.1),
            signal_strength: Some(-68.0),
            payload_power_draw: Some(15.0),
            payload_active: Some(true),
            event_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn mock_store_captures_inserts() {
        let store = MockTelemetryStore::new();
        let record = mock_telemetry_record("SAT-0001");
        store.record_insert(&record);
        store.record_insert(&record);
        assert_eq!(store.inserted_count(), 2);

        let drained = store.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(store.inserted_count(), 0);
    }
}

