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

use serde::{Deserialize, Serialize};

// Macro to implement sqlx Decode/Encode/Type for string-like enums
macro_rules! impl_sqlx_string_enum {
    ($t:ty) => {
        impl<'r> ::sqlx::Decode<'r, ::sqlx::Postgres> for $t {
            fn decode(
                value: ::sqlx::postgres::PgValueRef<'r>,
            ) -> Result<Self, ::sqlx::error::BoxDynError> {
                let s = <String as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?;
                s.parse::<Self>().map_err(|e| e.into())
            }
        }
        impl<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> for $t {
            fn encode_by_ref(
                &self,
                buf: &mut ::sqlx::postgres::PgArgumentBuffer,
            ) -> ::sqlx::encode::IsNull {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    Power,
    Thermal,
    Communications,
    Propulsion,
    Adcs,
    Payload,
    FlightComputer,
    #[default]
    Unified,
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
            "flight_computer" | "flightcomputer" | "flight computer" => {
                Ok(Subsystem::FlightComputer)
            }
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
            "SATELLITE" | "ACTIVE_SATELLITE" | "ACTIVESATELLITE" => {
                Ok(SecondaryObjectType::ActiveSatellite)
            }
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

impl_sqlx_string_enum!(Subsystem);
impl_sqlx_string_enum!(StationStatus);
impl_sqlx_string_enum!(WeatherState);
impl_sqlx_string_enum!(CommandStatus);
impl_sqlx_string_enum!(SecondaryObjectType);
impl_sqlx_string_enum!(ConjunctionRiskTier);
impl_sqlx_string_enum!(MissionPlanStatus);
impl_sqlx_string_enum!(MissionPlanType);
