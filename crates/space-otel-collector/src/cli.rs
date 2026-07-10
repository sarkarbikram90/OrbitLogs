use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "Space OTel Collector")]
pub struct Cli {
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    pub kafka_brokers: String,

    #[arg(
        long,
        env = "OTEL_EXPORTER_OTLP_ENDPOINT",
        default_value = "http://localhost:4317"
    )]
    pub otel_endpoint: String,

    #[arg(long, env = "PROCESSED_TOPIC", default_value = "telemetry.processed")]
    pub processed_topic: String,

    #[arg(long, env = "COMMAND_EVENTS_TOPIC", default_value = "command.events")]
    pub command_events_topic: String,

    #[arg(long, env = "GS_HEALTH_TOPIC", default_value = "groundstation.health")]
    pub gs_health_topic: String,

    #[arg(long, env = "HEALTH_TOPIC", default_value = "mission.health")]
    pub health_topic: String,

    #[arg(long, env = "METRICS_PORT", default_value = "8091")]
    pub metrics_port: u16,

    #[arg(long, env = "GROUP_ID", default_value = "space-otel-collector-group")]
    pub group_id: String,
}
