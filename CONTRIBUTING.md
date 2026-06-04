# Contributing to Open SpaceOps 🚀

Thank you for your interest in contributing to **Open SpaceOps**! This document provides guidelines, coding standards, and instructions for setting up your local environment to contribute to our real-time satellite telemetry platform.

---

## 🛠️ Local Development Setup

To compile, run, and test Open SpaceOps locally:

### 1. Prerequisites
- **Rust Toolchain**: Install the stable toolchain via [rustup](https://rustup.rs/).
- **Docker & Docker Compose**: Needed to run external infrastructure (TimescaleDB, Kafka, Jaeger, Prometheus, Grafana).
- **PostgreSQL Client (psql)**: Optional, for running raw SQL triage queries.

### 2. Starting Infrastructure
Spin up the backing databases and message brokers:
```bash
docker compose -f deployments/docker-compose/docker-compose.yml up -d timescaledb kafka zookeeper jaeger otel-collector prometheus grafana
```

Initialize the database schema:
```bash
PGPASSWORD=postgrespassword psql -h localhost -U postgres -d spaceops -f deployments/postgres/migrations/01_init.sql
```

### 3. Running Services Locally
Copy the environment template and configure defaults:
```bash
cp .env.example .env
```

To run the Telemetry Collector:
```bash
DATABASE_URL=postgres://postgres:postgrespassword@localhost:5432/spaceops KAFKA_BROKERS=localhost:9092 cargo run --bin telemetry-collector
```

To run the Telemetry Simulator:
```bash
COLLECTOR_URL=http://localhost:8080/telemetry cargo run --bin telemetry-simulator
```

---

## 📏 Coding Standards

To maintain production-grade quality, we enforce strict linting and formatting rules:

### 1. Code Formatting
All code must be formatted using `rustfmt`:
```bash
cargo fmt --all --check
```

### 2. Linting (Clippy)
All code must be clippy warning-free. We compile with denied warnings in CI:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### 3. Unit & Integration Testing
All changes must be covered by appropriate unit or integration tests:
```bash
cargo test --all-targets --all-features
```

---

## 🔀 Pull Request Guidelines

1. **Decouple Logic**: Keep service logic decoupled. Shared models must reside in the `spaceops-models` crate.
2. **Handle Errors Safely**: Avoid using `.unwrap()` or `.expect()` in runtime ingestion paths. Return descriptive `Result` wrappers or logs.
3. **No Unused Imports / Code**: Clean up any unused imports or dead code before pushing.
4. **Non-Root Verification**: If you modify the container setup, verify that containers continue to start as non-root (UID 10001).
5. **No Secrets in Code**: Never commit passwords or credentials. Use `.env` or system environment variables.
