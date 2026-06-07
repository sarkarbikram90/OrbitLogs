-- Enable compression on satellite_health hypertable
ALTER TABLE satellite_health SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'satellite_id',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Add compression policy for satellite_health older than 7 days
SELECT add_compression_policy('satellite_health', INTERVAL '7 days', if_not_exists => TRUE);

-- Add data retention policy to drop satellite_health older than 90 days
SELECT add_retention_policy('satellite_health', INTERVAL '90 days', if_not_exists => TRUE);

-- Enable compression on ground_station_health hypertable
ALTER TABLE ground_station_health SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'station_id',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Add compression policy for ground_station_health older than 7 days
SELECT add_compression_policy('ground_station_health', INTERVAL '7 days', if_not_exists => TRUE);

-- Add data retention policy to drop ground_station_health older than 90 days
SELECT add_retention_policy('ground_station_health', INTERVAL '90 days', if_not_exists => TRUE);
