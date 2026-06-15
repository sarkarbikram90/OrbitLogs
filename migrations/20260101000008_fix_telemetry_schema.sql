-- Alter telemetry table to make columns nullable to match the Rust TelemetryRecord Option fields
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

-- Add payload metrics columns if they don't exist
ALTER TABLE telemetry ADD COLUMN IF NOT EXISTS payload_power_draw DOUBLE PRECISION;
ALTER TABLE telemetry ADD COLUMN IF NOT EXISTS payload_active BOOLEAN;
