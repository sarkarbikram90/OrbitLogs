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
use orbitlogs_models::TelemetryRecord;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

pub fn send_telemetry(
    producer: Arc<FutureProducer>,
    topic: &str,
    key: String,
    rec: TelemetryRecord,
) {
    if let Ok(json_payload) = serde_json::to_string(&rec) {
        let topic_owned = topic.to_string();
        tokio::spawn(async move {
            let record = FutureRecord::to(&topic_owned)
                .key(&key)
                .payload(&json_payload);
            if let Err((e, _)) = producer.send(record, Duration::from_secs(5)).await {
                error!("Failed to produce to topic {}: {}", topic_owned, e);
            }
        });
    }
}
