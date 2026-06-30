use serde::{Deserialize, Serialize};
use spaceops_models::HealthState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    pub state: HealthState,
    pub score: i32,    // 0 to 100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteHealthSummary {
    pub id: String,
    pub overall_state: HealthState,
    pub overall_score: i32,    // 0 to 100
    pub fuel_remaining: f32,   // 0 to 100
    pub cpu_usage: f32,        // 0 to 100
    pub memory_usage: f32,     // 0 to 100
    pub payload_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMetrics {
    pub availability: f64, // % of fleet that is online (GREEN or YELLOW)
    pub risk_score: f64,   // 0 to 100 (weighted degradation)
    pub utilization: f64,  // % cpu/mem/payload usage
    pub capacity: f64,     // % remaining capacity (e.g. fuel + overall margin)
    pub readiness: f64,    // % of fleet that is GREEN (fully ready)
    pub healthy_count: usize,
    pub warning_count: usize,
    pub critical_count: usize,
}

pub struct FleetHealthEngine;

impl FleetHealthEngine {
    pub fn calculate(satellites: &[SatelliteHealthSummary]) -> FleetMetrics {
        let total = satellites.len();
        if total == 0 {
            return FleetMetrics {
                availability: 100.0,
                risk_score: 0.0,
                utilization: 0.0,
                capacity: 100.0,
                readiness: 100.0,
                healthy_count: 0,
                warning_count: 0,
                critical_count: 0,
            };
        }

        let mut green = 0;
        let mut yellow = 0;
        let mut red = 0;
        let mut sum_fuel = 0.0;
        let mut sum_cpu = 0.0;
        let mut sum_mem = 0.0;
        let mut active_payloads = 0;
        let mut sum_score = 0.0;

        for sat in satellites {
            match sat.overall_state {
                HealthState::Green => green += 1,
                HealthState::Yellow => yellow += 1,
                HealthState::Red => red += 1,
            }
            sum_fuel += sat.fuel_remaining;
            sum_cpu += sat.cpu_usage;
            sum_mem += sat.memory_usage;
            if sat.payload_active {
                active_payloads += 1;
            }
            sum_score += sat.overall_score as f32;
        }

        let total_f = total as f64;
        let availability = ((green + yellow) as f64 / total_f) * 100.0;
        let readiness = (green as f64 / total_f) * 100.0;
        let avg_score = sum_score as f64 / total_f;
        let risk_score = 100.0 - avg_score; // Lower health score -> Higher risk

        // Average utilization of resources
        let avg_cpu = sum_cpu as f64 / total_f;
        let avg_mem = sum_mem as f64 / total_f;
        let payload_util = (active_payloads as f64 / total_f) * 100.0;
        let utilization = (avg_cpu + avg_mem + payload_util) / 3.0;

        // Capacity: blend of remaining fuel and status score
        let avg_fuel = sum_fuel as f64 / total_f;
        let capacity = (avg_fuel + avg_score) / 2.0;

        FleetMetrics {
            availability,
            risk_score,
            utilization,
            capacity,
            readiness,
            healthy_count: green,
            warning_count: yellow,
            critical_count: red,
        }
    }
}