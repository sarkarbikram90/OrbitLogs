use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "Constellation Simulator")]
pub struct Cli {
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    pub kafka_brokers: String,

    #[arg(long, env = "FLEET_SIZE", default_value = "500")] // Support 100, 500, 1000, 5000, 10000
    pub fleet_size: usize,

    #[arg(long, env = "TICK_INTERVAL_MS", default_value = "5000")]
    pub tick_interval_ms: u64,

    #[arg(long, env = "RAW_TOPIC", default_value = "telemetry.raw")]
    pub raw_topic: String,
}
