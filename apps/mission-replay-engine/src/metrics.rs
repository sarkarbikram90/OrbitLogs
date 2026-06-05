use lazy_static::lazy_static;
use prometheus::{CounterVec, Gauge, Opts, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref ACTIVE_REPLAYS: Gauge = Gauge::new(
        "active_replay_sessions",
        "Number of active replay sessions currently playing"
    )
    .expect("active_replay_sessions metric must be created");

    pub static ref REPLAY_EVENTS_STREAMED: CounterVec = CounterVec::new(
        Opts::new(
            "replay_events_streamed_total",
            "Total number of events streamed during replay"
        ),
        &["session_id", "data_type"]
    )
    .expect("replay_events_streamed_total metric must be created");

    pub static ref REPLAY_CONTROLS: CounterVec = CounterVec::new(
        Opts::new(
            "replay_controls_total",
            "Total number of control actions received"
        ),
        &["action"]
    )
    .expect("replay_controls_total metric must be created");
}

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
