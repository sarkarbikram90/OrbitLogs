# Contributing to OrbitLogs 🚀

Thank you for your interest in contributing to **OrbitLogs**! This document provides guidelines and workflows for contributing to our cloud-native spacecraft telemetry workspace.

To align with Linux Foundation and CNCF Sandbox standards, we enforce Developer Certificate of Origin (DCO) commit sign-offs, automated code formatting, linting, testing, and security checks on all contributions.

---

## 1. Developer Certificate of Origin (DCO)

To ensure vendor neutrality and clean copyright provenance across all contributions, OrbitLogs requires all committers to certify their contributions under the **Developer Certificate of Origin (DCO)**.

### How to Sign Off Commits
Every Git commit **must** contain a `Signed-off-by:` header line matching the committer's Git email address.

```bash
git commit -s -m "feat(telemetry): add CRC-16 checksum validation layer"
```

If you forget to sign off a commit, you can amend your last commit before pushing:

```bash
git commit --amend -s
```

Pull requests containing unsigned commits will be automatically flagged and blocked by our DCO CI check ([.github/workflows/dco.yml](.github/workflows/dco.yml)).

---

## 2. Workspace Structure

OrbitLogs is organized as a Rust monorepo with **26 workspace members** (17 applications + 7 shared library crates + 2 utility crates):

*   **`apps/`**: Standalone binary microservices:
    *   `telemetry-simulator`, `telemetry-collector`, `mission-health-engine`, `ground-station-simulator`, `mission-replay-engine`, `mission-command-service`, `digital-twin-engine`, `mission-timeline`, `ai-incident-commander`, `constellation-simulator`, `orbit-ops-service`, `space-otel-collector`, `contact-window-predictor`, `constellation-scheduler`, `orbit-determination-service`, `autonomous-mission-planning`, `mission-control-ui` (React/TypeScript).
*   **`crates/`**: Reusable shared libraries:
    *   `orbitlogs-models`, `orbitlogs-packets`, `space-schema-registry`, `fleet-health`, `orbitlogs-astrodynamics`, `orbitlogs-propulsion`, `orbitlogs-test-utils`.
*   **`deployments/`**: Kubernetes manifests, Docker Compose setups, and configurations for Prometheus, Grafana, Alertmanager, and OpenTelemetry.
*   **`migrations/`**: TimescaleDB / Postgres SQL migration scripts.

---

## 3. Setting Up Your Development Environment

### Prerequisites
*   [Rust](https://www.rust-lang.org/tools/install) (stable toolchain, v1.82+)
*   [Docker](https://www.docker.com/products/docker-desktop) & Docker Compose
*   [Node.js](https://nodejs.org/) (v20+, for `mission-control-ui`)

### Bootstrap Local Infrastructure
```bash
git clone https://github.com/sarkarbikram90/OrbitLogs.git
cd OrbitLogs

cp .env.example .env
docker compose -f deployments/docker-compose/docker-compose.yml up timescaledb kafka -d
```

---

## 4. Pre-Commit Quality Gates

Before submitting a Pull Request, run the following checks locally:

### A. Formatting & Linting
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### B. Unit & Integration Testing
```bash
cargo test --all-targets --all-features
```

### C. Unused Dependency Check
```bash
cargo machete
```

### D. Security Auditing
```bash
cargo audit
cargo deny check
```

---

## 5. Development Workflow & Pull Requests

1. **Branching**: Create feature branches off `main` using `username/feature-name` or `issue-number/description`.
2. **Conventional Commits**: Use conventional commit titles:
   - `feat(service)`: New features
   - `fix(service)`: Bug fixes
   - `refactor(crate)`: Refactorings without behavior changes
   - `docs`: Documentation updates
3. **Pull Request Submission**:
   - Open a PR against `main`.
   - Complete the [Pull Request Template](.github/pull_request_template.md).
   - Ensure all automated GitHub Action checks pass.

---

## 6. Governance & Code of Conduct

*   **Governance Model**: See [GOVERNANCE.md](GOVERNANCE.md) for details on maintainer roles, lazy consensus, and decision making.
*   **Maintainers Roster**: See [MAINTAINERS.md](MAINTAINERS.md).
*   **Code of Conduct**: All participants must abide by the [CNCF Code of Conduct](CODE_OF_CONDUCT.md).
*   **Security Disclosures**: Report security issues according to [SECURITY.md](SECURITY.md).
