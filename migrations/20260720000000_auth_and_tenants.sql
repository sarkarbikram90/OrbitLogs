-- =============================================================================
-- Migration: Auth, Multi-Tenancy, and Dynamic Fleet Registration
-- Version: 20260720000000
-- Description: Adds multi-tenant authentication, RBAC, and dynamic
--              satellite/ground station registration tables.
-- =============================================================================

-- 1. Tenants — Organization/company registration
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_name VARCHAR(255) NOT NULL,
    tenant_slug VARCHAR(100) NOT NULL UNIQUE,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE'
        CHECK (status IN ('ACTIVE', 'SUSPENDED', 'TRIAL', 'DEACTIVATED')),
    plan_tier VARCHAR(30) NOT NULL DEFAULT 'developer'
        CHECK (plan_tier IN ('developer', 'professional', 'enterprise', 'defense')),
    max_satellites INTEGER NOT NULL DEFAULT 5,
    max_ground_stations INTEGER NOT NULL DEFAULT 4,
    contact_email VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants (tenant_slug);

-- 2. Users — User accounts linked to tenants with role assignments
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    display_name VARCHAR(255) NOT NULL DEFAULT '',
    password_hash VARCHAR(512) NOT NULL DEFAULT '',
    role VARCHAR(30) NOT NULL DEFAULT 'Operator'
        CHECK (role IN ('Viewer', 'Operator', 'FlightDirector', 'MissionManager', 'TenantAdmin', 'SuperAdmin')),
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE'
        CHECK (status IN ('ACTIVE', 'DISABLED', 'PENDING')),
    last_login_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (email)
);

CREATE INDEX IF NOT EXISTS idx_users_tenant ON users (tenant_id);
CREATE INDEX IF NOT EXISTS idx_users_email ON users (email);

-- 3. API Keys — Tenant-scoped API keys for programmatic access
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    key_hash VARCHAR(512) NOT NULL,
    key_prefix VARCHAR(12) NOT NULL,
    label VARCHAR(100) NOT NULL DEFAULT 'default',
    scopes TEXT[] NOT NULL DEFAULT '{"telemetry:write","telemetry:read"}',
    expires_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_keys_tenant ON api_keys (tenant_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys (key_prefix);

-- 4. Registered Satellites — Customer-managed satellite fleet
CREATE TABLE IF NOT EXISTS registered_satellites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    satellite_id VARCHAR(50) NOT NULL,
    satellite_name VARCHAR(255) NOT NULL,
    norad_id INTEGER,
    orbit_type VARCHAR(30) NOT NULL DEFAULT 'LEO'
        CHECK (orbit_type IN ('LEO', 'MEO', 'GEO', 'SSO', 'HEO', 'POLAR', 'OTHER')),
    tle_line1 VARCHAR(80),
    tle_line2 VARCHAR(80),
    tle_epoch TIMESTAMP WITH TIME ZONE,
    launch_date TIMESTAMP WITH TIME ZONE,
    mission_type VARCHAR(50),
    mass_kg REAL,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE'
        CHECK (status IN ('ACTIVE', 'COMMISSIONING', 'DEGRADED', 'MAINTENANCE', 'DECOMMISSIONED')),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, satellite_id)
);

CREATE INDEX IF NOT EXISTS idx_reg_sats_tenant ON registered_satellites (tenant_id);
CREATE INDEX IF NOT EXISTS idx_reg_sats_satellite_id ON registered_satellites (satellite_id);

-- 5. Registered Ground Stations — Customer-managed ground network
CREATE TABLE IF NOT EXISTS registered_ground_stations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    station_id VARCHAR(50) NOT NULL,
    station_name VARCHAR(255) NOT NULL,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    altitude_m DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    antenna_type VARCHAR(50) DEFAULT 'parabolic',
    frequency_band VARCHAR(20) DEFAULT 'S-band',
    min_elevation_deg DOUBLE PRECISION NOT NULL DEFAULT 10.0,
    status VARCHAR(20) NOT NULL DEFAULT 'ONLINE'
        CHECK (status IN ('ONLINE', 'OFFLINE', 'MAINTENANCE', 'DECOMMISSIONED')),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, station_id)
);

CREATE INDEX IF NOT EXISTS idx_reg_gs_tenant ON registered_ground_stations (tenant_id);

-- 6. Add optional tenant_id to existing core tables for backward compatibility
-- Existing data will have NULL tenant_id (system/legacy data).
-- New data inserted via tenant-scoped APIs will always have a tenant_id.
ALTER TABLE telemetry ADD COLUMN IF NOT EXISTS tenant_id UUID;
ALTER TABLE events ADD COLUMN IF NOT EXISTS tenant_id UUID;
ALTER TABLE commands ADD COLUMN IF NOT EXISTS tenant_id UUID;
ALTER TABLE command_audit_trail ADD COLUMN IF NOT EXISTS tenant_id UUID;
ALTER TABLE timeline_events ADD COLUMN IF NOT EXISTS tenant_id UUID;
ALTER TABLE mission_health ADD COLUMN IF NOT EXISTS tenant_id UUID;

-- 7. Seed a default system tenant for backward compatibility
INSERT INTO tenants (id, tenant_name, tenant_slug, status, plan_tier, max_satellites, max_ground_stations, contact_email)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'OrbitLogs System',
    'system',
    'ACTIVE',
    'enterprise',
    9999,
    9999,
    'admin@orbitlogs.io'
) ON CONFLICT (tenant_slug) DO NOTHING;
