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

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

pub mod metering;

#[derive(Parser, Debug)]
#[command(name = "Telemetry Metering & Billing Service")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8086")]
    port: u16,

    #[arg(long, env = "DATABASE_URL", default_value = "")]
    database_url: String,

    #[arg(
        long,
        env = "OTEL_EXPORTER_OTLP_ENDPOINT",
        default_value = "http://localhost:4317"
    )]
    otel_endpoint: String,
}

pub struct AppState {
    pub db_pool: PgPool,
}

async fn get_usage_handler(
    State(state): State<Arc<AppState>>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<metering::TenantUsageSummary>>, (StatusCode, String)> {
    metering::get_tenant_usage(&state.db_pool, tenant_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn record_usage_handler(
    State(state): State<Arc<AppState>>,
    Path(tenant_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, (StatusCode, String)> {
    let count = payload["count"].as_i64().unwrap_or(1);
    metering::record_telemetry_event(&state.db_pool, tenant_id, count)
        .await
        .map(|_| StatusCode::ACCEPTED)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    orbitlogs_models::telemetry::init_telemetry("telemetry-metering-service", &cli.otel_endpoint)?;
    info!("Starting Telemetry Metering & Billing Service...");

    let db_url = if !cli.database_url.trim().is_empty() {
        cli.database_url.clone()
    } else if let Some(val) =
        orbitlogs_models::load_secret("DATABASE_URL").filter(|s| !s.trim().is_empty())
    {
        val
    } else {
        return Err(anyhow::anyhow!("DATABASE_URL is required."));
    };

    let db_pool = orbitlogs_models::db::establish_db_connection(&db_url, 3).await?;
    let state = Arc::new(AppState { db_pool });

    let app = Router::new()
        .route("/api/metering/tenants/:tenant_id/usage", get(get_usage_handler))
        .route("/api/metering/tenants/:tenant_id/record", post(record_usage_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Telemetry Metering Service listening on http://localhost:{}", cli.port);

    axum::serve(listener, app).await?;
    Ok(())
}
