-- Enable TimescaleDB extension if not already enabled
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Create satellites metadata table
CREATE TABLE IF NOT EXISTS satellites (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    model VARCHAR(50) NOT NULL,
    launch_date TIMESTAMP WITH TIME ZONE NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'DEGRADED', 'MAINTENANCE', 'DECOMMISSIONED')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create telemetry hypertable
CREATE TABLE IF NOT EXISTS telemetry (
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    battery_voltage DOUBLE PRECISION NOT NULL,
    battery_current DOUBLE PRECISION NOT NULL,
    solar_panel_output DOUBLE PRECISION NOT NULL,
    cpu_usage REAL NOT NULL,
    memory_usage REAL NOT NULL,
    temperature DOUBLE PRECISION NOT NULL,
    orientation_pitch REAL NOT NULL,
    orientation_roll REAL NOT NULL,
    orientation_yaw REAL NOT NULL,
    velocity DOUBLE PRECISION NOT NULL,
    altitude DOUBLE PRECISION NOT NULL,
    fuel_remaining REAL NOT NULL,
    communication_latency REAL NOT NULL,
    packet_loss REAL NOT NULL,
    signal_strength REAL NOT NULL,
    event_id UUID NOT NULL
);

-- Convert telemetry to hypertable partitioned by time (1 day chunks)
SELECT create_hypertable('telemetry', 'timestamp', chunk_time_interval => INTERVAL '1 day', if_not_exists => TRUE);

-- Create optimized composite index for time-series queries
CREATE INDEX IF NOT EXISTS idx_telemetry_satellite_time ON telemetry (satellite_id, timestamp DESC);

-- Create events / anomalies hypertable
CREATE TABLE IF NOT EXISTS events (
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL, -- e.g., 'ANOMALY_THERMAL_SPIKE', 'FUEL_LEAK', etc.
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('INFO', 'WARNING', 'CRITICAL')),
    message TEXT NOT NULL,
    details JSONB
);

-- Convert events to hypertable partitioned by time (1 day chunks)
SELECT create_hypertable('events', 'timestamp', chunk_time_interval => INTERVAL '1 day', if_not_exists => TRUE);

-- Index for events
CREATE INDEX IF NOT EXISTS idx_events_satellite_time ON events (satellite_id, timestamp DESC);

---------------------------------------------------------
-- PERFORMANCE: COMPRESSION POLICIES
---------------------------------------------------------

-- Enable compression on telemetry hypertable
ALTER TABLE telemetry SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'satellite_id',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Add compression policy for telemetry older than 7 days
SELECT add_compression_policy('telemetry', INTERVAL '7 days', if_not_exists => TRUE);

-- Enable compression on events hypertable
ALTER TABLE events SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'satellite_id',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Add compression policy for events older than 7 days
SELECT add_compression_policy('events', INTERVAL '7 days', if_not_exists => TRUE);

---------------------------------------------------------
-- RETENTION: DATA RETENTION POLICIES
---------------------------------------------------------

-- Add data retention policy to drop telemetry data older than 90 days
SELECT add_retention_policy('telemetry', INTERVAL '90 days', if_not_exists => TRUE);

-- Add data retention policy to drop events data older than 180 days
SELECT add_retention_policy('events', INTERVAL '180 days', if_not_exists => TRUE);
