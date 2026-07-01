#!/bin/bash
set -e

# Default PG parameters
DB_HOST=${DATABASE_HOST:-"localhost"}
DB_PORT=${DATABASE_PORT:-5432}
DB_USER=${DATABASE_USER:-"postgres"}
DB_NAME=${DATABASE_NAME:-"orbitlogs"}
export PGPASSWORD=${DATABASE_PASSWORD:-"postgrespassword"}

echo "Seeding satellites metadata to database orbitlogs..."

psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" <<EOF
INSERT INTO satellites (id, name, model, launch_date, status) VALUES
('SAT-0001', 'OrbitLogs Explorer Alpha', 'CubeSat-3U', NOW() - INTERVAL '120 days', 'ACTIVE'),
('SAT-0002', 'OrbitLogs Weather Sentinel', 'Sentinel-V1', NOW() - INTERVAL '450 days', 'ACTIVE'),
('SAT-0003', 'OrbitLogs Broadband Router', 'Telecom-Mega', NOW() - INTERVAL '60 days', 'ACTIVE'),
('SAT-0004', 'OrbitLogs Geoscope Beta', 'RadarSensing-V2', NOW() - INTERVAL '900 days', 'MAINTENANCE'),
('SAT-0005', 'OrbitLogs Deep Space Link', 'DeepSpace-Pro', NOW() - INTERVAL '15 days', 'ACTIVE')
ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status;
EOF

echo "Database seeded successfully!"
