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
use crate::scenario::Scenario;
use clap::Parser;
use std::str::FromStr;

#[derive(Parser, Debug, Clone)]
#[command(name = "OrbitLogs - Telemetry Simulator")]
#[command(about = "Simulates satellite telemetry stream for testing the observability platform", long_about = None)]
pub struct Config {
    #[arg(
        long,
        env = "COLLECTOR_URL",
        default_value = "http://localhost:8080/telemetry"
    )]
    pub collector_url: String,

    #[arg(long, env = "SIM_FLEET_SIZE", default_value = "5")]
    pub fleet_size: usize,

    #[arg(long, env = "SIM_FREQUENCY_MS", default_value = "1000")]
    pub frequency_ms: u64,

    #[arg(long, env = "SIM_SCENARIO", default_value = "normal")]
    pub scenario: String,

    #[arg(long, env = "SIM_DETERMINISTIC")]
    pub deterministic: bool,

    #[arg(long, env = "SIM_SEED", default_value = "42")]
    pub seed: u64,

    #[arg(long, env = "METRICS_PORT", default_value = "8081")]
    pub metrics_port: u16,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    pub kafka_brokers: String,

    #[arg(long, env = "HEALTH_TOPIC", default_value = "groundstation.health")]
    pub health_topic: String,

    #[arg(long, env = "API_KEY")]
    pub api_key: Option<String>,

    #[arg(long, env = "KAFKA_SECURITY_PROTOCOL")]
    pub kafka_security_protocol: Option<String>,

    #[arg(long, env = "KAFKA_SASL_MECHANISM")]
    pub kafka_sasl_mechanism: Option<String>,

    #[arg(long, env = "KAFKA_SASL_USERNAME")]
    pub kafka_sasl_username: Option<String>,

    #[arg(long, env = "KAFKA_SASL_PASSWORD")]
    pub kafka_sasl_password: Option<String>,

    #[arg(long, env = "KAFKA_SSL_CA_LOCATION")]
    pub kafka_ssl_ca_location: Option<String>,
}

impl Config {
    pub fn get_scenario(&self) -> Scenario {
        Scenario::from_str(&self.scenario).unwrap_or(Scenario::Normal)
    }
}
