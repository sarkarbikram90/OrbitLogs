-- Create orbit_positions table
CREATE TABLE IF NOT EXISTS orbit_positions (
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    altitude DOUBLE PRECISION NOT NULL,
    velocity DOUBLE PRECISION NOT NULL,
    orbital_period DOUBLE PRECISION NOT NULL,
    inclination DOUBLE PRECISION NOT NULL,
    eccentricity DOUBLE PRECISION NOT NULL
);
SELECT create_hypertable('orbit_positions', 'timestamp', chunk_time_interval => INTERVAL '1 day', if_not_exists => TRUE);
CREATE INDEX IF NOT EXISTS idx_orbit_positions_satellite_time ON orbit_positions (satellite_id, timestamp DESC);

-- Create tle_records table
CREATE TABLE IF NOT EXISTS tle_records (
    id UUID PRIMARY KEY,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    epoch TIMESTAMP WITH TIME ZONE NOT NULL,
    line1 VARCHAR(100) NOT NULL,
    line2 VARCHAR(100) NOT NULL,
    inclination DOUBLE PRECISION NOT NULL,
    raan DOUBLE PRECISION NOT NULL,
    eccentricity DOUBLE PRECISION NOT NULL,
    arg_of_perigee DOUBLE PRECISION NOT NULL,
    mean_anomaly DOUBLE PRECISION NOT NULL,
    mean_motion DOUBLE PRECISION NOT NULL,
    semi_major_axis DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create pass_predictions table
CREATE TABLE IF NOT EXISTS pass_predictions (
    id UUID PRIMARY KEY,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    station_id VARCHAR(50) NOT NULL,
    aos TIMESTAMP WITH TIME ZONE NOT NULL,
    los TIMESTAMP WITH TIME ZONE NOT NULL,
    duration DOUBLE PRECISION NOT NULL,
    max_elevation DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create contact_schedules table
CREATE TABLE IF NOT EXISTS contact_schedules (
    id UUID PRIMARY KEY,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    station_id VARCHAR(50) NOT NULL,
    window_type VARCHAR(50) NOT NULL, -- 'TELEMETRY', 'COMMAND', 'DOWNLINK', 'MAINTENANCE'
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    end_time TIMESTAMP WITH TIME ZONE NOT NULL,
    priority VARCHAR(20) NOT NULL CHECK (priority IN ('ROUTINE', 'CRITICAL', 'EMERGENCY')),
    status VARCHAR(20) NOT NULL CHECK (status IN ('RESERVED', 'CONFLICT', 'EXECUTED')),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create ssa_objects table
CREATE TABLE IF NOT EXISTS ssa_objects (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    object_type VARCHAR(50) NOT NULL, -- 'DEBRIS', 'SATELLITE'
    inclination DOUBLE PRECISION NOT NULL,
    altitude DOUBLE PRECISION NOT NULL,
    velocity DOUBLE PRECISION NOT NULL,
    size_meters DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create conjunction_events table
CREATE TABLE IF NOT EXISTS conjunction_events (
    id UUID PRIMARY KEY,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    object_id VARCHAR(50) NOT NULL,
    object_name VARCHAR(100) NOT NULL,
    object_type VARCHAR(50) NOT NULL,
    probability DOUBLE PRECISION NOT NULL,
    distance_meters DOUBLE PRECISION NOT NULL,
    conjunction_time TIMESTAMP WITH TIME ZONE NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('LOW', 'MEDIUM', 'HIGH', 'CRITICAL')),
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'RESOLVED')),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create mission_plans table
CREATE TABLE IF NOT EXISTS mission_plans (
    id UUID PRIMARY KEY,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    plan_type VARCHAR(50) NOT NULL, -- 'Imaging', 'Communications', 'Calibration', 'Orbit Maneuver', 'Payload Test', 'Software Update'
    description TEXT NOT NULL,
    objectives TEXT NOT NULL,
    required_contacts JSONB NOT NULL DEFAULT '[]'::jsonb,
    required_commands JSONB NOT NULL DEFAULT '[]'::jsonb,
    required_payloads JSONB NOT NULL DEFAULT '[]'::jsonb,
    scheduled_time TIMESTAMP WITH TIME ZONE NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('DRAFT', 'REVIEW', 'APPROVED', 'SCHEDULED', 'EXECUTING', 'COMPLETED', 'CANCELLED')),
    created_by VARCHAR(50) NOT NULL,
    approved_by VARCHAR(50),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create orbit_maneuvers table
CREATE TABLE IF NOT EXISTS orbit_maneuvers (
    id UUID PRIMARY KEY,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    maneuver_type VARCHAR(50) NOT NULL, -- 'STATION_KEEPING', 'RAAN_ALIGNMENT', 'COLLISION_AVOIDANCE'
    delta_v DOUBLE PRECISION NOT NULL, -- m/s
    fuel_consumption_kg DOUBLE PRECISION NOT NULL,
    scheduled_time TIMESTAMP WITH TIME ZONE NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('RECOMMENDED', 'APPROVED', 'EXECUTING', 'COMPLETED', 'FAILED')),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create command_verifications table
CREATE TABLE IF NOT EXISTS command_verifications (
    id UUID PRIMARY KEY,
    command_id UUID NOT NULL,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    command_type VARCHAR(50) NOT NULL,
    expected_telemetry_field VARCHAR(100) NOT NULL,
    expected_telemetry_value VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('CREATED', 'APPROVED', 'QUEUED', 'UPLINKED', 'EXECUTING', 'CONFIRMED', 'FAILED', 'TIMEOUT')),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
