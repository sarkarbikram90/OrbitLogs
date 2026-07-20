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
use reqwest::Client;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

pub mod webhooks;

#[derive(Parser, Debug)]
#[command(name = "Alert Notification & Webhook Service")]
struct Cli {
    #[arg(long, env = "PORT", default_value = "8087")]
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
    pub http_client: Client,
}

#[derive(serde::Deserialize)]
pub struct CreateRuleRequest {
    pub rule_name: String,
    pub satellite_id: Option<String>,
    pub metric_name: String,
    pub condition: String,
    pub threshold: f64,
    pub severity: Option<String>,
    pub action_channel: Option<String>,
    pub webhook_url: Option<String>,
    pub secret_token: Option<String>,
}

async fn create_rule_handler(
    State(state): State<Arc<AppState>>,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<CreateRuleRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let id = Uuid::new_v4();
    let severity = req.severity.as_deref().unwrap_or("WARNING");
    let channel = req.action_channel.as_deref().unwrap_or("WEBHOOK");

    sqlx::query(
        "INSERT INTO tenant_alert_rules \
         (id, tenant_id, rule_name, satellite_id, metric_name, condition, threshold, severity, action_channel, webhook_url, secret_token) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(&req.rule_name)
    .bind(&req.satellite_id)
    .bind(&req.metric_name)
    .bind(&req.condition)
    .bind(req.threshold)
    .bind(severity)
    .bind(channel)
    .bind(&req.webhook_url)
    .bind(&req.secret_token)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id, "status": "created" }))))
}

async fn list_rules_handler(
    State(state): State<Arc<AppState>>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<webhooks::AlertRule>>, (StatusCode, String)> {
    let rows = sqlx::query(
        "SELECT id, tenant_id, rule_name, satellite_id, metric_name, condition, threshold, severity, action_channel, webhook_url, secret_token, is_active \
         FROM tenant_alert_rules WHERE tenant_id = $1 ORDER BY created_at DESC",
    )
    .bind(tenant_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rules = rows
        .iter()
        .map(|r| webhooks::AlertRule {
            id: r.get("id"),
            tenant_id: r.get("tenant_id"),
            rule_name: r.get("rule_name"),
            satellite_id: r.get("satellite_id"),
            metric_name: r.get("metric_name"),
            condition: r.get("condition"),
            threshold: r.get("threshold"),
            severity: r.get("severity"),
            action_channel: r.get("action_channel"),
            webhook_url: r.get("webhook_url"),
            secret_token: r.get("secret_token"),
            is_active: r.get("is_active"),
        })
        .collect();

    use sqlx::Row;
    Ok(Json(rules))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    orbitlogs_models::telemetry::init_telemetry("alert-notification-service", &cli.otel_endpoint)?;
    info!("Starting Alert Notification & Webhook Service...");

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
    let http_client = Client::new();
    let state = Arc::new(AppState { db_pool, http_client });

    let app = Router::new()
        .route("/api/alerts/tenants/:tenant_id/rules", get(list_rules_handler).post(create_rule_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Alert Notification Service listening on http://localhost:{}", cli.port);

    axum::serve(listener, app).await?;
    Ok(())
}
