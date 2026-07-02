use prometheus::{CounterVec, Gauge, Opts, Registry};
use std::sync::LazyLock;

pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);
pub static ACTIVE_REPLAYS: LazyLock<Gauge> = LazyLock::new(|| {
    Gauge::new(
        "active_replay_sessions",
        "Number of active replay sessions currently playing",
    )
    .expect("active_replay_sessions metric must be created")
});
pub static REPLAY_EVENTS_STREAMED: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new(
            "replay_events_streamed_total",
            "Total number of events streamed during replay",
        ),
        &["session_id", "data_type"],
    )
    .expect("replay_events_streamed_total metric must be created")
});
pub static REPLAY_CONTROLS: LazyLock<CounterVec> = LazyLock::new(|| {
    CounterVec::new(
        Opts::new(
            "replay_controls_total",
            "Total number of control actions received",
        ),
        &["action"],
    )
    .expect("replay_controls_total metric must be created")
});

pub fn register_metrics() {
    REGISTRY
        .register(Box::new(ACTIVE_REPLAYS.clone()))
        .expect("active_replays registration must succeed");
    REGISTRY
        .register(Box::new(REPLAY_EVENTS_STREAMED.clone()))
        .expect("replay_events_streamed registration must succeed");
    REGISTRY
        .register(Box::new(REPLAY_CONTROLS.clone()))
        .expect("replay_controls registration must succeed");
}
