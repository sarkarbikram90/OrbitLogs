pub mod db;
pub mod processor;

pub use processor::{ConsumerConfig, TelemetryConsumer};
use sqlx::PgPool;
use crate::server::producer::KafkaProducer;
use tracing::info;

pub async fn start_consumer(
    config: ConsumerConfig,
    db_pool: PgPool,
    kafka_producer: KafkaProducer,
) -> Result<(), anyhow::Error> {
    info!("Starting Telemetry Consumer...");
    let consumer = TelemetryConsumer::new(config, db_pool, kafka_producer);
    consumer.start().await?;
    Ok(())
}
