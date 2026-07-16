# OrbitLogs Project Roadmap 🗺️

OrbitLogs is a real-time spacecraft telemetry and mission control observability platform. This document outlines our future development milestones, categorized by timeline.

## 🟢 Completed Milestones (v1.0.0 - v4.5.0)
*   **Core Subsystems & Health Engine**: 7 onboard subsystems simulated with green/yellow/red rules-based evaluation.
*   **Astrodynamics & Ground Stations**: Integrated SGP4 orbital propagation, coordinate rotations (ECI/ECEF/ENU), and weather link budgets.
*   **Operations & Planning**: HTN-based command planner, Extended Kalman Filter (EKF) orbit determination, and satellite conjunction assessment.
*   **Reliability & Security**: TimescaleDB compression policies, Kafka manual offset commits, and non-root Docker builds.
*   **OpenTelemetry Compliance**: W3C context propagation over Kafka headers, OTLP exporters, and Prometheus scraper integration.

## 🟡 Short-Term Goals (Next 1 - 3 Months)
*   **CI License Header Auditing**: Implement an automated pull request check to ensure all source code files have proper Apache 2.0 license headers.
*   **API Error Standardization**: Replace raw string error payloads with a structured JSON schema containing application-specific error codes.
*   **Zero-Copy Parser Refactoring**: Enhance telemetry ingestion performance by using lifetime references (`TelemetryView<'buf>`) rather than allocating owned structures.

## 🔵 Medium-Term Goals (Next 3 - 6 Months)
*   **WASM Client-Side Propagation**: Compile `orbitlogs-astrodynamics` to WebAssembly so that the web client can propagate 10,000+ orbits directly in the browser, offloading the backend.
*   **Ingress Deduplication Buffer**: Establish a window-based deduplication queue in the ingestion service to filter duplicate packets received by multiple overlapping ground stations.
*   **Unified Helm Chart**: Author a production-ready Helm chart to deploy all 17 microservices on Kubernetes, supporting custom overrides.

## 🔴 Long-Term Goals (6 Months+)
*   **Multi-Region TimescaleDB Replication**: Implement geo-distributed database replication to ensure minimal write latencies for ground stations distributed across polar regions.
*   **KubeEdge Integration**: Adapt the Telemetry Collector to run as a KubeEdge edge app, optimizing processing on low-bandwidth satellite links.
