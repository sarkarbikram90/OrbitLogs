mod cli;
mod handlers;
mod metrics;
mod server;
mod signal;

use clap::Parser;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use std::time::Duration;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = cli::Cli::parse();

    // Init tracing & OTel
    orbitlogs_models::telemetry::init_telemetry("space-otel-collector", &cli.otel_endpoint)?;

    info!("Starting Space OTel Collector...");
    metrics::register_metrics();
    tokio::spawn(server::start_metrics_server(cli.metrics_port));

    // Setup Kafka Consumer
    let mut consumer_config = KafkaConfigBuilder::from_env(cli.kafka_brokers.clone()).build();
    consumer_config
        .set("group.id", &cli.group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest");

    let consumer: StreamConsumer = consumer_config.create()?;
    consumer.subscribe(&[
        &cli.processed_topic,
        &cli.command_events_topic,
        &cli.gs_health_topic,
        &cli.health_topic,
    ])?;

    info!("Subscribed to Kafka topics for OTel ingestion");

    let shutdown_fut = signal::shutdown_signal();
    tokio::pin!(shutdown_fut);

    loop {
        tokio::select! {
            _ = &mut shutdown_fut => {
                info!("Shutdown signal received. Exiting Space OTel Collector loop...");
                break;
            }
            msg_res = consumer.recv() => {
                match msg_res {
                    Ok(msg) => {
                        // Extract trace context from Kafka headers
                        let mut carrier = std::collections::HashMap::new();
                        if let Some(headers) = msg.headers() {
                            for i in 0..rdkafka::message::Headers::count(headers) {
                                let header = rdkafka::message::Headers::get(headers, i);
                                if let Ok(val) = std::str::from_utf8(header.value.unwrap_or(&[])) {
                                    carrier.insert(header.key.to_string(), val.to_string());
                                }
                            }
                        }
                        let parent_context = opentelemetry::global::get_text_map_propagator(|propagator| {
                            propagator.extract(&carrier)
                        });

                        let topic = msg.topic();
                        let payload = match msg.payload_view::<str>() {
                            Some(Ok(s)) => s,
                            _ => {
                                let _ = consumer.commit_message(&msg, CommitMode::Async);
                                continue;
                            }
                        };

                        // Dispatch to the appropriate handler
                        if topic == cli.processed_topic {
                            handlers::handle_telemetry(payload, parent_context);
                        } else if topic == cli.command_events_topic {
                            handlers::handle_command_event(payload, parent_context);
                        } else if topic == cli.gs_health_topic {
                            handlers::handle_gs_health(payload, parent_context);
                        } else if topic == cli.health_topic {
                            handlers::handle_satellite_health(payload, parent_context);
                        }

                        let _ = consumer.commit_message(&msg, CommitMode::Async);
                    }
                    Err(e) => {
                        error!("Error in Space OTel Collector loop: {}", e);
                        tokio::time::sleep(Duration::from_secs_f64(0.5)).await;
                    }
                }
            }
        }
    }

    info!("Cleaning up telemetry before exit...");
    orbitlogs_models::telemetry::shutdown_telemetry();
    Ok(())
}
