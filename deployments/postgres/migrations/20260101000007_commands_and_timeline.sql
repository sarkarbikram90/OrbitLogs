-- Create commands table
CREATE TABLE IF NOT EXISTS commands (
    id UUID PRIMARY KEY,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    command_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('CREATED', 'APPROVED', 'QUEUED', 'TRANSMITTED', 'RECEIVED', 'EXECUTED', 'VERIFIED', 'FAILED')),
    parameters JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by VARCHAR(50) NOT NULL,
    approved_by VARCHAR(50),
    scheduled_time TIMESTAMP WITH TIME ZONE,
    executed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create command audit trail table
CREATE TABLE IF NOT EXISTS command_audit_trail (
    id BIGSERIAL PRIMARY KEY,
    command_id UUID NOT NULL REFERENCES commands(id) ON DELETE CASCADE,
    transition_from VARCHAR(20),
    transition_to VARCHAR(20) NOT NULL,
    changed_by VARCHAR(50) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    trace_id VARCHAR(100),
    notes TEXT
);

-- Create unified timeline events table
CREATE TABLE IF NOT EXISTS timeline_events (
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    event_id UUID NOT NULL,
    source VARCHAR(50) NOT NULL CHECK (source IN ('TELEMETRY', 'COMMAND', 'ALERT', 'INCIDENT', 'GROUND_STATION', 'DIGITAL_TWIN')),
    satellite_id VARCHAR(50) REFERENCES satellites(id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('INFO', 'WARNING', 'CRITICAL')),
    message TEXT NOT NULL,
    details JSONB
);

-- Convert timeline_events to hypertable partitioned by time (1 day chunks)
SELECT create_hypertable('timeline_events', 'timestamp', chunk_time_interval => INTERVAL '1 day', if_not_exists => TRUE);

-- Create optimized composite index for timeline queries
CREATE INDEX IF NOT EXISTS idx_timeline_events_lookup ON timeline_events (satellite_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_timeline_events_source ON timeline_events (source, timestamp DESC);
