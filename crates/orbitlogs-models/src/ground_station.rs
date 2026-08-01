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

use crate::{StationId, StationStatus, WeatherState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
