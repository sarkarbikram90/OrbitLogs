# Security Policy 🛡️

The OrbitLogs maintainers take the security of our real-time spacecraft telemetry and mission control platform seriously. This document outlines our vulnerability reporting process, supported versions, and response SLAs.

---

## Supported Versions

Only the latest release tag and active development on the `main` branch receive security patches.

| Version Tag | Supported | Security Patch Policy |
| :--- | :---: | :--- |
| **>= 4.5.x** | ✅ | Actively supported with security patches and advisory releases. |
| **< 4.5.0** | ❌ | End of Life (EOL). Upgrade to the latest release. |

---

## Reporting a Vulnerability

If you discover a security vulnerability, please do **NOT** open a public issue.

### Primary Reporting Method
1. **GitHub Private Security Advisory**: Go to the [Security Tab](https://github.com/sarkarbikram90/OrbitLogs/security) of the OrbitLogs repository and click **Report a vulnerability**.
2. **Email Escalation**: If private reporting is unavailable, email the maintainers directly at **sarkarbikram90@gmail.com**.

### Information to Include
- Detailed description of the vulnerability and potential impact.
- Step-by-step instructions or Proof-of-Concept (PoC) code to reproduce the issue.
- Impacted workspace microservices or library crates.

---

## Response & Disclosure Timeline (SLA)

| Phase | Timeline SLA | Description |
| :--- | :--- | :--- |
| **Acknowledgment** | Within **24 Hours** | Maintainers confirm receipt of report. |
| **Triage & Assessment** | Within **72 Hours** | Maintainers validate vulnerability and assign severity rating (CVSS). |
| **Fix Development** | Within **14 Days** | Patch is developed, reviewed privately, and tested. |
| **Responsible Disclosure** | Within **90 Days** | Public security advisory (GHSA/CVE) and patch release published. |

---

## Security Commitments & Quality Practices

OrbitLogs enforces automated security checks across all pull requests:
- **Dependency Auditing**: `cargo-audit` and `cargo-deny` execute on every CI build to detect vulnerable or non-compliant dependencies.
- **Constant-Time Cryptography**: Authentication and security token checks utilize `subtle` constant-time operations to prevent timing side-channel attacks.
- **Non-Root Containers**: Service containers run under unprivileged dedicated user account `orbitlogs` (UID/GID `10001`).
