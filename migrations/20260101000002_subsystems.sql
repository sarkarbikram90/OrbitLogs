-- Add subsystem_id and mission_id columns
ALTER TABLE telemetry ADD COLUMN IF NOT EXISTS subsystem_id VARCHAR(50) NOT NULL DEFAULT 'unified';
ALTER TABLE telemetry ADD COLUMN IF NOT EXISTS mission_id VARCHAR(50) NOT NULL DEFAULT 'unknown';

-- Add payload columns
ALTER TABLE telemetry ADD COLUMN IF NOT EXISTS payload_power_draw DOUBLE PRECISION;
ALTER TABLE telemetry ADD COLUMN IF NOT EXISTS payload_active BOOLEAN;

-- Make all metric columns nullable to support sparse subsystem-specific writes
ALTER TABLE telemetry ALTER COLUMN battery_voltage DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN battery_current DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN solar_panel_output DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN cpu_usage DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN memory_usage DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN temperature DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN orientation_pitch DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN orientation_roll DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN orientation_yaw DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN velocity DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN altitude DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN fuel_remaining DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN communication_latency DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN packet_loss DROP NOT NULL;
ALTER TABLE telemetry ALTER COLUMN signal_strength DROP NOT NULL;

-- Create composite index for faster subsystem-level and mission-level filtering
CREATE INDEX IF NOT EXISTS idx_telemetry_subsystem_mission ON telemetry (satellite_id, subsystem_id, mission_id, timestamp DESC);
