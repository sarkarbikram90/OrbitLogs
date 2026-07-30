-- =============================================================================
-- Migration: India GST Compliance & Dynamic Plan Pricing
-- Version: 20260730000000
-- Description: Adds GSTIN, SAC code, state codes, tax amounts (CGST/SGST/IGST),
--              and sequential GST-compliant invoice numbering for Indian SaaS billing.
-- =============================================================================

-- 1. Extend tenants table with GST & Address metadata
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS gstin VARCHAR(15);
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS state_code VARCHAR(2) DEFAULT '27'; -- Default 27 (Maharashtra) / 29 (Karnataka)
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS billing_address TEXT;
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS sac_code VARCHAR(10) DEFAULT '998314'; -- SAC Code for Information Technology / SaaS

-- 2. Extend tenant_invoices with GST tax breakdown & GST sequential invoice numbers
ALTER TABLE tenant_invoices ADD COLUMN IF NOT EXISTS invoice_number VARCHAR(30);
ALTER TABLE tenant_invoices ADD COLUMN IF NOT EXISTS sac_code VARCHAR(10) DEFAULT '998314';
ALTER TABLE tenant_invoices ADD COLUMN IF NOT EXISTS base_amount_inr DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE tenant_invoices ADD COLUMN IF NOT EXISTS cgst_amount_inr DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE tenant_invoices ADD COLUMN IF NOT EXISTS sgst_amount_inr DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE tenant_invoices ADD COLUMN IF NOT EXISTS igst_amount_inr DOUBLE PRECISION NOT NULL DEFAULT 0.0;
ALTER TABLE tenant_invoices ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'INR';

CREATE UNIQUE INDEX IF NOT EXISTS idx_invoices_number ON tenant_invoices (invoice_number) WHERE invoice_number IS NOT NULL;
