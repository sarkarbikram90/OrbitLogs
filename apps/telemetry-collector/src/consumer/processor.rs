use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use sqlx::PgPool;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

use crate::consumer::db;
use crate::metrics::{
    DATABASE_INSERT_LATENCY, DLQ_PUBLISH_FAILURES_TOTAL, TELEMETRY_PROCESSED_TOTAL,
};
use crate::models::TelemetryRecord;
use crate::server::producer::KafkaProducer;

pub struct ConsumerConfig {
    pub brokers: String,
    pub group_id: String,
    pub raw_topic: String,
    pub processed_topic: String,
    pub invalid_topic: String,
}

pub struct TelemetryConsumer {
    config: ConsumerConfig,
    db_pool: PgPool,
    kafka_producer: KafkaProducer,
}

impl TelemetryConsumer {
    pub fn new(config: ConsumerConfig, db_pool: PgPool, kafka_producer: KafkaProducer) -> Self {
        Self {
            config,
            db_pool,
            kafka_producer,
        }
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.config.brokers)
            .set("group.id", &self.config.group_id)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()?;

        consumer.subscribe(&[&self.config.raw_topic])?;
        info!(
            "Kafka Consumer subscribed to topic '{}' with manual offset management (at-least-once delivery)",
            self.config.raw_topic
        );

