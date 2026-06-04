use lazy_static::lazy_static;
use prometheus::{Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref TELEMETRY_RECEIVED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "telemetry_received_total",
            "Total number of telemetry events received by the HTTP collector endpoint"
        ),
        &["satellite_id", "status"]
    )
    .expect("telemetry_received_total metric must be created");
    pub static ref TELEMETRY_PROCESSED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "telemetry_processed_total",
            "Total number of telemetry events successfully processed by the consumer"
        ),
        &["satellite_id"]
    )
    .expect("telemetry_processed_total metric must be created");
    pub static ref TELEMETRY_INVALID_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "telemetry_invalid_total",
            "Total number of telemetry events rejected due to schema/validation errors"
        ),
        &["reason"]
    )
    .expect("telemetry_invalid_total metric must be created");
    pub static ref DLQ_PUBLISH_FAILURES_TOTAL: IntCounter = IntCounter::new(
        "dlq_publish_failures_total",
        "Total number of failed attempts to publish messages to the dead letter queue"
    )
    .expect("dlq_publish_failures_total metric must be created");
    pub static ref PROCESSING_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "processing_latency",
            "Time taken to validate, enrich and queue incoming telemetry (seconds)"
        )
        .buckets(vec![
            0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0
        ])
    )
    .expect("processing_latency metric must be created");
    pub static ref KAFKA_PUBLISH_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "kafka_publish_latency",
            "Time taken to publish message to Kafka topic (seconds)"
        )
        .buckets(vec![
            0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0
        ])
    )
    .expect("kafka_publish_latency metric must be created");
    pub static ref DATABASE_INSERT_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "database_insert_latency",
            "Time taken to write telemetry payload to TimescaleDB (seconds)"
        )
        .buckets(vec![
            0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0
        ])
    )
    .expect("database_insert_latency metric must be created");
}

pub fn register_metrics() {
    REGISTRY
        .register(Box::new(TELEMETRY_RECEIVED_TOTAL.clone()))
        .expect("telemetry_received_total registration must succeed");
    REGISTRY
        .register(Box::new(TELEMETRY_PROCESSED_TOTAL.clone()))
        .expect("telemetry_processed_total registration must succeed");
    REGISTRY
        .register(Box::new(TELEMETRY_INVALID_TOTAL.clone()))
        .expect("telemetry_invalid_total registration must succeed");
    REGISTRY
        .register(Box::new(DLQ_PUBLISH_FAILURES_TOTAL.clone()))
        .expect("dlq_publish_failures_total registration must succeed");
    REGISTRY
        .register(Box::new(PROCESSING_LATENCY.clone()))
        .expect("processing_latency registration must succeed");
    REGISTRY
        .register(Box::new(KAFKA_PUBLISH_LATENCY.clone()))
        .expect("kafka_publish_latency registration must succeed");
    REGISTRY
        .register(Box::new(DATABASE_INSERT_LATENCY.clone()))
        .expect("database_insert_latency registration must succeed");
}
