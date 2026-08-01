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

/// Executes a single planner evaluation cycle on a target digital twin state.
pub async fn evaluate_twin_timeline(
    twin: &DigitalTwinState,
) -> Result<(), crate::error::SafetyViolation> {
    info!(satellite = %twin.satellite_id, "Evaluating autonomous mission timeline safety constraints...");
    let timeline = build_demo_timeline(Utc::now());
    timeline.validate_safety_constraints(twin)
}

/// Executes a single planner evaluation cycle.
pub async fn run_planner_cycle() -> Result<()> {
    let twin = DigitalTwinState::demo();
    match evaluate_twin_timeline(&twin).await {
        Ok(_) => {
            info!(
                satellite = %twin.satellite_id,
                "Timeline safety validation passed successfully"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    fn test_evaluate_nominal_twin() {
        let twin = DigitalTwinState::demo();
        let res = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(evaluate_twin_timeline(&twin));
        assert!(res.is_ok());
    }

    #[test]
    fn test_evaluate_severely_degraded_battery() {
        let mut twin = DigitalTwinState::demo();
        twin.battery_capacity_fade = 0.15; // Severe battery capacity degradation (15% remaining)
        let timeline = build_demo_timeline(Utc::now());
        let res = timeline.validate_safety_constraints(&twin);
        assert!(res.is_err());
    }
}
