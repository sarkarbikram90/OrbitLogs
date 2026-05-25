use lazy_static::lazy_static;
use prometheus::{Histogram, HistogramOpts, IntCounterVec, Opts, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref TELEMETRY_RECEIVED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "telemetry_received_total",
            "Total number of telemetry events received by the HTTP collector endpoint"
        ),
        &["satellite_id", "status"]
    )
    .unwrap();
    pub static ref TELEMETRY_PROCESSED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "telemetry_processed_total",
            "Total number of telemetry events successfully processed by the consumer"
        ),
        &["satellite_id"]
    )
    .unwrap();
    pub static ref TELEMETRY_INVALID_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "telemetry_invalid_total",
            "Total number of telemetry events rejected due to schema/validation errors"
        ),
        &["reason"]
    )
    .unwrap();
    pub static ref PROCESSING_LATENCY: Histogram = Histogram::with_opts(HistogramOpts::new(
        "processing_latency",
        "Time taken to validate, enrich and queue incoming telemetry (seconds)"
    ))
    .unwrap();
    pub static ref KAFKA_PUBLISH_LATENCY: Histogram = Histogram::with_opts(HistogramOpts::new(
        "kafka_publish_latency",
        "Time taken to publish message to Kafka topic (seconds)"
    ))
    .unwrap();
    pub static ref DATABASE_INSERT_LATENCY: Histogram = Histogram::with_opts(HistogramOpts::new(
        "database_insert_latency",
        "Time taken to write telemetry payload to TimescaleDB (seconds)"
    ))
    .unwrap();
}

pub fn register_metrics() {
    REGISTRY
        .register(Box::new(TELEMETRY_RECEIVED_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(TELEMETRY_PROCESSED_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(TELEMETRY_INVALID_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(PROCESSING_LATENCY.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(KAFKA_PUBLISH_LATENCY.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(DATABASE_INSERT_LATENCY.clone()))
        .unwrap();
}
