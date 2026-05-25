#!/bin/bash
set -e

echo "=== Open SpaceOps Local Bootstrap ==="

# 1. Check prerequisites
echo "Checking prerequisites..."
command -v cargo >/dev/null 2>&1 || { echo >&2 "Rust (cargo) is required but not installed. Aborting."; exit 1; }
command -v docker >/dev/null 2>&1 || { echo >&2 "Docker is required but not installed. Aborting."; exit 1; }

# 2. Build local Rust packages to make sure they compile
echo "Compiling Rust workspace..."
cargo build

# 3. Spin up Docker containers
echo "Starting local docker-compose environment..."
docker compose -f deployments/docker-compose/docker-compose.yml up -d --build

# 4. Wait for database health
echo "Waiting for TimescaleDB to become healthy..."
until docker exec spaceops-timescaledb pg_isready -U postgres -d spaceops >/dev/null 2>&1; do
  echo -n "."
  sleep 1
done
echo ""
echo "TimescaleDB is online!"

# 5. Run database seed
echo "Seeding initial satellite metadata..."
./scripts/seed_db.sh

echo "=== System Bootstrapped successfully! ==="
echo "Access points:"
echo "- API Collector: http://localhost:8080/telemetry"
echo "- Prometheus:    http://localhost:9090"
echo "- Jaeger UI:     http://localhost:16686"
echo "- Grafana:       http://localhost:3000 (User: admin, Pass: admin)"
