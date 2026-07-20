-- =============================================================================
-- Migration: Usage Metering, Billing, Alert Rules, and Webhooks
-- Version: 20260720000001
-- Description: Adds tables for daily usage metering, invoices,
--              alert notification rules, and webhook log trails.
-- =============================================================================

-- 1. Daily Usage Metering Per Tenant
CREATE TABLE IF NOT EXISTS tenant_usage_daily (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    metric_date DATE NOT NULL,
    telemetry_count BIGINT NOT NULL DEFAULT 0,
    api_call_count BIGINT NOT NULL DEFAULT 0,
    storage_mb DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    active_satellites INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, metric_date)
);

CREATE INDEX IF NOT EXISTS idx_usage_tenant_date ON tenant_usage_daily (tenant_id, metric_date DESC);

-- 2. Tenant Invoices & Billing Logs
CREATE TABLE IF NOT EXISTS tenant_invoices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    billing_period_start DATE NOT NULL,
    billing_period_end DATE NOT NULL,
    telemetry_volume_billable BIGINT NOT NULL DEFAULT 0,
    storage_mb_billable DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    total_amount_inr DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    status VARCHAR(20) NOT NULL DEFAULT 'DRAFT'
        CHECK (status IN ('DRAFT', 'ISSUED', 'PAID', 'OVERDUE', 'CANCELLED')),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_invoices_tenant ON tenant_invoices (tenant_id);

-- 3. Tenant Alert Notification Rules
CREATE TABLE IF NOT EXISTS tenant_alert_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    rule_name VARCHAR(255) NOT NULL,
    satellite_id VARCHAR(50), -- NULL means all satellites in tenant
    metric_name VARCHAR(100) NOT NULL, -- e.g. 'temperature', 'battery_voltage', 'pc'
    condition VARCHAR(10) NOT NULL CHECK (condition IN ('>', '>=', '<', '<=', '==', '!=')),
    threshold DOUBLE PRECISION NOT NULL,
    severity VARCHAR(20) NOT NULL DEFAULT 'WARNING'
        CHECK (severity IN ('INFO', 'WARNING', 'CRITICAL')),
    action_channel VARCHAR(30) NOT NULL DEFAULT 'WEBHOOK'
        CHECK (action_channel IN ('WEBHOOK', 'SLACK', 'EMAIL', 'PAGERDUTY')),
    webhook_url TEXT,
    secret_token VARCHAR(255), -- Used for HMAC-SHA256 signature
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alert_rules_tenant ON tenant_alert_rules (tenant_id);

-- 4. Alert Notification Audit & Webhook Dispatch Logs
CREATE TABLE IF NOT EXISTS alert_dispatch_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id UUID REFERENCES tenant_alert_rules(id) ON DELETE SET NULL,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    satellite_id VARCHAR(50) NOT NULL,
    metric_name VARCHAR(100) NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    channel VARCHAR(30) NOT NULL,
    destination TEXT NOT NULL,
    status_code INTEGER,
    response_body TEXT,
    dispatched_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alert_logs_tenant ON alert_dispatch_logs (tenant_id, dispatched_at DESC);
