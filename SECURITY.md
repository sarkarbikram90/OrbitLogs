# Enterprise Security & Compliance Policy 🛡️

At OrbitLogs, security is fundamental to our mission control observability platform. This document outlines our vulnerability disclosure SLAs, enterprise security controls, and SOC 2 Type II compliance framework.

---

## Supported Versions

Only the active release version line receives security patches and hotfixes.

| Version Tag | Supported | Security Policy |
| :--- | :---: | :--- |
| **>= 4.5.x** | ✅ | Actively supported with security hotfixes and vulnerability patches. |
| **< 4.5.0** | ❌ | End of Life (EOL). Upgrade to current release. |

---

## Enterprise Security Controls

- **Tenant Isolation**: Multi-tenant data pipelines enforce `tenant_id` context isolation across API gateways, Kafka streams, and TimescaleDB hypertables.
- **Immutable Audit Logging**: RBAC authorization state changes, satellite command submissions, and override executions are recorded in tamper-evident audit hypertables with OpenTelemetry trace identifiers.
- **Constant-Time Cryptography**: Token validation and WebSocket upgrade authentication utilize `subtle` constant-time comparisons to prevent timing side-channel profiling attacks.
- **Unprivileged Container Hardening**: Production microservice containers execute under a dedicated unprivileged user `orbitlogs` (UID/GID `10001`).

---

## Reporting a Security Vulnerability

If you discover a security vulnerability or potential threat in OrbitLogs, please report it privately:

### Contact Information
- **Private Reporting**: Submit via [GitHub Private Security Advisory](https://github.com/sarkarbikram90/OrbitLogs/security).
- **Security Escalation Email**: `security@orbitlogs.io` (or `sarkarbikram90@gmail.com`).

### Vulnerability Response SLA

| Phase | SLA | Action |
| :--- | :--- | :--- |
| **Acknowledgment** | Within **24 Hours** | Maintainers confirm receipt of vulnerability report. |
| **Triage & Assessment** | Within **48 Hours** | Security team assesses severity (CVSS v3) and affected microservices. |
| **Patch Resolution** | Within **7 Days** | Private fix developed, tested, and validated. |
| **Public Advisory / Customer Release** | Within **30 Days** | Customer patch released and advisory issued. |
