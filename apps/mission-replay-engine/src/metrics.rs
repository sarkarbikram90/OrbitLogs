// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
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
