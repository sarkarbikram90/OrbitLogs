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
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, instrument};

const PRODUCE_TIMEOUT: Duration = Duration::from_secs(5);

#[instrument(
    skip(producer, rec),
    fields(
        topic = %topic,
        key = %key
    )
)]
pub async fn send_telemetry(
    producer: Arc<FutureProducer>,
    topic: &str,
    key: &str,
    rec: &TelemetryRecord,
) {
    let payload = match serde_json::to_string(rec) {
        Ok(payload) => payload,
        Err(err) => {
            error!("Failed to serialize telemetry: {}", err);
            return;
        }
    };

    let record = FutureRecord::to(topic).key(key).payload(&payload);

    match producer.send(record, PRODUCE_TIMEOUT).await {
        Ok(delivery) => {
            debug!("Telemetry delivered: {:?}", delivery);
        }

        Err((err, _)) => {
            error!("Kafka send failed: {}", err);
        }
    }
}
