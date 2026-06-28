

use std::sync::LazyLock;
use prometheus::{GaugeVec, Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts, Registry};

pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);
pub static TELEMETRY_RECEIVED_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    IntCounterVec::new(
        Opts::new(
            "Telemetry_received_total",
            "Total number of telemetry events received by the HTTP collector endpoint"
        ),
        &["satellite_id", "status"]
    )
    .expect("Telemetry_received_total metric must be created")
});
pub static TELEMETRY_PROCESSED_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    IntCounterVec::new(
        Opts::new(
            "Telemetry_processed_total",
            "Total number of telemetry events successfully processed by the consumer"
        ),
        &["satellite_id"]
    )
    .expect("Telemetry_processed_total metric must be created")
});
pub static TELEMETRY_INVALID_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    IntCounterVec::new(
        Opts::new(
            "Telemetry_invalid_total",
            "Total number of telemetry events rejected due to schema/validation errors"
        ),
        &["reason"]
    )
    .expect("Telemetry_invalid_total metric must be created")
});

// A Dead Letter Queue (DLQ) is a special queue where messages are sent
// after they cannot be processed successfully, instead of being lost or retried forever.

pub static DLQ_PUBLISH_FAILURES_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    IntCounter::new(
        "DLQ_Publish_failures_total",
        "Total number of failed attempts to publish messages to the dead letter queue"
    )
    .expect("DLQ_Publish_failures_total metric must be created")
});
pub static PROCESSING_LATENCY: LazyLock<Histogram> = LazyLock::new(|| {
    Histogram::with_opts(
        HistogramOpts::new(
            "Processing_latency",
            "Time taken to validate, enrich and queue incoming telemetry (seconds)"
        )
        .buckets(vec![
            0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0
        ])
    )
    .expect("Processing_latency metric must be created")
});
pub static KAFKA_PUBLISH_LATENCY: LazyLock<Histogram> = LazyLock::new(|| {
    Histogram::with_opts(
        HistogramOpts::new(
            "Kafka_publish_latency",
            "Time taken to publish message to Kafka topic (seconds)"
        )
        .buckets(vec![
            0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0
        ])
    )
    .expect("Kafka_publish_latency metric must be created")
});
pub static DATABASE_INSERT_LATENCY: LazyLock<Histogram> = LazyLock::new(|| {
    Histogram::with_opts(
        HistogramOpts::new(
            "Database_insert_latency",
            "Time taken to write telemetry payload to TimescaleDB (seconds)"
        )
        .buckets(vec![
            0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0
        ])
    )
    .expect("Database_insert_latency metric must be created")
});
pub static KAFKA_CONSUMER_LAG: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new(
            "Kafka_consumer_lag",
            "Lag of the Kafka consumer group (offsets behind high watermark)"
        ),
        &["topic", "partition"]
    )
    .expect("Kafka_consumer_lag metric must be created")
});

pub fn register_metrics() {
    REGISTRY
        .register(Box::new(TELEMETRY_RECEIVED_TOTAL.clone()))
        .expect("Telemetry_received_total registration must succeed");
    REGISTRY
        .register(Box::new(TELEMETRY_PROCESSED_TOTAL.clone()))
        .expect("Telemetry_processed_total registration must succeed");
    REGISTRY
        .register(Box::new(TELEMETRY_INVALID_TOTAL.clone()))
        .expect("Telemetry_invalid_total registration must succeed");
    REGISTRY
        .register(Box::new(DLQ_PUBLISH_FAILURES_TOTAL.clone()))
        .expect("DLQ_Publish_failures_total registration must succeed");
    REGISTRY
        .register(Box::new(PROCESSING_LATENCY.clone()))
        .expect("Processing_latency registration must succeed");
    REGISTRY
        .register(Box::new(KAFKA_PUBLISH_LATENCY.clone()))
        .expect("Kafka_publish_latency registration must succeed");
    REGISTRY
        .register(Box::new(DATABASE_INSERT_LATENCY.clone()))
        .expect("Database_insert_latency registration must succeed");
    REGISTRY
        .register(Box::new(KAFKA_CONSUMER_LAG.clone()))
        .expect("Kafka_consumer_lag registration must succeed");
}
