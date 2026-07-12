use orbitlogs_models::TelemetryRecord;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

pub fn send_telemetry(
    producer: Arc<FutureProducer>,
    topic: &str,
    key: String,
    rec: TelemetryRecord,
) {
    if let Ok(json_payload) = serde_json::to_string(&rec) {
        let topic_owned = topic.to_string();
        tokio::spawn(async move {
            let record = FutureRecord::to(&topic_owned)
                .key(&key)
                .payload(&json_payload);
            if let Err((e, _)) = producer.send(record, Duration::from_secs(5)).await {
                error!("Failed to produce to topic {}: {}", topic_owned, e);
            }
        });
    }
}
