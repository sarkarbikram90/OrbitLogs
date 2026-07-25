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

//! Background Kafka consumer for the Digital Twin Engine.
//!
//! Subscribes to the processed-telemetry topic, deserialises each record,
//! applies any active simulated failures, updates the recursive
//! `DigitalTwinState` estimators, and stores the enriched twin snapshot
//! in shared memory.

use crate::metrics;
use crate::simulation::DigitalTwinState;
use crate::AppState;
use orbitlogs_models::kafka::KafkaConfigBuilder;
use orbitlogs_models::TelemetryRecord;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Maximum consecutive consumer errors before we apply a longer back-off.
const MAX_CONSECUTIVE_ERRORS: u32 = 10;

/// Runs the telemetry consumer loop until the cancellation token fires.
///
/// This function is designed to be spawned as a background `tokio::task`.
/// It will log and count deserialization errors without crashing, apply
/// exponential back-off on broker-level errors, and cleanly shut down on
/// cancellation.
pub async fn run_telemetry_consumer(
    state: Arc<AppState>,
    brokers: String,
    topic: String,
    group_id: String,
    cancel_token: CancellationToken,
) -> Result<(), anyhow::Error> {
    let mut config = KafkaConfigBuilder::from_env(brokers).build();
    config
        .set("group.id", &group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest");

    let consumer: StreamConsumer = config.create()?;
    consumer.subscribe(&[&topic])?;

    info!(
        topic = %topic,
        group_id = %group_id,
        "Digital twin background consumer subscribed"
    );

    let mut consecutive_errors: u32 = 0;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Digital twin background consumer stopping (cancellation received)");
                break;
            }
            msg_result = consumer.recv() => {
                match msg_result {
                    Ok(msg) => {
                        consecutive_errors = 0;
                        process_message(&state, &msg, &consumer);
                    }
                    Err(e) => {
                        consecutive_errors = consecutive_errors.saturating_add(1);
                        let backoff_ms = calculate_backoff(consecutive_errors);
                        error!(
                            error = %e,
                            consecutive_errors,
                            backoff_ms,
                            "Error receiving message from Kafka"
                        );
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Processes a single Kafka message: deserialise → apply failures → update
/// twin state → store snapshot.
fn process_message(
    state: &AppState,
    msg: &rdkafka::message::BorrowedMessage<'_>,
    consumer: &StreamConsumer,
) {
    let payload = match msg.payload_view::<str>() {
        Some(Ok(s)) => s,
        Some(Err(e)) => {
            warn!(error = %e, "Non-UTF-8 Kafka message payload");
            metrics::DESERIALIZATION_ERRORS_TOTAL.inc();
            // Commit to avoid reprocessing a poison pill forever.
            let _ = consumer.commit_message(msg, CommitMode::Async);
            return;
        }
        None => {
            // Null/empty payload — skip silently.
            let _ = consumer.commit_message(msg, CommitMode::Async);
            return;
        }
    };

    let mut rec = match serde_json::from_str::<TelemetryRecord>(payload) {
        Ok(r) => r,
        Err(e) => {
            warn!(
                error = %e,
                payload_len = payload.len(),
                "Failed to deserialise telemetry record"
            );
            metrics::DESERIALIZATION_ERRORS_TOTAL.inc();
            let _ = consumer.commit_message(msg, CommitMode::Async);
            return;
        }
    };

    let satellite_id = rec.satellite_id.to_string();

    // ---- Apply active simulated failures ----
    apply_simulated_failures(state, &mut rec);

    // ---- Update recursive DigitalTwinState estimators ----
    update_twin_state(state, &satellite_id, &rec);

    // ---- Store the enriched twin snapshot ----
    let is_new = !state.twins.contains_key(&satellite_id);
    state.twins.insert(satellite_id.clone(), rec);

    if is_new {
        metrics::ACTIVE_TWINS.inc();
    }
    metrics::TELEMETRY_PROCESSED_TOTAL
        .with_label_values(&[&satellite_id])
        .inc();

    // Commit after successful processing.
    if let Err(e) = consumer.commit_message(msg, CommitMode::Async) {
        warn!(error = %e, "Failed to commit Kafka offset");
    }
}

/// Mutates the telemetry record by overlaying any active simulated failures
/// for the record's satellite.
fn apply_simulated_failures(state: &AppState, rec: &mut TelemetryRecord) {
    if let Some(failures) = state.simulated_failures.get(rec.satellite_id.as_str()) {
        for sub in failures.value() {
            match sub.as_str() {
                "power" => {
                    rec.battery_voltage = Some(rec.battery_voltage.unwrap_or(12.0) * 0.75);
                    rec.solar_panel_output = Some(0.0);
                }
                "thermal" => {
                    rec.temperature = Some(rec.temperature.unwrap_or(25.0) + 65.0);
                }
                "adcs" => {
                    rec.orientation_pitch = Some(rec.orientation_pitch.unwrap_or(0.0) + 15.0);
                }
                "communications" => {
                    rec.packet_loss = Some(95.0);
                    rec.communication_latency = Some(5000.0);
                }
                "propulsion" => {
                    rec.fuel_remaining = Some(rec.fuel_remaining.unwrap_or(10.0) * 0.1);
                }
                _ => {}
            }
        }
    }
}

/// Updates the `DigitalTwinState` recursive estimators (solar efficiency,
/// battery capacity fade, radiator thermal resistance) using the latest
/// telemetry snapshot.
fn update_twin_state(state: &AppState, satellite_id: &str, rec: &TelemetryRecord) {
    let mut twin = state
        .twin_states
        .entry(satellite_id.to_string())
        .or_insert_with(|| DigitalTwinState::new(satellite_id.to_string()));

    // Solar efficiency estimation
    if let Some(solar_w) = rec.solar_panel_output {
        // Use a nominal exposure factor of 1.0 for now (could be computed
        // from orbital position in a future iteration).
        twin.estimate_solar_efficiency(solar_w as f32, 1.0);
    }

    // Battery capacity fade estimation
    if let (Some(current), Some(voltage)) = (rec.battery_current, rec.battery_voltage) {
        // Approximate voltage drop as deviation from nominal 14.0V
        let voltage_drop = (14.0 - voltage).max(0.0) as f32;
        twin.estimate_battery_capacity_fade(current as f32, voltage_drop);
    }

    // Radiator thermal resistance estimation
    if let (Some(temp), Some(cpu)) = (rec.temperature, rec.cpu_usage) {
        // Approximate CPU power draw from usage percentage.
        // Nominal max CPU power draw ≈ 30W for a satellite flight computer.
        let cpu_power_w = (cpu / 100.0) * 30.0;
        twin.estimate_radiator_thermal_resistance(temp as f32, cpu_power_w);
    }
}

/// Calculates exponential back-off duration in milliseconds for consumer errors.
///
/// Uses 500ms * 2^(n-1) capped at 30 seconds.
fn calculate_backoff(consecutive_errors: u32) -> u64 {
    let capped = consecutive_errors.min(MAX_CONSECUTIVE_ERRORS);
    let base = 500u64.saturating_mul(1u64 << capped.saturating_sub(1));
    base.min(30_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_increases_exponentially() {
        assert_eq!(calculate_backoff(1), 500);
        assert_eq!(calculate_backoff(2), 1000);
        assert_eq!(calculate_backoff(3), 2000);
        assert_eq!(calculate_backoff(4), 4000);
    }

    #[test]
    fn test_backoff_caps_at_30_seconds() {
        assert!(calculate_backoff(20) <= 30_000);
        assert!(calculate_backoff(100) <= 30_000);
    }

    #[test]
    fn test_backoff_zero_errors() {
        // 0 consecutive errors shouldn't happen in practice, but guard it.
        assert_eq!(calculate_backoff(0), 500);
    }
}
