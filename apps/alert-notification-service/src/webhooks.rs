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

//! Webhook and alert notification dispatcher with HMAC-SHA256 signature calculation.

use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::{PgPool, Row};
use tracing::{error, info};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub rule_name: String,
    pub satellite_id: Option<String>,
    pub metric_name: String,
    pub condition: String,
    pub threshold: f64,
    pub severity: String,
    pub action_channel: String,
    pub webhook_url: Option<String>,
    pub secret_token: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub alert_id: Uuid,
    pub tenant_id: Uuid,
    pub satellite_id: String,
    pub metric_name: String,
    pub metric_value: f64,
    pub condition: String,
    pub threshold: f64,
    pub severity: String,
    pub timestamp: String,
    pub message: String,
}

pub fn calculate_hmac_signature(secret: &str, body: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn evaluate_condition(value: f64, condition: &str, threshold: f64) -> bool {
    match condition {
        ">" => value > threshold,
        ">=" => value >= threshold,
        "<" => value < threshold,
        "<=" => value <= threshold,
        "==" => (value - threshold).abs() < 1e-6,
        "!=" => (value - threshold).abs() >= 1e-6,
        _ => false,
    }
}

pub async fn dispatch_webhook(
    client: &Client,
    url: &str,
    secret: Option<&str>,
    payload: &WebhookPayload,
) -> Result<u16, String> {
    let body_json = serde_json::to_string(payload).map_err(|e| e.to_string())?;

    let mut req = client.post(url).header("Content-Type", "application/json");

    if let Some(secret_str) = secret {
        let sig = calculate_hmac_signature(secret_str, &body_json);
        req = req.header("X-OrbitLogs-Signature", format!("sha256={}", sig));
    }

    let resp = req
        .body(body_json)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("HTTP post failed: {}", e))?;

    Ok(resp.status().as_u16())
}

pub async fn fetch_active_rules(
    pool: &PgPool,
    tenant_id: Uuid,
    metric_name: &str,
) -> Result<Vec<AlertRule>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, tenant_id, rule_name, satellite_id, metric_name, condition, \
         threshold, severity, action_channel, webhook_url, secret_token, is_active \
         FROM tenant_alert_rules \
         WHERE tenant_id = $1 AND metric_name = $2 AND is_active = TRUE",
    )
    .bind(tenant_id)
    .bind(metric_name)
    .fetch_all(pool)
    .await?;

    let rules = rows
        .iter()
        .map(|r| AlertRule {
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

    Ok(rules)
}

pub async fn log_dispatch_attempt(
    pool: &PgPool,
    rule_id: Uuid,
    tenant_id: Uuid,
    satellite_id: &str,
    metric_name: &str,
    metric_value: f64,
    channel: &str,
    destination: &str,
    status_code: Option<u16>,
    response_body: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO alert_dispatch_logs \
         (rule_id, tenant_id, satellite_id, metric_name, metric_value, channel, destination, status_code, response_body) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(rule_id)
    .bind(tenant_id)
    .bind(satellite_id)
    .bind(metric_name)
    .bind(metric_value)
    .bind(channel)
    .bind(destination)
    .bind(status_code.map(|c| c as i32))
    .bind(response_body)
    .execute(pool)
    .await?;

    Ok(())
}
