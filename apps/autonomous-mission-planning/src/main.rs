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
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, Level};

#[derive(Parser, Debug)]
#[command(name = "Autonomous Mission Planner")]
struct Cli {
    #[arg(long, env = "PLANNER_INTERVAL_SECS", default_value = "60.0")]
    interval_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    ShedSecondaryLoad,
    AdjustRadiatorYaw { angle_degrees: f32 },
    OrientSolarPanels { heading_degrees: f32 },
    ExecuteBurn { vector: [f64; 3] },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DigitalTwinState {
    pub satellite_id: String,
    pub solar_efficiency: f32,
    pub battery_capacity_fade: f32,
    pub radiator_thermal_resistance: f32,
}

pub struct OnboardTimeline {
    pub schedule: Vec<(DateTime<Utc>, Command)>,
}

impl OnboardTimeline {
    /// Runs a simulation check to ensure no commands violate flight constraints (e.g. power draining).
    pub fn validate_safety_constraints(&self, twin: &DigitalTwinState) -> Result<(), &'static str> {
        let mut simulated_battery = 100.0 * twin.battery_capacity_fade;

        for (_time, cmd) in &self.schedule {
            match cmd {
                Command::ExecuteBurn { .. } => {
                    simulated_battery -= 25.0; // Heavy discharge for thruster actuators
                }
                Command::ShedSecondaryLoad => {
                    simulated_battery += 5.0;
                }
                Command::OrientSolarPanels { .. } => {
                    simulated_battery += 2.0;
                }
                _ => {}
            }
            if simulated_battery < 20.0 {
                return Err("Constraint Violated: Battery capacity dropped below critical safe threshold during execution.");
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    info!("Starting Autonomous Mission Planner...");

    loop {
        info!("Evaluating incident command timelines for flight safety...");

        // Simulating twin state of a satellite
        let twin = DigitalTwinState {
            satellite_id: "SAT-0001".to_string(),
            solar_efficiency: 0.95,
            battery_capacity_fade: 0.90,
            radiator_thermal_resistance: 1.05,
        };

        // Construct timeline
        let now = Utc::now();
        let timeline = OnboardTimeline {
            schedule: vec![
                (
                    now + chrono::Duration::minutes(5),
                    Command::ShedSecondaryLoad,
                ),
                (
                    now + chrono::Duration::minutes(15),
                    Command::AdjustRadiatorYaw {
                        angle_degrees: 45.0,
                    },
                ),
                (
                    now + chrono::Duration::minutes(30),
                    Command::ExecuteBurn {
                        vector: [1.2, 0.0, 0.0],
                    },
                ),
            ],
        };

        match timeline.validate_safety_constraints(&twin) {
            Ok(_) => {
                info!(
                    "Timeline safety constraints validation passed for satellite {}",
                    twin.satellite_id
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Timeline safety constraints check failed for {}: {}",
                    twin.satellite_id,
                    e
                );
            }
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received. Exiting Autonomous Mission Planner gracefully.");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs_f64(cli.interval_secs)) => {}
        }
    }
    Ok(())
}
