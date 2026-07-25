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

//! Prometheus metrics for the Digital Twin Engine.
//!
//! Provides constellation-scale observability: active twin count, message
//! throughput, deserialization errors, simulation failure tracking,
//! health projection latency, and HTTP request latency.

use prometheus::{
    register_counter_vec, register_gauge, register_gauge_vec, register_histogram,
    register_histogram_vec, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec,
};
use std::sync::LazyLock;

// -------------------------------------------------------------------------
// Twin State Metrics
// -------------------------------------------------------------------------

/// Number of satellites with an active in-memory digital twin.
pub static ACTIVE_TWINS: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "digital_twin_active_twins",
        "Number of satellites with an active digital twin state"
    )
    .expect("digital_twin_active_twins must register")
});

// -------------------------------------------------------------------------
// Kafka Consumer Metrics
// -------------------------------------------------------------------------

/// Total telemetry messages successfully consumed and processed.
pub static TELEMETRY_PROCESSED_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec!(
        "digital_twin_telemetry_processed_total",
        "Telemetry messages consumed and applied to twin state",
        &["satellite_id"]
    )
    .expect("digital_twin_telemetry_processed_total must register")
});

/// Total telemetry messages that failed JSON deserialization.
pub static DESERIALIZATION_ERRORS_TOTAL: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "digital_twin_telemetry_deserialization_errors_total",
        "Telemetry messages that failed deserialization"
    )
    .expect("digital_twin_telemetry_deserialization_errors_total must register")
});

// -------------------------------------------------------------------------
// Failure Simulation Metrics
// -------------------------------------------------------------------------

/// Number of currently active simulated failure injections.
pub static SIMULATED_FAILURES_ACTIVE: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "digital_twin_simulated_failures_active",
        "Active simulated failure injections by satellite and subsystem",
        &["satellite_id", "subsystem"]
    )
    .expect("digital_twin_simulated_failures_active must register")
});

// -------------------------------------------------------------------------
// Health Projection Metrics
// -------------------------------------------------------------------------

/// Histogram of time spent computing health projections (seconds).
pub static HEALTH_PROJECTION_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "digital_twin_health_projection_duration_seconds",
        "Time spent computing a twin health projection",
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    )
    .expect("digital_twin_health_projection_duration_seconds must register")
});

// -------------------------------------------------------------------------
// HTTP API Metrics
// -------------------------------------------------------------------------

/// Histogram of HTTP request durations (seconds), labelled by method, path, and
/// response status code.
pub static HTTP_REQUEST_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        "digital_twin_api_request_duration_seconds",
        "HTTP request latency for the Digital Twin Engine API",
        &["method", "path", "status"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .expect("digital_twin_api_request_duration_seconds must register")
});

/// Force-initialise all lazy metrics so they appear in `/metrics` output
/// even before the first observation.
pub fn init_metrics() {
    LazyLock::force(&ACTIVE_TWINS);
    LazyLock::force(&TELEMETRY_PROCESSED_TOTAL);
    LazyLock::force(&DESERIALIZATION_ERRORS_TOTAL);
    LazyLock::force(&SIMULATED_FAILURES_ACTIVE);
    LazyLock::force(&HEALTH_PROJECTION_DURATION);
    LazyLock::force(&HTTP_REQUEST_DURATION);
}
