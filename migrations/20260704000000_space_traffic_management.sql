-- Create debris_catalog table
CREATE TABLE IF NOT EXISTS debris_catalog (
    norad_id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    object_type VARCHAR(50) NOT NULL DEFAULT 'DEBRIS', -- 'DEBRIS', 'ROCKET_BODY', 'ACTIVE_SATELLITE'
    inclination DOUBLE PRECISION NOT NULL DEFAULT 51.6,
    apogee_km DOUBLE PRECISION NOT NULL DEFAULT 550.0,
    perigee_km DOUBLE PRECISION NOT NULL DEFAULT 500.0,
    rcs_size VARCHAR(20) NOT NULL DEFAULT 'MEDIUM', -- 'SMALL', 'MEDIUM', 'LARGE'
    tle_line1 VARCHAR(100),
    tle_line2 VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create conjunction_alerts table
CREATE TABLE IF NOT EXISTS conjunction_alerts (
    conjunction_id UUID PRIMARY KEY,
    primary_satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    secondary_object_id VARCHAR(50) NOT NULL,
    secondary_object_type VARCHAR(50) NOT NULL DEFAULT 'Debris',
    tca TIMESTAMP WITH TIME ZONE NOT NULL,
    miss_distance_km DOUBLE PRECISION NOT NULL,
    radial_miss_m DOUBLE PRECISION NOT NULL,
    intrack_miss_m DOUBLE PRECISION NOT NULL,
    crosstrack_miss_m DOUBLE PRECISION NOT NULL,
    collision_probability DOUBLE PRECISION NOT NULL,
    relative_velocity_kms DOUBLE PRECISION NOT NULL,
    risk_tier VARCHAR(20) NOT NULL CHECK (risk_tier IN ('Low', 'Medium', 'High', 'Critical')),
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'MANEUVER_SCHEDULED', 'RESOLVED', 'IGNORED')),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_conjunction_alerts_tca ON conjunction_alerts (tca DESC);
CREATE INDEX IF NOT EXISTS idx_conjunction_alerts_sat ON conjunction_alerts (primary_satellite_id, risk_tier);

-- Create conjunction_maneuver_recommendations table
CREATE TABLE IF NOT EXISTS conjunction_maneuver_recommendations (
    recommendation_id UUID PRIMARY KEY,
    conjunction_id UUID NOT NULL REFERENCES conjunction_alerts(conjunction_id) ON DELETE CASCADE,
    satellite_id VARCHAR(50) NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    burn_execution_window_start TIMESTAMP WITH TIME ZONE NOT NULL,
    delta_v_radial DOUBLE PRECISION NOT NULL,
    delta_v_intrack DOUBLE PRECISION NOT NULL,
    delta_v_crosstrack DOUBLE PRECISION NOT NULL,
    post_burn_miss_distance_km DOUBLE PRECISION NOT NULL,
    post_burn_pc DOUBLE PRECISION NOT NULL,
    fuel_cost_kg DOUBLE PRECISION NOT NULL,
    payload_downtime_minutes INT NOT NULL DEFAULT 15,
    recommended_action TEXT NOT NULL,
    rationale TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'RECOMMENDED' CHECK (status IN ('RECOMMENDED', 'APPROVED', 'EXECUTED', 'REJECTED')),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
