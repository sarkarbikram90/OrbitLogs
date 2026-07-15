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
pub mod db;
pub mod processor;

use crate::server::producer::KafkaProducer;
pub use processor::{ConsumerConfig, TelemetryConsumer};
use sqlx::PgPool;
use tracing::info;

pub async fn start_consumer(
    config: ConsumerConfig,
    db_pool: PgPool,
    kafka_producer: KafkaProducer,
) -> Result<(), anyhow::Error> {
    info!("Starting Telemetry Consumer...");
    let store = std::sync::Arc::new(db::PostgresTelemetryStore::new(db_pool));
    let consumer = TelemetryConsumer::new(config, store, kafka_producer);
    consumer.start().await?;
    Ok(())
}
