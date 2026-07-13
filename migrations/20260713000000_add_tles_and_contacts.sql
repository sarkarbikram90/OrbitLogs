-- Migration to add Two-Line Elements (TLEs) and Contact Windows tables for Starlink Operations
CREATE TABLE IF NOT EXISTS satellite_tles (
    satellite_id VARCHAR(50) NOT NULL,
    epoch TIMESTAMPTZ NOT NULL,
    line1 CHAR(69) NOT NULL,
    line2 CHAR(69) NOT NULL,
    PRIMARY KEY (satellite_id, epoch)
);

CREATE TABLE IF NOT EXISTS contact_windows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    satellite_id VARCHAR(50) NOT NULL,
    ground_station_id VARCHAR(50) NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    max_elevation REAL NOT NULL
);
