use prometheus::{Gauge, GaugeVec, Opts, Registry};
use std::sync::LazyLock;

pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);
pub static FLEET_HEALTH_SCORE: LazyLock<Gauge> = LazyLock::new(|| {
    Gauge::new(
        "fleet_health_score",
        "Average health score of the entire satellite fleet (0-100)",
    )
    .expect("fleet_health_score metric must be created")
});
pub static SATELLITE_HEALTH_SCORE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new(
            "satellite_health_score",
            "Health score of each individual satellite (0-100)",
        ),
        &["satellite_id"],
    )
    .expect("satellite_health_score metric must be created")
});
pub static SUBSYSTEM_HEALTH_SCORE: LazyLock<GaugeVec> = LazyLock::new(|| {
    GaugeVec::new(
        Opts::new(
            "subsystem_health_score",
            "Health score of each subsystem (0-100)",
        ),
        &["satellite_id", "subsystem_id"],
    )
    .expect("subsystem_health_score metric must be created")
});
pub use orbitlogs_models::kafka::KAFKA_CONSUMER_LAG;

pub fn register_metrics() {
    REGISTRY
        .register(Box::new(FLEET_HEALTH_SCORE.clone()))
        .expect("fleet_health_score registration must succeed");
    REGISTRY
        .register(Box::new(SATELLITE_HEALTH_SCORE.clone()))
        .expect("satellite_health_score registration must succeed");
    REGISTRY
        .register(Box::new(SUBSYSTEM_HEALTH_SCORE.clone()))
        .expect("subsystem_health_score registration must succeed");
    REGISTRY
        .register(Box::new(KAFKA_CONSUMER_LAG.clone()))
        .expect("kafka_consumer_lag registration must succeed");
}
