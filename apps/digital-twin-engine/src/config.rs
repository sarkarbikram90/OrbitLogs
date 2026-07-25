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

//! Configuration module for the Digital Twin Engine.
//!
//! Centralises all CLI flags and environment variable parsing into a single
//! validated configuration struct. This keeps `main.rs` focused on wiring
//! and lifecycle, not parsing.

use clap::Parser;

/// CLI / environment configuration for the Digital Twin Engine.
///
/// Every field can be set via CLI flag **or** the corresponding env var.
/// The engine will refuse to start if required values (like `API_KEY`) are
/// missing and cannot be resolved from Docker secrets either.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "Digital Twin Engine",
    about = "Virtual representation of spacecraft status, health, and simulation",
    long_about = None
)]
pub struct Config {
    /// Port the primary Axum HTTP server listens on.
    #[arg(long, env = "PORT", default_value = "8086")]
    pub port: u16,

    /// Port for the Kubernetes health probe server (`/healthz`, `/readyz`).
    #[arg(long, env = "HEALTH_PORT", default_value = "8087")]
    pub health_port: u16,

    /// Comma-separated list of Kafka broker endpoints.
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    pub kafka_brokers: String,

    /// Postgres / TimescaleDB connection string.
    #[arg(long, env = "DATABASE_URL", default_value = "")]
    pub database_url: String,

    /// OpenTelemetry OTLP collector endpoint for distributed tracing.
    #[arg(
        long,
        env = "OTEL_EXPORTER_OTLP_ENDPOINT",
        default_value = "http://localhost:4317"
    )]
    pub otel_endpoint: String,

    /// Kafka topic to consume processed telemetry from.
    #[arg(long, env = "PROCESSED_TOPIC", default_value = "telemetry.processed")]
    pub processed_topic: String,

    /// Target Kafka topic for timeline event notifications.
    #[arg(long, env = "TIMELINE_EVENTS_TOPIC", default_value = "timeline.events")]
    pub timeline_topic: String,

    /// Kafka consumer group ID.
    #[arg(long, env = "GROUP_ID", default_value = "digital-twin-group")]
    pub group_id: String,

    /// Bearer token for authenticated endpoints. Falls back to Docker
    /// secrets (`API_KEY`) if not set via flag or env.
    #[arg(long, env = "API_KEY")]
    pub api_key: Option<String>,

    /// Allowed CORS origin for browser-based clients.
    #[arg(
        long,
        env = "CORS_ALLOWED_ORIGIN",
        default_value = "http://localhost:3000"
    )]
    pub cors_allowed_origin: String,
}

impl Config {
    /// Resolves the API key from CLI flag, environment variable, or Docker
    /// secrets. Returns an error if none are available.
    pub fn resolve_api_key(&self) -> Result<String, anyhow::Error> {
        self.api_key
            .clone()
            .or_else(|| orbitlogs_models::load_secret("API_KEY"))
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "API key is required. Set env var API_KEY, pass --api-key, \
                     or use Docker secrets API_KEY."
                )
            })
    }
}
