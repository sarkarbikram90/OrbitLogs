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

pub mod alert;
pub mod auth;
pub mod bounded;
pub mod command_state;
pub mod constellation_iter;
pub mod cow_log;
pub mod db;
pub mod domain_error;
pub mod enums;
pub mod ground_station;
pub mod health_broadcast;
pub mod ids;
pub mod kafka;
pub mod rate_limit;
pub mod telemetry;
pub mod telemetry_record;
pub mod telemetry_view;
pub mod tests;

// Re-export all public domain types for 100% backward compatibility across workspace
pub use alert::{
    load_secret, ApiError, AvoidanceManeuverRecommendation, ConjunctionEvent, MissionImpactSummary,
    TleElements,
};
pub use domain_error::{DomainError, MissionIdError, SatelliteIdError, StationIdError};
pub use enums::{
    CommandStatus, ConjunctionRiskTier, ErrorCode, HealthState, MissionPlanStatus, MissionPlanType,
    SecondaryObjectType, StationStatus, Subsystem, SubsystemStatus, WeatherState,
};
pub use ground_station::GroundStationHealth;
pub use ids::{MissionId, SatelliteId, StationId};
pub use rate_limit::{PlanTier, TenantRateLimiter};
pub use telemetry_record::TelemetryRecord;
pub use tests::test_support;
