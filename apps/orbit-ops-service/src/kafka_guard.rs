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
use rdkafka::consumer::{Consumer, StreamConsumer};
use std::ops::Deref;

/// A guard for Kafka consumer that ensures unsubscription on drop (RAII pattern) (P1 #5).
/// Implements Deref to allow direct usage of the inner StreamConsumer.
pub struct KafkaConsumerGuard {
    pub consumer: StreamConsumer,
    pub topic: String,
}

impl Deref for KafkaConsumerGuard {
    type Target = StreamConsumer;

    fn deref(&self) -> &Self::Target {
        &self.consumer
    }
}

impl Drop for KafkaConsumerGuard {
    fn drop(&mut self) {
        tracing::info!(
            topic = %self.topic,
            "Unsubscribing Kafka consumer on drop"
        );
        self.consumer.unsubscribe();
    }
}
