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

use std::time::Duration;
use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn, Level};

/// Flight safety parameters & battery constraints
const EXECUTE_BURN_COST: f32 = 25.0;      // Battery percentage drop for thruster actuators
const LOAD_SHED_RECOVERY: f32 = 5.0;       // Battery percentage saved from load shedding
const BASE_SOLAR_RECOVERY: f32 = 2.0;      // Base battery percentage gained from solar orientation
const MAX_BATTERY_PERCENT: f32 = 100.0;
const MIN_SAFE_BATTERY: f32 = 20.0;       // Minimum allowable SoC threshold (%)

/// Default demo schedule offsets (minutes from evaluation epoch).
const LOAD_SHED_DELAY_MINUTES: i64 = 5;
const RADIATOR_DELAY_MINUTES: i64 = 15;
const EXECUTE_BURN_DELAY_MINUTES: i64 = 30;

/// Planner configuration loaded from CLI/environment.
#[derive(Parser, Debug)]
#[command(name = "Autonomous Mission Planner")]
struct Cli {
    /// Planner execution interval in seconds.
    #[arg(
        long,
        env = "PLANNER_INTERVAL_SECS",
        default_value = "60.0"
    )]
    interval_secs: f64,
}

/// Structured safety constraint violations for flight telemetry and audit logging.
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyViolation {
    BatteryDepleted {
        timestamp: DateTime<Utc>,
        battery_level: f32,
        min_safe_threshold: f32,
    },
}

impl std::fmt::Display for SafetyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BatteryDepleted {
                timestamp,
                battery_level,
                min_safe_threshold,
            } => {
                write!(
                    f,
                    "Constraint violated at {timestamp}: battery level ({battery_level:.1}%) dropped below safe operating threshold ({min_safe_threshold:.1}%).",
                )
            }
        }
    }
}

impl std::error::Error for SafetyViolation {}

/// High-level spacecraft commands.
///
/// Every variant represents a valid onboard command sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    ShedSecondaryLoad,

    AdjustRadiatorYaw {
        angle_degrees: f32,
    },

    OrientSolarPanels {
        heading_degrees: f32,
    },

    ExecuteBurn {
        vector: [f64; 3],
    },
}

impl Command {
    /// Returns the net battery SoC delta (%) resulting from command execution,
    /// factoring in hardware efficiency parameters.
    fn battery_delta(&self, solar_efficiency: f32) -> f32 {
        match self {
            Self::ExecuteBurn { .. } => -EXECUTE_BURN_COST,

            Self::ShedSecondaryLoad => LOAD_SHED_RECOVERY,

            Self::OrientSolarPanels { heading_degrees } => {
                // Efficiency scaling based on sun vector alignment (0-180 deg) and solar degradation
                let alignment_factor = (heading_degrees.to_radians().cos().abs()).max(0.1);
                BASE_SOLAR_RECOVERY * solar_efficiency * alignment_factor
            }

            Self::AdjustRadiatorYaw { .. } => 0.0,
        }
    }
}

/// Snapshot of spacecraft health used by the mission planner.
#[derive(Debug, Serialize, Deserialize)]
pub struct DigitalTwinState {
    pub satellite_id: String,
    pub solar_efficiency: f32,
    pub battery_capacity_fade: f32,
    pub radiator_thermal_resistance: f32,
}

impl DigitalTwinState {
    /// Returns a demonstration digital twin state.
    ///
    /// Production deployments retrieve this from real-time vehicle telemetry.
    pub fn demo() -> Self {
        Self {
            satellite_id: "SAT-0001".to_string(),
            solar_efficiency: 0.95,
            battery_capacity_fade: 0.90,
            radiator_thermal_resistance: 1.05,
        }
    }
}

pub type ScheduledCommand = (DateTime<Utc>, Command);

/// Ordered sequence of spacecraft commands scheduled for onboard execution.
pub struct OnboardTimeline {
    schedule: Vec<ScheduledCommand>,
}

impl OnboardTimeline {
    pub fn new(schedule: Vec<ScheduledCommand>) -> Self {
        Self { schedule }
    }

    /// Simulates command timeline execution and verifies battery state of charge (SoC)
    /// safety constraints under degradation models.
    pub fn validate_safety_constraints(
        &self,
        twin: &DigitalTwinState,
    ) -> Result<(), SafetyViolation> {
        // Upper bound capacity ceiling based on battery degradation (capacity fade)
        let max_usable_capacity = MAX_BATTERY_PERCENT * twin.battery_capacity_fade.clamp(0.1, 1.0);
        let mut battery = max_usable_capacity;

        for (timestamp, command) in &self.schedule {
            let delta = command.battery_delta(twin.solar_efficiency);

            // Energy conservation law: battery SoC cannot exceed maximum physical capacity ceiling
            battery = (battery + delta).min(max_usable_capacity);

            if battery < MIN_SAFE_BATTERY {
                return Err(SafetyViolation::BatteryDepleted {
                    timestamp: *timestamp,
                    battery_level: battery,
                    min_safe_threshold: MIN_SAFE_BATTERY,
                });
            }
        }

        Ok(())
    }
}

/// Constructs a demonstration mission timeline.
fn build_demo_timeline(now: DateTime<Utc>) -> OnboardTimeline {
    OnboardTimeline::new(vec![
        (
            now + chrono::Duration::minutes(LOAD_SHED_DELAY_MINUTES),
            Command::ShedSecondaryLoad,
        ),
        (
            now + chrono::Duration::minutes(RADIATOR_DELAY_MINUTES),
            Command::AdjustRadiatorYaw {
                angle_degrees: 45.0,
            },
        ),
        (
            now + chrono::Duration::minutes(EXECUTE_BURN_DELAY_MINUTES),
            Command::ExecuteBurn {
                vector: [1.2, 0.0, 0.0],
            },
        ),
    ])
}

/// Executes a single planner evaluation cycle.
async fn run_planner_cycle() -> Result<()> {
    info!("Evaluating mission timeline...");

    let twin = DigitalTwinState::demo();
    let timeline = build_demo_timeline(Utc::now());

    match timeline.validate_safety_constraints(&twin) {
        Ok(_) => {
            info!(
                satellite = %twin.satellite_id,
                "Timeline validation passed"
            );
        }

        Err(violation) => {
            error!(
                satellite = %twin.satellite_id,
                reason = %violation,
                "Timeline safety constraint check failed"
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from a .env file if present.
    let _ = dotenvy::dotenv();

    // Parse command-line arguments.
    let cli = Cli::parse();

    // Initialize structured logging.
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting Autonomous Mission Planner...");

    // Sanitize planner interval input against negative values or NaN
    let interval_secs = if cli.interval_secs.is_finite() && cli.interval_secs > 0.0 {
        cli.interval_secs
    } else {
        warn!(
            invalid_interval = cli.interval_secs,
            "Invalid PLANNER_INTERVAL_SECS configuration; falling back to 60.0 seconds"
        );
        60.0
    };

    loop {
        // Execute a single planner cycle.
        if let Err(error) = run_planner_cycle().await {
            warn!(
                error = %error,
                "Planner cycle completed with an error"
            );
        }

        // Wait for either:
        //   1. Ctrl+C to gracefully terminate.
        //   2. The configured interval before running again.
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received.");
                info!("Stopping Autonomous Mission Planner gracefully.");
                break;
            }

            _ = tokio::time::sleep(
                Duration::from_secs_f64(interval_secs)
            ) => {}
        }
    }

    info!("Autonomous Mission Planner stopped.");

    Ok(())
}