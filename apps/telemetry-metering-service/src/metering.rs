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
    pub invoice_number: String,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub telemetry_volume_billable: i64,
    pub storage_mb_billable: f64,
    pub base_amount_inr: f64,
    pub cgst_amount_inr: f64,
    pub sgst_amount_inr: f64,
    pub igst_amount_inr: f64,
    pub total_amount_inr: f64,
    pub currency: String,
    pub status: String,
}

/// Pricing Tiers in INR (₹)
pub const COST_PER_1K_TELEMETRY_INR: f64 = 0.50; // ₹0.50 per 1,000 telemetry events
pub const COST_PER_GB_STORAGE_INR: f64 = 15.00;    // ₹15.00 per GB-month
pub const BASE_DEVELOPER_PLAN_INR: f64 = 4999.00;  // ₹4,999 / month base platform fee
pub const GST_RATE: f64 = 0.18;                      // 18% GST for SaaS in India (SAC 998314)
pub const OPERATING_STATE_CODE: &str = "27";         // 27 = Maharashtra (OrbitLogs HQ)


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
    let base_amount_inr = BASE_DEVELOPER_PLAN_INR + telemetry_cost + storage_cost;

    // Fetch tenant state_code for GST determination (Intra-state = CGST+SGST, Inter-state = IGST)
    let tenant_state: String = sqlx::query_scalar(
        "SELECT COALESCE(state_code, '27') FROM tenants WHERE id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| "27".to_string());

    let (cgst, sgst, igst) = if tenant_state == OPERATING_STATE_CODE {
        // Intra-state supply: 9% CGST + 9% SGST
        let tax = (base_amount_inr * (GST_RATE / 2.0) * 100.0).round() / 100.0;
        (tax, tax, 0.0)
    } else {
        // Inter-state supply: 18% IGST
        let tax = (base_amount_inr * GST_RATE * 100.0).round() / 100.0;
        (0.0, 0.0, tax)
    };

    let total_amount_inr = base_amount_inr + cgst + sgst + igst;
    let id = Uuid::new_v4();
    let invoice_number = format!("OL-INV-{}-{}", start.format("%Y%m"), &id.to_string()[..8]);

    sqlx::query(
        "INSERT INTO tenant_invoices \
         (id, tenant_id, invoice_number, billing_period_start, billing_period_end, \
          telemetry_volume_billable, storage_mb_billable, base_amount_inr, \
          cgst_amount_inr, sgst_amount_inr, igst_amount_inr, total_amount_inr, currency, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'INR', 'ISSUED')",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(&invoice_number)
    .bind(start)
    .bind(end)
    .bind(total_telemetry)
    .bind(avg_storage)
    .bind(base_amount_inr)
    .bind(cgst)
    .bind(sgst)
    .bind(igst)
    .bind(total_amount_inr)
    .execute(pool)
    .await?;

    Ok(Invoice {
        id,
        tenant_id,
        invoice_number,
        billing_period_start: start,
        billing_period_end: end,
        telemetry_volume_billable: total_telemetry,
        storage_mb_billable: avg_storage,
        base_amount_inr,
        cgst_amount_inr: cgst,
        sgst_amount_inr: sgst,
        igst_amount_inr: igst,
        total_amount_inr,
        currency: "INR".to_string(),
        status: "ISSUED".to_string(),
    })
}
