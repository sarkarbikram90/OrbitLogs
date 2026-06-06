-- Create satellite_health hypertable for historical health analysis
CREATE TABLE IF NOT EXISTS satellite_health (
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    mission_id VARCHAR(50) NOT NULL,
    overall_state VARCHAR(20) NOT NULL CHECK (overall_state IN ('GREEN', 'YELLOW', 'RED')),
    overall_score INT NOT NULL,
    power_state VARCHAR(20) NOT NULL CHECK (power_state IN ('GREEN', 'YELLOW', 'RED')),
    thermal_state VARCHAR(20) NOT NULL CHECK (thermal_state IN ('GREEN', 'YELLOW', 'RED')),
    comm_state VARCHAR(20) NOT NULL CHECK (comm_state IN ('GREEN', 'YELLOW', 'RED')),
    propulsion_state VARCHAR(20) NOT NULL CHECK (propulsion_state IN ('GREEN', 'YELLOW', 'RED')),
    adcs_state VARCHAR(20) NOT NULL CHECK (adcs_state IN ('GREEN', 'YELLOW', 'RED')),
    payload_state VARCHAR(20) NOT NULL CHECK (payload_state IN ('GREEN', 'YELLOW', 'RED')),
    fc_state VARCHAR(20) NOT NULL CHECK (fc_state IN ('GREEN', 'YELLOW', 'RED')),
    explanation TEXT NOT NULL
);

-- Convert to TimescaleDB hypertable partitioned by time (1 day chunks)
SELECT create_hypertable('satellite_health', 'timestamp', chunk_time_interval => INTERVAL '1 day', if_not_exists => TRUE);

-- Create optimized composite index for time-series queries
CREATE INDEX IF NOT EXISTS idx_satellite_health_lookup ON satellite_health (satellite_id, timestamp DESC);
