use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use tracing::{debug, error};

#[derive(Clone)]
pub struct KafkaProducer {
    producer: FutureProducer,
}

impl KafkaProducer {
    pub fn new(brokers: &str) -> Result<Self, anyhow::Error> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("queue.buffering.max.messages", "100000")
            .create()?;

        Ok(Self { producer })
    }

    pub async fn publish(&self, topic: &str, key: &str, payload: &str) -> Result<(), anyhow::Error> {
        let record = FutureRecord::to(topic)
            .key(key)
            .payload(payload);

        match self.producer.send(record, Duration::from_secs(5)).await {
            Ok((partition, offset)) => {
                debug!("Successfully published event to topic {} [partition: {}, offset: {}]", topic, partition, offset);
                Ok(())
            }
            Err((e, _)) => {
                error!("Failed to publish event to topic {}: {}", topic, e);
                Err(anyhow::anyhow!("Kafka send error: {}", e))
            }
        }
    }
}
