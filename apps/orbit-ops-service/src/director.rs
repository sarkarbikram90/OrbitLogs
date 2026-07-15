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
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorRecommendation {
    pub category: String, // 'CRITICAL', 'WARNING', 'OPTIMIZATION'
    pub title: String,
    pub description: String,
    pub reference_runbook: String,
    pub proposed_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorStatus {
    pub satellite_id: orbitlogs_models::SatelliteId,
    pub operational_status: String, // 'NOMINAL', 'DEGRADED', 'SAFE_MODE'
    pub recommendations: Vec<DirectorRecommendation>,
}

pub struct DirectorManager {
    pub active_recommendations: DashMap<String, Vec<DirectorRecommendation>>,
}

impl DirectorManager {
    pub fn new() -> Self {
        Self {
            active_recommendations: DashMap::new(),
        }
    }

    pub fn evaluate_telemetry(
        &self,
        sat_id: &str,
        temp: f64,
        battery_v: f64,
        packet_loss: f64,
        fuel: f64,
    ) {
        let mut recs = Vec::new();

        if temp > 60.0 {
            recs.push(DirectorRecommendation {
                category: "CRITICAL".to_string(),
                title: "TCS Thermal Overload Detected".to_string(),
                description: format!("Cpu/Thermal core temperature ({:.1}°C) exceeds normal operating range. Offloading secondary payload systems recommended to prevent chip degradation.", temp),
                reference_runbook: "TCS-RUNBOOK-042 (Thermal Runaway Mitigation)".to_string(),
                proposed_commands: vec![
                    "CMD_SHED_LOAD_SECONDARY".to_string(),
                    "CMD_RADIATOR_YAW_ADJUST".to_string()
                ],
            });
        }

        if battery_v < 11.2 {
            recs.push(DirectorRecommendation {
                category: "CRITICAL".to_string(),
                title: "EPS Low Battery Voltage".to_string(),
                description: format!("Battery cell voltage ({:.2}V) has dropped below safety limits. Immediately disable active payload sensors to optimize charge accumulation.", battery_v),
                reference_runbook: "EPS-RUNBOOK-011 (Battery Under-Voltage Recovery)".to_string(),
                proposed_commands: vec![
                    "CMD_PAYLOAD_DEACTIVATE".to_string(),
                    "CMD_ENTER_SAFE_MODE".to_string()
                ],
            });
        }

        if packet_loss > 15.0 {
            recs.push(DirectorRecommendation {
                category: "WARNING".to_string(),
                title: "TTC Link Degraded".to_string(),
                description: format!("High packet loss ({:.1}%) detected over active ground station link. Re-calibrating high-gain transmitter array or rescheduling passes recommended.", packet_loss),
                reference_runbook: "TTC-RUNBOOK-078 (RF Attenuation & Interference)".to_string(),
                proposed_commands: vec![
                    "CMD_TTC_POWER_BOOST".to_string(),
                    "CMD_SCHEDULE_PASS_BANGALORE".to_string()
                ],
            });
        }

        if fuel < 20.0 && fuel > 5.0 {
            recs.push(DirectorRecommendation {
                category: "OPTIMIZATION".to_string(),
                title: "PROP Fuel Management".to_string(),
                description: format!("Satellite fuel remaining is low ({:.1}%). Station-keeping maneuvers should be shifted to sparse cold-gas pulses to prolong spacecraft lifecycle.", fuel),
                reference_runbook: "PROP-RUNBOOK-009 (Low-Thrust Sparse Maneuvers)".to_string(),
                proposed_commands: vec![
                    "CMD_PROP_SPARSE_CYCLE_ENABLE".to_string()
                ],
            });
        }

        self.active_recommendations.insert(sat_id.to_string(), recs);
    }
}

pub async fn get_director_status(
    State(state): State<Arc<crate::ServiceState>>,
    Path(satellite): Path<orbitlogs_models::SatelliteId>,
) -> Result<Json<DirectorStatus>, StatusCode> {
    let recs = state
        .director
        .active_recommendations
        .get(satellite.as_str())
        .map(|entry| entry.value().clone())
        .unwrap_or_else(Vec::new);

    let status = if recs.iter().any(|r| r.category == "CRITICAL") {
        "DEGRADED"
    } else {
        "NOMINAL"
    };

    Ok(Json(DirectorStatus {
        satellite_id: satellite,
        operational_status: status.to_string(),
        recommendations: recs,
    }))
}

impl crate::processor::TelemetryProcessor for DirectorManager {
    type Error = std::convert::Infallible;
    type Output = ();

    fn process<'a>(
        &'a self,
        sat_id: &'a str,
        record: &'a orbitlogs_models::TelemetryRecord,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>,
    > {
        let temp = record.temperature.unwrap_or(20.0);
        let batt = record.battery_voltage.unwrap_or(13.8);
        let loss = record.packet_loss.unwrap_or(0.0) as f64;
        let fuel = record.fuel_remaining.unwrap_or(100.0) as f64;

        self.evaluate_telemetry(sat_id, temp, batt, loss, fuel);

        Box::pin(async move { Ok(()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_director_manager_new() {
        let manager = DirectorManager::new();
        assert!(manager.active_recommendations.is_empty());
    }

    #[test]
    fn test_evaluate_telemetry_nominal() {
        let manager = DirectorManager::new();
        manager.evaluate_telemetry("SAT-NOMINAL", 25.0, 13.8, 1.0, 50.0);
        let recs = manager.active_recommendations.get("SAT-NOMINAL").unwrap();
        assert!(recs.is_empty());
    }

    #[test]
    fn test_evaluate_telemetry_thermal_overload() {
        let manager = DirectorManager::new();
        manager.evaluate_telemetry("SAT-HOT", 61.5, 13.8, 1.0, 50.0);
        let recs = manager.active_recommendations.get("SAT-HOT").unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].category, "CRITICAL");
        assert_eq!(recs[0].title, "TCS Thermal Overload Detected");
        assert_eq!(
            recs[0].proposed_commands,
            vec!["CMD_SHED_LOAD_SECONDARY", "CMD_RADIATOR_YAW_ADJUST"]
        );
    }

    #[test]
    fn test_evaluate_telemetry_low_battery() {
        let manager = DirectorManager::new();
        manager.evaluate_telemetry("SAT-LOW-BAT", 25.0, 11.0, 1.0, 50.0);
        let recs = manager.active_recommendations.get("SAT-LOW-BAT").unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].category, "CRITICAL");
        assert_eq!(recs[0].title, "EPS Low Battery Voltage");
        assert_eq!(
            recs[0].proposed_commands,
            vec!["CMD_PAYLOAD_DEACTIVATE", "CMD_ENTER_SAFE_MODE"]
        );
    }

    #[test]
    fn test_evaluate_telemetry_packet_loss() {
        let manager = DirectorManager::new();
        manager.evaluate_telemetry("SAT-LOSSY", 25.0, 13.8, 16.5, 50.0);
        let recs = manager.active_recommendations.get("SAT-LOSSY").unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].category, "WARNING");
        assert_eq!(recs[0].title, "TTC Link Degraded");
        assert_eq!(
            recs[0].proposed_commands,
            vec!["CMD_TTC_POWER_BOOST", "CMD_SCHEDULE_PASS_BANGALORE"]
        );
    }

    #[test]
    fn test_evaluate_telemetry_low_fuel() {
        let manager = DirectorManager::new();
        // fuel < 20.0 && fuel > 5.0
        manager.evaluate_telemetry("SAT-LOW-FUEL", 25.0, 13.8, 1.0, 12.0);
        let recs = manager.active_recommendations.get("SAT-LOW-FUEL").unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].category, "OPTIMIZATION");
        assert_eq!(recs[0].title, "PROP Fuel Management");
        assert_eq!(
            recs[0].proposed_commands,
            vec!["CMD_PROP_SPARSE_CYCLE_ENABLE"]
        );

        // fuel <= 5.0 should not trigger PROP fuel management rule
        manager.evaluate_telemetry("SAT-VERY-LOW-FUEL", 25.0, 13.8, 1.0, 4.0);
        let recs = manager
            .active_recommendations
            .get("SAT-VERY-LOW-FUEL")
            .unwrap();
        assert!(recs.is_empty());
    }

    #[test]
    fn test_evaluate_telemetry_multiple_anomalies() {
        let manager = DirectorManager::new();
        manager.evaluate_telemetry("SAT-COMBO", 65.0, 10.5, 18.0, 15.0);
        let recs = manager.active_recommendations.get("SAT-COMBO").unwrap();
        assert_eq!(recs.len(), 4);
        assert_eq!(recs[0].category, "CRITICAL"); // Thermal
        assert_eq!(recs[1].category, "CRITICAL"); // Battery
        assert_eq!(recs[2].category, "WARNING"); // Packet loss
        assert_eq!(recs[3].category, "OPTIMIZATION"); // Fuel
    }
}
