macro_rules! merge_opt_fields {
    ($merged:expr, $incoming:expr, $($field:ident),* $(,)?) => {
        $(
            if $incoming.$field.is_some() {
                $merged.$field = $incoming.$field;
            }
        )*
    };
}

use rdkafka::client::ClientContext;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, StreamConsumer};
use rdkafka::message::Message;
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::consumer::db;
use crate::metrics::{
    DATABASE_INSERT_LATENCY, DLQ_PUBLISH_FAILURES_TOTAL, TELEMETRY_PROCESSED_TOTAL,
};
use crate::models::TelemetryRecord;
use crate::server::producer::KafkaProducer;

struct MetricsContext;

impl ClientContext for MetricsContext {
    fn stats_raw(&self, statistics: &[u8]) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(statistics) {
            if let Some(topics) = v.get("topics").and_then(|t| t.as_object()) {
                for (topic_name, topic_val) in topics {
                    if let Some(partitions) = topic_val.get("partitions").and_then(|p| p.as_object()) {
                        for (partition_id, partition_val) in partitions {
                            if let Some(lag) = partition_val.get("consumer_lag").and_then(|l| l.as_i64()) {
                                crate::metrics::KAFKA_CONSUMER_LAG
                                    .with_label_values(&[topic_name, partition_id])
                                    .set(lag as f64);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl ConsumerContext for MetricsContext {}

pub struct ConsumerConfig {
    pub brokers: String,
    pub group_id: String,
    pub raw_topic: String,
    pub processed_topic: String,
    pub invalid_topic: String,
    pub security_protocol: Option<String>,
    pub sasl_mechanism: Option<String>,
    pub sasl_username: Option<String>,
    pub sasl_password: Option<String>,
    pub ssl_ca_location: Option<String>,
}

struct LruCache {
    map: HashMap<String, TelemetryRecord>,
    order: std::collections::VecDeque<String>,
    capacity: usize,
}

impl LruCache {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: std::collections::VecDeque::new(),
            capacity,
        }
    }

    fn get(&mut self, key: &str) -> Option<&TelemetryRecord> {
        if self.map.contains_key(key) {
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key.to_string());
            self.map.get(key)
        } else {
            None
        }
    }

    fn insert(&mut self, key: String, value: TelemetryRecord) {
        if self.map.contains_key(&key) {
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
        } else if self.map.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }
}

pub struct TelemetryConsumer {
    config: ConsumerConfig,
    db_pool: PgPool,
    kafka_producer: KafkaProducer,
    cache: Mutex<LruCache>,
    anomaly_cooldowns: Mutex<HashMap<(String, String), Instant>>,
}

impl TelemetryConsumer {
    pub fn new(config: ConsumerConfig, db_pool: PgPool, kafka_producer: KafkaProducer) -> Self {
        Self {
            config,
            db_pool,
            kafka_producer,
            cache: Mutex::new(LruCache::new(10000)),
            anomaly_cooldowns: Mutex::new(HashMap::new()),
        }
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        let mut client_config = spaceops_models::kafka::KafkaConfigBuilder::new(self.config.brokers.clone())
            .with_security(
                self.config.security_protocol.clone(),
                self.config.sasl_mechanism.clone(),
                self.config.sasl_username.clone(),
                self.config.sasl_password.clone(),
                self.config.ssl_ca_location.clone(),
            )
            .build();
        client_config
            .set("group.id", &self.config.group_id)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("statistics.interval.ms", "1000");

        let consumer: StreamConsumer<MetricsContext> = client_config.create_with_context(MetricsContext)?;

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

                    // Merge incoming subsystem telemetry with last known telemetry (or fetch from DB)
                    let merged_record = match self.get_merged_record(&record).await {
                        Ok(merged) => merged,
                        Err(e) => {
                            error!(
                                "Failed to merge telemetry record for satellite {}: {}",
                                record.satellite_id, e
                            );
                            record.clone() // Fallback to raw record
                        }
                    };

                    // Run anomaly checks (on updated fields) and insert merged record into DB with backoff retries
                    if let Err(e) = self.process_record(&record, &merged_record).await {
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

                        let key = format!("{}:{}", record.mission_id, record.satellite_id);
                        if let Err(dlq_err) = self
                            .kafka_producer
                            .publish(
                                &self.config.invalid_topic,
                                &key,
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

    async fn get_merged_record(
        &self,
        incoming: &TelemetryRecord,
    ) -> Result<TelemetryRecord, anyhow::Error> {
        let sat_id = &incoming.satellite_id;

        // 1. Check in-memory cache first
        let cached_opt = {
            let mut cache = self.cache.lock().await;
            cache.get(sat_id).cloned()
        };

        let mut merged = if let Some(cached) = cached_opt {
            cached
        } else {
            // 2. Fetch from DB if not in cache
            match db::get_latest_telemetry(&self.db_pool, sat_id).await {
                Ok(Some(db_rec)) => db_rec,
                Ok(None) => {
                    // Create an empty/base template record
                    TelemetryRecord {
                        satellite_id: sat_id.clone(),
                        subsystem_id: "unified".to_string(),
                        mission_id: incoming.mission_id.clone(),
                        timestamp: incoming.timestamp,
                        battery_voltage: Some(0.0),
                        battery_current: Some(0.0),
                        solar_panel_output: Some(0.0),
                        cpu_usage: Some(0.0),
                        memory_usage: Some(0.0),
                        temperature: Some(0.0),
                        orientation_pitch: Some(0.0),
                        orientation_roll: Some(0.0),
                        orientation_yaw: Some(0.0),
                        velocity: Some(0.0),
                        altitude: Some(0.0),
                        fuel_remaining: Some(0.0),
                        communication_latency: Some(0.0),
                        packet_loss: Some(0.0),
                        signal_strength: Some(0.0),
                        payload_power_draw: Some(0.0),
                        payload_active: Some(false),
                        event_id: incoming.event_id,
                    }
                }
                Err(e) => {
                    warn!("Failed to query DB for latest telemetry of {}: {}. Falling back to default template.", sat_id, e);
                    TelemetryRecord {
                        satellite_id: sat_id.clone(),
                        subsystem_id: "unified".to_string(),
                        mission_id: incoming.mission_id.clone(),
                        timestamp: incoming.timestamp,
                        battery_voltage: Some(0.0),
                        battery_current: Some(0.0),
                        solar_panel_output: Some(0.0),
                        cpu_usage: Some(0.0),
                        memory_usage: Some(0.0),
                        temperature: Some(0.0),
                        orientation_pitch: Some(0.0),
                        orientation_roll: Some(0.0),
                        orientation_yaw: Some(0.0),
                        velocity: Some(0.0),
                        altitude: Some(0.0),
                        fuel_remaining: Some(0.0),
                        communication_latency: Some(0.0),
                        packet_loss: Some(0.0),
                        signal_strength: Some(0.0),
                        payload_power_draw: Some(0.0),
                        payload_active: Some(false),
                        event_id: incoming.event_id,
                    }
                }
            }
        };

        // 3. Update core fields
        merged.timestamp = incoming.timestamp;
        merged.subsystem_id = incoming.subsystem_id.clone();
        merged.mission_id = incoming.mission_id.clone();
        merged.event_id = incoming.event_id;

        // 4. Overwrite fields that are populated in the incoming subsystem stream
        merge_opt_fields!(
            merged,
            incoming,
            battery_voltage,
            battery_current,
            solar_panel_output,
            cpu_usage,
            memory_usage,
            temperature,
            orientation_pitch,
            orientation_roll,
            orientation_yaw,
            velocity,
            altitude,
            fuel_remaining,
            communication_latency,
            packet_loss,
            signal_strength,
            payload_power_draw,
            payload_active,
        );

        // 5. Save back to cache
        {
            let mut cache = self.cache.lock().await;
            cache.insert(sat_id.clone(), merged.clone());
        }

        Ok(merged)
    }

    async fn process_record(
        &self,
        incoming: &TelemetryRecord,
        merged: &TelemetryRecord,
    ) -> Result<(), anyhow::Error> {
        let sat_id = &merged.satellite_id;

        // Perform anomaly/event detection using updated fields in incoming stream
        self.detect_anomalies(incoming, merged).await;

        // Persist to TimescaleDB hypertable with exponential backoff retries
        let mut attempts = 0;
        let max_attempts = 3;
        let mut backoff = Duration::from_millis(100);
        let db_start = Instant::now();

        loop {
            match db::insert_telemetry(&self.db_pool, merged).await {
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
        let processed_payload = serde_json::to_string(merged)?;
        let key = format!("{}:{}", merged.mission_id, sat_id);
        self.kafka_producer
            .publish(&self.config.processed_topic, &key, &processed_payload)
            .await?;

        // Update telemetry counters
        TELEMETRY_PROCESSED_TOTAL.with_label_values(&[sat_id]).inc();
        Ok(())
    }

    async fn should_alert(&self, satellite_id: &str, anomaly_type: &str) -> bool {
        let mut cooldowns = self.anomaly_cooldowns.lock().await;
        let now = Instant::now();
        let key = (satellite_id.to_string(), anomaly_type.to_string());
        if let Some(&last_alert) = cooldowns.get(&key) {
            if now.duration_since(last_alert) < Duration::from_secs(60) {
                return false;
            }
        }
        cooldowns.insert(key, now);
        true
    }

    async fn detect_anomalies(&self, incoming: &TelemetryRecord, merged: &TelemetryRecord) {
        let sat_id = &incoming.satellite_id;

        // 1. Thermal Spike anomaly detection - only check if temperature is updated
        if let Some(temperature) = incoming.temperature {
            if temperature > 85.0 {
                if self.should_alert(sat_id, "ANOMALY_THERMAL_SPIKE").await {
                    let details = serde_json::json!({
                        "temperature": temperature,
                        "limit": 85.0
                    });
                    if let Err(e) = db::insert_event(
                        &self.db_pool,
                        sat_id,
                        "ANOMALY_THERMAL_SPIKE",
                        "CRITICAL",
                        &format!("Satellite temperature has spiked to {}°C", temperature),
                        details,
                    )
                    .await
                    {
                        error!("Failed to log thermal spike anomaly for {}: {}", sat_id, e);
                    }
                }
            }
        }

        // 2. Battery Degradation anomaly detection - only check if battery_voltage is updated
        if let Some(battery_voltage) = incoming.battery_voltage {
            if battery_voltage < 11.0 {
                if self.should_alert(sat_id, "ANOMALY_BATTERY_DEGRADATION").await {
                    let details = serde_json::json!({
                        "battery_voltage": battery_voltage,
                        "limit": 11.0
                    });
                    if let Err(e) = db::insert_event(
                        &self.db_pool,
                        sat_id,
                        "ANOMALY_BATTERY_DEGRADATION",
                        "CRITICAL",
                        &format!("Battery voltage is critical: {}V", battery_voltage),
                        details,
                    )
                    .await
                    {
                        error!("Failed to log battery anomaly for {}: {}", sat_id, e);
                    }
                }
            }
        }

        // 3. Fuel Leak anomaly detection - only check if fuel_remaining is updated
        if let Some(fuel_remaining) = incoming.fuel_remaining {
            if fuel_remaining < 15.0 {
                if self.should_alert(sat_id, "ANOMALY_FUEL_LOW").await {
                    let details = serde_json::json!({
                        "fuel_remaining": fuel_remaining,
                        "limit": 15.0
                    });
                    if let Err(e) = db::insert_event(
                        &self.db_pool,
                        sat_id,
                        "ANOMALY_FUEL_LOW",
                        if fuel_remaining < 5.0 {
                            "CRITICAL"
                        } else {
                            "WARNING"
                        },
                        &format!("Satellite fuel level has dropped to {}%", fuel_remaining),
                        details,
                    )
                    .await
                    {
                        error!("Failed to log fuel anomaly for {}: {}", sat_id, e);
                    }
                }
            }
        }

        // 4. Communication Degradation anomaly detection - only check if communications parameters are updated
        if incoming.packet_loss.is_some() || incoming.communication_latency.is_some() {
            let packet_loss = merged.packet_loss.unwrap_or(0.0);
            let comm_latency = merged.communication_latency.unwrap_or(0.0);

            if packet_loss > 10.0 || comm_latency > 1000.0 {
                if self.should_alert(sat_id, "ANOMALY_COMMUNICATION_DEGRADED").await {
                    let details = serde_json::json!({
                        "packet_loss": packet_loss,
                        "latency": comm_latency
                    });
                    if let Err(e) = db::insert_event(
                        &self.db_pool,
                        sat_id,
                        "ANOMALY_COMMUNICATION_DEGRADED",
                        "WARNING",
                        &format!(
                            "Communications degraded. Latency: {}ms, Packet Loss: {}%",
                            comm_latency, packet_loss
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
    }
}
