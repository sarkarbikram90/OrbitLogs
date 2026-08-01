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

use clap::Parser;

/// Planner configuration loaded from CLI/environment.
#[derive(Parser, Debug)]
#[command(name = "Autonomous Mission Planner")]
pub struct Cli {
    /// Planner execution interval in seconds.
    #[arg(long, env = "PLANNER_INTERVAL_SECS", default_value = "60.0")]
    pub interval_secs: f64,

    /// HTTP port for health and readiness probes.
    #[arg(long, env = "PORT", default_value = "8099")]
    pub port: u16,
}
