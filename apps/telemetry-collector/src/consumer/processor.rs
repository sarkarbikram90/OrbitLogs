use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use sqlx::PgPool;
use std::collections::HashMap;
use tokio::sync::Mutex;
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
    pub security_protocol: Option<String>,
    pub sasl_mechanism: Option<String>,
    pub sasl_username: Option<String>,
    pub sasl_password: Option<String>,
    pub ssl_ca_location: Option<String>,
}

pub struct TelemetryConsumer {
    config: ConsumerConfig,
    db_pool: PgPool,
    kafka_producer: KafkaProducer,
    cache: Mutex<HashMap<String, TelemetryRecord>>,
}

impl TelemetryConsumer {
    pub fn new(config: ConsumerConfig, db_pool: PgPool, kafka_producer: KafkaProducer) -> Self {
        Self {
            config,
            db_pool,
            kafka_producer,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        let mut client_config = ClientConfig::new();
        client_config
            .set("bootstrap.servers", &self.config.brokers)
            .set("group.id", &self.config.group_id)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest");

        if let Some(ref proto) = self.config.security_protocol {
            client_config.set("security.protocol", proto);
        }
        if let Some(ref mech) = self.config.sasl_mechanism {
            client_config.set("sasl.mechanism", mech);
        }
        if let Some(ref user) = self.config.sasl_username {
            client_config.set("sasl.username", user);
        }
        if let Some(ref pass) = self.config.sasl_password {
            client_config.set("sasl.password", pass);
        }
        if let Some(ref ca) = self.config.ssl_ca_location {
            client_config.set("ssl.ca.location", ca);
        }

        let consumer: StreamConsumer = client_config.create()?;

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
                            error!("Failed to merge telemetry record for satellite {}: {}", record.satellite_id, e);
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

    async fn get_merged_record(&self, incoming: &TelemetryRecord) -> Result<TelemetryRecord, anyhow::Error> {
        let sat_id = &incoming.satellite_id;

        // 1. Check in-memory cache first
        let cached_opt = {
            let cache = self.cache.lock().await;
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
        if incoming.battery_voltage.is_some() {
            merged.battery_voltage = incoming.battery_voltage;
        }
        if incoming.battery_current.is_some() {
            merged.battery_current = incoming.battery_current;
        }
        if incoming.solar_panel_output.is_some() {
            merged.solar_panel_output = incoming.solar_panel_output;
        }
        if incoming.cpu_usage.is_some() {
            merged.cpu_usage = incoming.cpu_usage;
        }
        if incoming.memory_usage.is_some() {
            merged.memory_usage = incoming.memory_usage;
        }
        if incoming.temperature.is_some() {
            merged.temperature = incoming.temperature;
        }
        if incoming.orientation_pitch.is_some() {
            merged.orientation_pitch = incoming.orientation_pitch;
        }
        if incoming.orientation_roll.is_some() {
            merged.orientation_roll = incoming.orientation_roll;
        }
        if incoming.orientation_yaw.is_some() {
            merged.orientation_yaw = incoming.orientation_yaw;
        }
        if incoming.velocity.is_some() {
            merged.velocity = incoming.velocity;
        }
        if incoming.altitude.is_some() {
            merged.altitude = incoming.altitude;
        }
        if incoming.fuel_remaining.is_some() {
            merged.fuel_remaining = incoming.fuel_remaining;
        }
        if incoming.communication_latency.is_some() {
            merged.communication_latency = incoming.communication_latency;
        }
        if incoming.packet_loss.is_some() {
            merged.packet_loss = incoming.packet_loss;
        }
        if incoming.signal_strength.is_some() {
            merged.signal_strength = incoming.signal_strength;
        }
        if incoming.payload_power_draw.is_some() {
            merged.payload_power_draw = incoming.payload_power_draw;
        }
        if incoming.payload_active.is_some() {
            merged.payload_active = incoming.payload_active;
        }

        // 5. Save back to cache
        {
            let mut cache = self.cache.lock().await;
            cache.insert(sat_id.clone(), merged.clone());
        }

        Ok(merged)
    }

    async fn process_record(&self, incoming: &TelemetryRecord, merged: &TelemetryRecord) -> Result<(), anyhow::Error> {
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
        self.kafka_producer
            .publish(&self.config.processed_topic, sat_id, &processed_payload)
            .await?;

        // Update telemetry counters
        TELEMETRY_PROCESSED_TOTAL.with_label_values(&[sat_id]).inc();
        Ok(())
    }

    async fn detect_anomalies(&self, incoming: &TelemetryRecord, merged: &TelemetryRecord) {
        let sat_id = &incoming.satellite_id;

        // 1. Thermal Spike anomaly detection - only check if temperature is updated
        if let Some(temperature) = incoming.temperature {
            if temperature > 85.0 {
                let details = serde_json::json!({
                    "temperature": temperature,
                    "limit": 85.0
                });
                if let Err(e) = db::insert_event(
                    &self.db_pool,
                    sat_id,
                    "ANOMALY_THERMAL_SPIKE",
                    "CRITICAL",
                    &format!(
                        "Satellite temperature has spiked to {}°C",
                        temperature
                    ),
                    details,
                )
                .await
                {
                    error!("Failed to log thermal spike anomaly for {}: {}", sat_id, e);
                }
            }
        }

        // 2. Battery Degradation anomaly detection - only check if battery_voltage is updated
        if let Some(battery_voltage) = incoming.battery_voltage {
            if battery_voltage < 11.0 {
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

        // 3. Fuel Leak anomaly detection - only check if fuel_remaining is updated
        if let Some(fuel_remaining) = incoming.fuel_remaining {
            if fuel_remaining < 15.0 {
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
                    &format!(
                        "Satellite fuel level has dropped to {}%",
                        fuel_remaining
                    ),
                    details,
                )
                .await
                {
                    error!("Failed to log fuel anomaly for {}: {}", sat_id, e);
                }
            }
        }

        // 4. Communication Degradation anomaly detection - only check if communications parameters are updated
        if incoming.packet_loss.is_some() || incoming.communication_latency.is_some() {
            let packet_loss = merged.packet_loss.unwrap_or(0.0);
            let comm_latency = merged.communication_latency.unwrap_or(0.0);

            if packet_loss > 10.0 || comm_latency > 1000.0 {
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
