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

mod command;
mod config;
mod digital_twin;
mod error;
mod planner;
mod timeline;

use std::time::Duration;
use anyhow::Result;
use clap::Parser;
use tracing::{info, warn, Level};

pub use command::Command;
pub use config::Cli;
pub use digital_twin::DigitalTwinState;
pub use error::SafetyViolation;
pub use planner::run_planner_cycle;
pub use timeline::OnboardTimeline;

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