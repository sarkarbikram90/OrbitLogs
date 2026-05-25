use crate::scenario::Scenario;
use clap::Parser;
use std::str::FromStr;

#[derive(Parser, Debug, Clone)]
#[command(name = "Open SpaceOps - Telemetry Simulator")]
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
}

impl Config {
    pub fn get_scenario(&self) -> Scenario {
        Scenario::from_str(&self.scenario).unwrap_or(Scenario::Normal)
    }
}
