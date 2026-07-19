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

use anyhow::Result;
use chrono::Utc;
use tracing::{error, info};

use crate::digital_twin::DigitalTwinState;
use crate::timeline::build_demo_timeline;

/// Executes a single planner evaluation cycle.
pub async fn run_planner_cycle() -> Result<()> {
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
