# OrbitLogs Product Governance & Engineering Guidelines 🏛️

OrbitLogs is a commercial spacecraft telemetry, digital twin, and mission control observability platform. This document outlines product technical governance, engineering standards, and contribution processes for commercial partners and engineering teams.

---

## 1. Product Technical Leadership

OrbitLogs product direction and architecture are led by the core engineering leadership:

* **Product Founder & Lead Architect**: Bikram Sarkar ([`@sarkarbikram90`](https://github.com/sarkarbikram90))

---

## 2. Engineering Principles & Standards

All contributions, feature developments, and module extensions must adhere to three core principles:

1. **Mission-Critical Reliability**: Ground segment telemetry systems must be resilient. Systems must guarantee zero message loss via manual Kafka offset commits and Rust memory safety.
2. **Sub-Second Low Latency**: Ingestion API endpoints, stream processors, and digital twin state updates must operate with sub-second end-to-end latency.
3. **Enterprise Security & Compliance**: All customer data must be isolated by `tenant_id`. Every command submission, RBAC modification, and emergency override must produce an immutable audit log for SOC 2 Type II compliance.

---

## 3. Contribution & Module Extension Guidelines

Commercial partners, customer integration engineers, and open-source contributors are welcome to extend OrbitLogs:

- **Bug Reports & Feature Requests**: Submit structured issues via [GitHub Issue Templates](.github/ISSUE_TEMPLATE/).
- **Pull Request Review**: All PRs require review from a core maintainer, pass automated CI checks (`cargo fmt`, `cargo clippy`, `cargo test`), and include test coverage.
- **Enterprise Integrations**: Custom ground station adapters, proprietary telemetry packet decoders, or dedicated LLM diagnostic providers should be authored as decoupled workspace microservices under `apps/` or shared crates under `crates/`.
