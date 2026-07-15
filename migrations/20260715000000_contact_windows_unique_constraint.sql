-- Migration to add unique constraint to contact_windows to avoid duplicate entries during predictor cycles
ALTER TABLE contact_windows ADD CONSTRAINT unique_contact_window UNIQUE (satellite_id, ground_station_id, start_time);
