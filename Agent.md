# Open Orbitlogs 2.0 – OpenTelemetry-Native Mission Operations Platform

You are a Principal Software Engineer at SpaceX responsible for designing mission operations platforms that can operate and monitor thousands of spacecraft simultaneously.

Your task is to evolve the existing Open Orbitlogs codebase into a production-grade, open-source Orbitlogs platform that combines:

* Mission Operations
* Satellite Fleet Management
* OpenTelemetry
* Observability
* Distributed Systems
* AI-Assisted Incident Response
* Ground Station Monitoring
* Digital Twins
* Constellation Operations

The existing codebase already contains:

* Rust microservices
* Kafka event streaming
* TimescaleDB
* OpenTelemetry
* Jaeger
* Prometheus
* Grafana
* Mission Replay Engine
* Telemetry Simulator
* Ground Station Simulator
* Three.js Mission Control Console

DO NOT replace existing architecture.

Build on top of the current architecture.

---

# Product Vision

Traditional observability platforms understand:

* Applications
* Containers
* Kubernetes
* Databases

Open SpaceOps should understand:

* Satellites
* Ground Stations
* Telemetry
* Commands
* Payloads
* Constellations
* Mission Events
* Orbital Assets

Open Orbitlogs becomes:

Datadog + Grafana + ServiceNow + OpenTelemetry + Mission Control for Space Systems.

---

# Core Design Principles

1. OpenTelemetry Native
2. Rust First
3. Event Driven
4. Cloud Native
5. Mission Operations Focused
6. Fleet Scale Architecture
7. AI Assisted Operations
8. Extensible Plugin Framework

---

# Phase 8: Space OpenTelemetry Collector

Create:

crates/space-otel-collector

Purpose:

Convert spacecraft telemetry into standard OpenTelemetry signals.

Input Sources:

* Satellite Telemetry
* Ground Station Events
* Command Events
* Mission Events
* Digital Twin State Changes

Output:

Metrics:

* battery_voltage
* battery_current
* solar_output
* cpu_usage
* memory_usage
* fuel_remaining
* signal_strength
* packet_loss
* link_latency

Logs:

* subsystem_failures
* payload_events
* thermal_events
* communication_events

Traces:

* command_lifecycle
* payload_operations
* mission_sequences

Requirements:

* Async Rust
* Tokio
* OTLP Exporters
* Prometheus Exporters
* Kafka Integration
* Zero-copy processing where possible
* High throughput design

---

# Phase 9: Mission Command Service

Create:

apps/mission-command-service

Purpose:

Model real spacecraft command workflows.

Features:

* Command Creation
* Command Validation
* Approval Workflow
* Scheduling
* Execution Tracking
* Audit Trail

Command Lifecycle:

CREATED
APPROVED
QUEUED
TRANSMITTED
RECEIVED
EXECUTED
VERIFIED
FAILED

Every lifecycle transition must generate:

* OTel Trace
* OTel Event
* Kafka Event
* Timeline Event

Implement RBAC:

Roles:

* Operator
* Flight Director
* Mission Manager
* Administrator

---

# Phase 10: Digital Twin Engine

Create:

apps/digital-twin-engine

Purpose:

Maintain a virtual spacecraft representation.

Model:

Satellite
Subsystems
Power
Thermal
ADCS
Communications
Payload
Propulsion

Capabilities:

* State Synchronization
* Health Projection
* Failure Simulation
* Historical Reconstruction

Expose APIs:

GET /twin/{satellite}
GET /twin/{satellite}/health
GET /twin/{satellite}/timeline

---

# Phase 11: Fleet Health Engine

Create:

crates/fleet-health

Purpose:

Evaluate fleet-wide health.

Metrics:

Fleet Availability
Fleet Risk Score
Fleet Utilization
Fleet Capacity
Fleet Readiness

Dashboard:

Healthy
Warning
Critical

Scalable to:

10 satellites
100 satellites
1000 satellites
10000 satellites

---

# Phase 12: Mission Timeline Engine

Create:

apps/mission-timeline

Purpose:

Store every operational event.

Sources:

Telemetry
Commands
Alerts
Incidents
Ground Station Events
Digital Twin Events

Timeline Example:

12:01 Battery Warning

12:03 Alert Generated

12:05 Operator Acknowledged

12:07 Payload Disabled

12:09 Thermal Recovery

Features:

* Event Correlation
* Search
* Replay Integration
* RCA Support

---

# Phase 13: AI Incident Commander

Create:

apps/ai-incident-commander

Purpose:

Provide mission intelligence.

Capabilities:

Incident Summaries
Root Cause Analysis
Anomaly Explanations
Recovery Recommendations
Mission Briefings

Example Queries:

Why is SAT-101 unhealthy?

What changed before anomaly detection?

Summarize the last 24 hours.

Generate postmortem report.

Generate operator handoff notes.

Supported Models:

OpenAI
Anthropic
Gemini

Implement RAG:

Knowledge Sources:

* Mission Events
* Telemetry
* Command History
* Runbooks
* Incidents
* Timeline Events

---

# Phase 14: Space Schema Registry

Create:

crates/space-schema-registry

Purpose:

Version telemetry schemas.

Support:

Telemetry V1
Telemetry V2
Telemetry V3

Features:

Schema Validation
Compatibility Checking
Migration Support
Version Tracking

---

# Phase 15: Real Data Sources

Keep simulator.

Add adapters.

Implement:

ISS Adapter
TLE Adapter
SatNOGS Adapter
N2YO Adapter

Architecture:

Source Adapter
↓
Normalization Layer
↓
Space OTel Collector
↓
Kafka
↓
Storage

---

# Phase 16: Constellation Simulator

Create:

apps/constellation-simulator

Capabilities:

100
500
1000
5000

simulated satellites.

Simulate:

Battery Aging
Thermal Drift
Orbit Changes
Ground Station Failures
Payload Failures
Network Congestion

---

# Phase 17: Ground Station Operations

Enhance existing ground station simulator.

Add:

Antenna Health
Backhaul Network Health
Weather Effects
Hardware Failures
Maintenance Windows

Metrics:

Availability
Latency
Packet Loss
Signal Quality

---

# Phase 18: Space Observability Platform

Support all four pillars:

Metrics
Logs
Traces
Profiles

Integrations:

Prometheus
Loki
Tempo
Mimir
Jaeger

Build a unified observability view.

---

# Frontend Enhancements

Enhance Mission Control Console.

Add:

Fleet Overview
Constellation Health
Mission Timeline
Ground Station Dashboard
Command Center
Incident Center
AI Copilot Chat
Digital Twin Viewer

Three.js Enhancements:

Live Orbits
Ground Coverage
Communication Beams
Collision Warnings
Orbital Paths

---

# Rust Engineering Standards

Use:

Tokio
Axum
Serde
Tracing
OpenTelemetry
SQLx

Requirements:

100% async services

Structured logging

Graceful shutdown

Health endpoints

Metrics endpoints

OpenAPI specs

Integration tests

Property tests

Load tests

Benchmark suite

---

# Documentation

Generate:

Architecture Diagrams
Sequence Diagrams
Data Flow Diagrams
ADR Documents
API Specifications
Operational Runbooks

---

# Long-Term Goal

Open Orbitlogs should become:

The OpenTelemetry-Native Operating System for Space Missions.

A platform capable of operating, monitoring, troubleshooting, replaying, and analyzing thousands of spacecraft and ground stations while providing AI-assisted mission operations.

Preserve all existing functionality and evolve the platform incrementally with backward compatibility.
