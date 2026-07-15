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
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Scenario {
    Normal,
    ThermalSpike,
    BatteryDegradation,
    CommunicationDegradation,
    FuelLeakage,
}

impl FromStr for Scenario {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "normal" => Ok(Scenario::Normal),
            "thermal_spike" | "thermal-spike" | "thermal" => Ok(Scenario::ThermalSpike),
            "battery_degradation" | "battery-degradation" | "battery" => {
                Ok(Scenario::BatteryDegradation)
            }
            "communication_degradation" | "communication-degradation" | "comm" => {
                Ok(Scenario::CommunicationDegradation)
            }
            "fuel_leakage" | "fuel-leakage" | "fuel" => Ok(Scenario::FuelLeakage),
            _ => Err(format!("Unknown scenario: {}", s)),
        }
    }
}
