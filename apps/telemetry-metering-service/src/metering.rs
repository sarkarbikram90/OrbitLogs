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

//! Metering engine logic — tracks per-tenant telemetry volume, API calls, storage MB, and generates invoices in INR.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsageSummary {
    pub tenant_id: Uuid,
    pub metric_date: NaiveDate,
    pub telemetry_count: i64,
    pub api_call_count: i64,
    pub storage_mb: f64,
    pub active_satellites: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub telemetry_volume_billable: i64,
    pub storage_mb_billable: f64,
    pub total_amount_inr: f64,
    pub status: String,
}

/// Pricing Tiers in INR (₹)
pub const COST_PER_1K_TELEMETRY_INR: f64 = 0.50; // ₹0.50 per 1,000 telemetry events
pub const COST_PER_GB_STORAGE_INR: f64 = 15.00;    // ₹15.00 per GB-month
pub const BASE_DEVELOPER_PLAN_INR: f64 = 4999.00;  // ₹4,999 / month base platform fee

pub async fn record_telemetry_event(
    pool: &PgPool,
    tenant_id: Uuid,
    count: i64,
) -> Result<(), sqlx::Error> {
    let today = chrono::Utc::now().date_naive();

    sqlx::query(
        "INSERT INTO tenant_usage_daily (tenant_id, metric_date, telemetry_count, updated_at) \
         VALUES ($1, $2, $3, NOW()) \
         ON CONFLICT (tenant_id, metric_date) DO UPDATE \
         SET telemetry_count = tenant_usage_daily.telemetry_count + EXCLUDED.telemetry_count, \
             updated_at = NOW()",
    )
    .bind(tenant_id)
    .bind(today)
    .bind(count)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_tenant_usage(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<TenantUsageSummary>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT tenant_id, metric_date, telemetry_count, api_call_count, storage_mb, active_satellites \
         FROM tenant_usage_daily WHERE tenant_id = $1 \
         ORDER BY metric_date DESC LIMIT 30",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    let summary = rows
        .iter()
        .map(|r| TenantUsageSummary {
            tenant_id: r.get("tenant_id"),
            metric_date: r.get("metric_date"),
            telemetry_count: r.get("telemetry_count"),
            api_call_count: r.get("api_call_count"),
            storage_mb: r.get("storage_mb"),
            active_satellites: r.get("active_satellites"),
        })
        .collect();

    Ok(summary)
}

pub async fn generate_monthly_invoice(
    pool: &PgPool,
    tenant_id: Uuid,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Invoice, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COALESCE(SUM(telemetry_count), 0) as total_telemetry, \
                COALESCE(AVG(storage_mb), 0.0) as avg_storage \
         FROM tenant_usage_daily \
         WHERE tenant_id = $1 AND metric_date >= $2 AND metric_date <= $3",
    )
    .bind(tenant_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;

    let total_telemetry: i64 = row.get("total_telemetry");
    let avg_storage: f64 = row.get("avg_storage");

    let telemetry_cost = (total_telemetry as f64 / 1000.0) * COST_PER_1K_TELEMETRY_INR;
    let storage_cost = (avg_storage / 1024.0) * COST_PER_GB_STORAGE_INR;
    let total_inr = BASE_DEVELOPER_PLAN_INR + telemetry_cost + storage_cost;

    let id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO tenant_invoices \
         (id, tenant_id, billing_period_start, billing_period_end, telemetry_volume_billable, storage_mb_billable, total_amount_inr, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'ISSUED')",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(start)
    .bind(end)
    .bind(total_telemetry)
    .bind(avg_storage)
    .bind(total_inr)
    .execute(pool)
    .await?;

    Ok(Invoice {
        id,
        tenant_id,
        billing_period_start: start,
        billing_period_end: end,
        telemetry_volume_billable: total_telemetry,
        storage_mb_billable: avg_storage,
        total_amount_inr: total_inr,
        status: "ISSUED".to_string(),
    })
}
