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
            "battery_degradation" | "battery-degradation" | "battery" => Ok(Scenario::BatteryDegradation),
            "communication_degradation" | "communication-degradation" | "comm" => Ok(Scenario::CommunicationDegradation),
            "fuel_leakage" | "fuel-leakage" | "fuel" => Ok(Scenario::FuelLeakage),
            _ => Err(format!("Unknown scenario: {}", s)),
        }
    }
}
