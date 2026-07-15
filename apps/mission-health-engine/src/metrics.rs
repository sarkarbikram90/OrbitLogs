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
