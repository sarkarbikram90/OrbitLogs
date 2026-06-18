-- Create a continuous aggregate for hourly telemetry summaries
CREATE MATERIALIZED VIEW IF NOT EXISTS telemetry_hourly
WITH (timescaledb.continuous) AS
SELECT time_bucket('1 hour', timestamp) AS bucket,
       satellite_id,
       AVG(battery_voltage) AS avg_battery_voltage,
       AVG(solar_panel_output) AS avg_solar_panel_output,
       AVG(cpu_usage) AS avg_cpu_usage,
       AVG(temperature) AS avg_temperature,
       AVG(altitude) AS avg_altitude
FROM telemetry
GROUP BY bucket, satellite_id;

-- Add a continuous aggregate refresh policy
SELECT add_continuous_aggregate_policy('telemetry_hourly',
  start_offset => INTERVAL '3 hours',
  end_offset => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour',
  if_not_exists => TRUE);