        loop {
            match consumer.recv().await {
                Ok(msg) => {
                    let payload = match msg.payload_view::<str>() {
                        Some(Ok(s)) => s,
                        Some(Err(e)) => {
                            error!("Error parsing message payload as string: {}", e);
                            // Commit offset to skip corrupt binary message
                            if let Err(ce) = consumer.commit_message(&msg, CommitMode::Async) {
                                error!("Failed to commit offset for corrupt message: {}", ce);
                            }
                            continue;
                        }
                        None => {
                            warn!("Received empty message payload from Kafka");
                            if let Err(ce) = consumer.commit_message(&msg, CommitMode::Async) {
                                error!("Failed to commit offset for empty message: {}", ce);
                            }
                            continue;
                        }
                    };

                    let record: TelemetryRecord = match serde_json::from_str(payload) {
                        Ok(r) => r,
                        Err(e) => {
                            error!(
                                "JSON deserialization failed for message: {}. Payload: {}",
                                e, payload
                            );
                            // Route to dead letter topic since message is corrupted
                            if let Err(dlq_err) = self
                                .kafka_producer
                                .publish(
                                    &self.config.invalid_topic,
                                    "corrupt-payload",
                                    &serde_json::json!({
                                        "payload": payload,
                                        "error": e.to_string(),
                                        "reason": "json_deserialization_failed"
                                    })
                                    .to_string(),
                                )
                                .await
                            {
                                error!("DLQ publish failed for corrupt payload: {}", dlq_err);
                                DLQ_PUBLISH_FAILURES_TOTAL.inc();
                            }
                            // Commit offset to prevent infinite reprocessing of corrupt data
                            if let Err(ce) = consumer.commit_message(&msg, CommitMode::Async) {
                                error!("Failed to commit offset after DLQ routing: {}", ce);
                            }
                            continue;
                        }
                    };

                    // Run anomaly checks and insert into DB with backoff retries
                    if let Err(e) = self.process_record(&record).await {
                        error!(
                            "Failed to process record for satellite {}: {}",
                            record.satellite_id, e
                        );

                        // Dead letter route if processing/database insertion completely fails
                        let dlq_payload = serde_json::json!({
                            "record": record,
                            "error": e.to_string(),
                            "reason": "database_persistence_failed"
                        })
                        .to_string();

                        if let Err(dlq_err) = self
                            .kafka_producer
                            .publish(
                                &self.config.invalid_topic,
                                &record.satellite_id,
                                &dlq_payload,
                            )
                            .await
                        {
                            error!(
                                "DLQ publish failed for satellite {} after DB failure: {}",
                                record.satellite_id, dlq_err
                            );
                            DLQ_PUBLISH_FAILURES_TOTAL.inc();
                        }
                    }

                    // Commit offset only after successful processing or DLQ routing
                    // This ensures at-least-once delivery semantics
                    if let Err(ce) = consumer.commit_message(&msg, CommitMode::Async) {
                        error!("Failed to commit Kafka offset: {}", ce);
                    }
                }
                Err(e) => {
                    error!("Kafka consumer receive error: {}", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    async fn process_record(&self, record: &TelemetryRecord) -> Result<(), anyhow::Error> {
        let sat_id = &record.satellite_id;

        // Perform anomaly/event detection
        self.detect_anomalies(record).await;

        // Persist to TimescaleDB hypertable with exponential backoff retries
        let mut attempts = 0;
        let max_attempts = 3;
        let mut backoff = Duration::from_millis(100);
        let db_start = Instant::now();

        loop {
            match db::insert_telemetry(&self.db_pool, record).await {
                Ok(_) => {
                    DATABASE_INSERT_LATENCY.observe(db_start.elapsed().as_secs_f64());
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        error!(
                            "Database insert failed after {} attempts: {}",
                            max_attempts, e
                        );
                        return Err(anyhow::anyhow!("Database write exhausted: {}", e));
                    }
                    warn!(
                        "Database insert attempt {} failed. Retrying in {:?}: {}",
                        attempts, backoff, e
                    );
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
            }
        }

        // Publish to telemetry.processed topic
        let processed_payload = serde_json::to_string(record)?;
        self.kafka_producer
            .publish(&self.config.processed_topic, sat_id, &processed_payload)
            .await?;

        // Update telemetry counters
        TELEMETRY_PROCESSED_TOTAL.with_label_values(&[sat_id]).inc();
        Ok(())
    }

    async fn detect_anomalies(&self, record: &TelemetryRecord) {
        let sat_id = &record.satellite_id;

        // 1. Thermal Spike anomaly detection
        if record.temperature > 85.0 {
            let details = serde_json::json!({
                "temperature": record.temperature,
                "limit": 85.0
            });
            if let Err(e) = db::insert_event(
                &self.db_pool,
                sat_id,
                "ANOMALY_THERMAL_SPIKE",
                "CRITICAL",
                &format!(
                    "Satellite temperature has spiked to {}°C",
                    record.temperature
                ),
                details,
            )
            .await
            {
                error!("Failed to log thermal spike anomaly for {}: {}", sat_id, e);
            }
        }

        // 2. Battery Degradation anomaly detection
        if record.battery_voltage < 11.0 {
            let details = serde_json::json!({
                "battery_voltage": record.battery_voltage,
                "limit": 11.0
            });
            if let Err(e) = db::insert_event(
                &self.db_pool,
                sat_id,
                "ANOMALY_BATTERY_DEGRADATION",
                "CRITICAL",
                &format!("Battery voltage is critical: {}V", record.battery_voltage),
                details,
            )
            .await
            {
                error!("Failed to log battery anomaly for {}: {}", sat_id, e);
            }
        }

        // 3. Fuel Leak anomaly detection
        if record.fuel_remaining < 15.0 {
            let details = serde_json::json!({
                "fuel_remaining": record.fuel_remaining,
                "limit": 15.0
            });
            if let Err(e) = db::insert_event(
                &self.db_pool,
                sat_id,
                "ANOMALY_FUEL_LOW",
                if record.fuel_remaining < 5.0 {
                    "CRITICAL"
                } else {
                    "WARNING"
                },
                &format!(
                    "Satellite fuel level has dropped to {}%",
                    record.fuel_remaining
                ),
                details,
            )
            .await
            {
                error!("Failed to log fuel anomaly for {}: {}", sat_id, e);
            }
        }

        // 4. Communication Degradation anomaly detection
        if record.packet_loss > 10.0 || record.communication_latency > 1000.0 {
            let details = serde_json::json!({
                "packet_loss": record.packet_loss,
                "latency": record.communication_latency
            });
            if let Err(e) = db::insert_event(
                &self.db_pool,
                sat_id,
                "ANOMALY_COMMUNICATION_DEGRADED",
                "WARNING",
                &format!(
                    "Communications degraded. Latency: {}ms, Packet Loss: {}%",
                    record.communication_latency, record.packet_loss
                ),
                details,
            )
            .await
            {
                error!("Failed to log communication anomaly for {}: {}", sat_id, e);
            }
        }
    }
}
