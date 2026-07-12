# Contributing to OrbitLogs 🚀

Thank you for your interest in contributing to **OrbitLogs**! This document provides guidelines and workflows for contributing to our monorepo.

To align with Staff-level engineering standards, we enforce strict code formatting, linting, testing, and security checks on all contributions.

---

## 1. Repository Structure

OrbitLogs is organized as a Rust monorepo:

*   **`apps/`**: Standalone binary microservices (e.g., `telemetry-collector`, `mission-timeline`, `digital-twin-engine`).
*   **`crates/`**: Reusable shared libraries utilized across multiple applications (e.g., `orbitlogs-models`, `orbitlogs-packets`).
*   **`deployments/`**: Kubernetes manifests, Docker Compose setups, and configurations for Prometheus, Grafana, Alertmanager, and OpenTelemetry.
*   **`migrations/`**: TimescaleDB / Postgres SQL migration scripts (consolidated source of truth).

---

## 2. Setting Up Your Development Environment

### Prerequisites

Ensure you have the following installed locally:
*   [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
*   [Docker](https://www.docker.com/products/docker-desktop) & Docker Compose
*   [PostgreSQL Client](https://www.postgresql.org/download/) (optional, for running migration checks)
*   [Node.js](https://nodejs.org/) (optional, for frontend changes in `mission-control-ui`)

### Clone and Bootstrap

1.  Clone the repository:
    ```bash
    git clone https://github.com/sarkarbikram90/OrbitLogs.git
    cd OrbitLogs
    ```
2.  Set up local development configuration:
    ```bash
    cp .env.example .env
    ```
3.  Initialize the TimescaleDB database:
    ```bash
    docker compose -f deployments/docker-compose/docker-compose.yml up timescaledb -d
    ```

---

## 3. Pre-Commit Checklist & Quality Gates

Before submitting a Pull Request, please run the following checks locally to ensure a green CI pipeline:

### A. Formatting & Linting
We enforce standard formatting and deny compiler/Clippy warnings:
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### B. Unused Dependency Check
We use `cargo-machete` to identify unused crates in our manifests:
```bash
cargo install cargo-machete
cargo machete
```

### C. Testing
Verify that all unit and integration tests compile and pass successfully:
```bash
cargo test --all-targets --all-features
```

### D. Coverage (Optional)
Generate a coverage report locally using `cargo-tarpaulin`:
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --all-features
```

---

## 4. Development Workflow

### Branching Strategy
*   Always create a feature branch off of the `main` branch.
*   Branch naming convention: `username/feature-name` or `issue-number/short-description`.

### Commit Message Guidelines
We encourage semantic commit messages to keep the Git history clean:
*   `feat`: A new feature (e.g., `feat(ssa): add orbital debris tracking`)
*   `fix`: A bug fix (e.g., `fix(timeline): resolve CORS preflight header typo`)
*   `refactor`: Code changes that do not fix a bug or add a feature (e.g., `refactor(db): break down digital twin main.rs`)
*   `test`: Adding or modifying tests (e.g., `test(auth): add rate limiter unit tests`)
*   `docs`: Documentation changes (e.g., `docs: update deployment guidelines`)

### Submitting a Pull Request
1.  Push your branch to your fork or origin:
    ```bash
    git push origin username/feature-name
    ```
2.  Open a Pull Request against the `main` branch.
3.  Fill out the **Pull Request Template** completely.
4.  Ensure all GitHub Actions checks pass successfully.
5.  Wait for review from core maintainers.
