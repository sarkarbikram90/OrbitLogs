# OrbitLogs Project Governance 🏛️

OrbitLogs is an open-source, vendor-neutral project focused on real-time spacecraft telemetry, flight observability, and mission control engineering. This document details the project's open governance structure, decision-making processes, maintainership rules, and Technical Steering Committee (TSC) policies, adhering to **Cloud Native Computing Foundation (CNCF)** Sandbox guidelines.

---

## 1. Principles of Open Governance

OrbitLogs operates under four core principles:

1. **Vendor Neutrality**: No single corporate entity holds a permanent majority or exclusive rights over project direction or governance.
2. **Meritocracy**: Influence and leadership roles are earned through active, sustained technical and community contributions.
3. **Openness**: All project discussions, design proposals (RFCs), issue triage, and technical decisions take place publicly on GitHub and community forums.
4. **Inclusivity**: Every contributor is welcomed and evaluated fairly under our adopted [CNCF Code of Conduct](CODE_OF_CONDUCT.md).

---

## 2. Leadership Roles & Progression

### A. Contributors
Contributors are community members who submit pull requests, report issues, improve documentation, or participate in design discussions.
* **Privileges**: Can open issues and pull requests, participate in design reviews.
* **Requirements**: Must sign off all commits using the Developer Certificate of Origin (`git commit -s`).

### B. Maintainers
Maintainers are contributors who have demonstrated sustained technical excellence, code quality, and active community participation.
* **Privileges**: Maintainers have write access to the repository, can approve and merge pull requests, manage issue triage, and participate in security reviews.
* **Onboarding Process**: Any active maintainer may nominate a contributor who has submitted significant pull requests (typically 5+ substantial contributions) and actively participated in code reviews over a 3-month period. Approval requires a 2/3 majority vote among active maintainers.
* **Offboarding & Emeritus Status**: A maintainer who is inactive for 6 months without notice will be transitioned to "Maintainer Emeritus" status, relinquishing write access. Emeritus maintainers can be reinstated upon active re-engagement.

### C. Technical Steering Committee (TSC)
The Technical Steering Committee governs overall architecture, security policies, release schedules, and vendor neutrality compliance.
* **Composition**: The TSC consists of up to 5 elected maintainers representing diverse organizations.
* **Voting Thresholds**:
  - **Lazy Consensus**: Standard day-to-day decisions and minor pull requests are approved unless an explicit objection is raised within 72 hours.
  - **Majority Vote**: Architectural shifts, dependency additions, and governance amendments require a simple majority (>50%) vote.
  - **Supermajority Vote (2/3)**: Maintainer onboarding/offboarding, security policy modifications, and CNCF transition milestones require a 2/3 supermajority vote.

---

## 3. Current Maintainer Roster

The current active project maintainer list is maintained in **[MAINTAINERS.md](MAINTAINERS.md)**.

* **Project Lead**: Bikram Sarkar (`@sarkarbikram90`)

---

## 4. Conflict Resolution & Escalations

Technical disputes that cannot be resolved through lazy consensus will be brought to the TSC for discussion and resolution. If a vote is required, each TSC member receives one vote. Ties are resolved by the Project Lead.
