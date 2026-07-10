use rdkafka::consumer::{Consumer, StreamConsumer};
use std::ops::Deref;

/// A guard for Kafka consumer that ensures unsubscription on drop (RAII pattern) (P1 #5).
/// Implements Deref to allow direct usage of the inner StreamConsumer.
pub struct KafkaConsumerGuard {
    pub consumer: StreamConsumer,
    pub topic: String,
}

impl Deref for KafkaConsumerGuard {
    type Target = StreamConsumer;

    fn deref(&self) -> &Self::Target {
        &self.consumer
    }
}

impl Drop for KafkaConsumerGuard {
    fn drop(&mut self) {
        tracing::info!(
            topic = %self.topic,
            "Unsubscribing Kafka consumer on drop"
        );
        self.consumer.unsubscribe();
    }
}
