-- Create ground_station_health hypertable for historical health status tracking
CREATE TABLE IF NOT EXISTS ground_station_health (
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    station_id VARCHAR(50) NOT NULL,
    station_name VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('ONLINE', 'DEGRADED', 'OUTAGE')),
    antenna_health REAL NOT NULL,
    weather_state VARCHAR(50) NOT NULL,
    congestion REAL NOT NULL,
    latency REAL NOT NULL,
    bandwidth REAL NOT NULL,
    explanation TEXT NOT NULL
);

-- Convert to TimescaleDB hypertable partitioned by time (1 day chunks)
SELECT create_hypertable('ground_station_health', 'timestamp', chunk_time_interval => INTERVAL '1 day', if_not_exists => TRUE);

-- Create optimized index
CREATE INDEX IF NOT EXISTS idx_ground_station_health_lookup ON ground_station_health (station_id, timestamp DESC);
