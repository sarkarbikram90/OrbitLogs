# AGENT.md

# Open SpaceOps

## Vision

Open SpaceOps is not a monitoring dashboard.

Open SpaceOps is an open-source Mission Operations Platform that demonstrates how modern satellite operators monitor, analyze, troubleshoot, replay, and recover spacecraft using contemporary observability technologies.

The project combines:

* Spacecraft Telemetry
* Distributed Systems
* Observability Engineering
* Site Reliability Engineering
* Event Streaming
* Mission Operations
* Artificial Intelligence

The long-term goal is to build a miniature version of:

* SpaceX Mission Control
* NASA Mission Operations
* ESA Ground Segment
* Datadog
* Grafana Cloud
* Splunk Observability
* Modern AI Incident Management Platforms

The project should be technically realistic, highly observable, well-documented, professionally engineered, and capable of serving as a flagship GitHub portfolio repository.

---

# Core Philosophy

The project must prioritize:

1. Mission Operations Realism
2. Engineering Quality
3. Reliability
4. Observability
5. Documentation
6. Testing
7. Security
8. Scalability

Feature count is less important than realism and execution quality.

Every architectural decision should answer:

> Would a mission operations engineer, SRE, platform engineer, or space-tech engineer find this realistic and useful?

---

# Engineering Principles

## Principle 1

Never break existing functionality.

All new capabilities must be backward compatible.

---

## Principle 2

Every service must expose:

* Logs
* Metrics
* Traces

Observability is mandatory.

---

## Principle 3

Every feature requires:

* Architecture documentation
* Design documentation
* Diagrams
* Unit tests
* Integration tests

---

## Principle 4

All services communicate through event-driven architecture whenever practical.

Kafka is the primary integration mechanism.

---

## Principle 5

Documentation quality should be comparable to:

* Grafana Labs
* Datadog
* Cloudflare
* Stripe
* Google SRE

---

## Principle 6

All operational assumptions must be documented.

Failure scenarios must be documented.

Recovery procedures must be documented.

---

## Principle 7

Every subsystem must be observable.

Every event must be traceable.

Every failure must be diagnosable.

---

# Current Foundation (V1)

Existing Architecture:

Telemetry Simulator
↓
Rust Telemetry Collector
↓
Kafka
↓
TimescaleDB/PostgreSQL
↓
Grafana

This foundation must remain stable.

Future work should build on top of it rather than replacing it.

---

# Target End-State Architecture

Spacecraft Subsystems
↓
Telemetry Packet Layer
↓
Ground Station Network
↓
Telemetry Collector
↓
Kafka Event Bus
↓
Mission Health Engine
↓
Telemetry Archive
↓
Replay Engine
↓
Mission Control UI
↓
AI Incident Commander

---

# Phase 1

## Spacecraft Subsystem Architecture

Replace simplistic telemetry generation with realistic spacecraft subsystem modeling.

Create:

* Power System
* Thermal Control System
* Communications System
* Propulsion System
* ADCS
* Payload System
* Flight Computer

Each subsystem must emit independent telemetry.

Telemetry must include:

* satellite_id
* subsystem_id
* mission_id
* timestamp

Subsystem failures must propagate realistic side effects.

Example:

Thermal failure
↓
CPU throttling
↓
Communication degradation
↓
Mission health degradation

Deliverables:

* Architecture document
* Sequence diagrams
* Failure mode documentation
* Unit tests
* Integration tests

---

# Phase 2

## Mission Health Engine

Create:

apps/mission-health-engine

Purpose:

Convert raw telemetry into operational state.

Input:

Kafka telemetry streams

Output:

Fleet Health Score

Satellite Health Score

Subsystem Health Score

Health States:

GREEN

YELLOW

RED

Health assessments should be explainable.

Publish results to:

mission.health

Store mission health history.

Create Grafana dashboards.

Mission operators should see:

Mission Status

instead of

Raw Metrics

---

# Phase 3

## Ground Station Simulator

Create:

apps/ground-station-simulator

Purpose:

Simulate realistic space-to-ground communications.

Model:

* Signal Strength
* Packet Loss
* Antenna Health
* Weather Interference
* Latency
* Bandwidth
* Outages

Create scenarios:

* Ground station outage
* Severe weather
* Antenna degradation
* Network congestion

Publish:

groundstation.health

Integrate with mission health engine.

---

# Phase 4

## Telemetry Packet Layer

Create:

crates/spaceops-packets

Implement:

TelemetryPacket

TelemetryFrame

PacketEncoder

PacketDecoder

Capabilities:

* Binary serialization
* Versioning
* Checksum validation
* Metadata support

Maintain JSON compatibility for external APIs.

Telemetry should begin resembling real spacecraft telemetry rather than generic monitoring data.

Include documentation explaining simplified CCSDS-inspired design concepts.

---

# Phase 5

## Mission Replay Engine

Create:

apps/mission-replay-engine

Purpose:

Answer:

"What happened before the failure?"

Capabilities:

Replay:

* Telemetry
* Events
* Health changes
* Anomalies

Replay speeds:

* 1x
* 5x
* 10x
* 50x
* 100x

Features:

* Pause
* Resume
* Timeline scrubbing
* Event markers
* Failure markers

Store replay metadata.

Create replay APIs.

Create replay dashboards.

Generate sequence diagrams.

---

# Phase 6

## Mission Control User Interface

Replace Grafana-only operational experience.

Create:

mission-control-ui

Technology:

* React
* TypeScript
* WebSocket
* Three.js

Capabilities:

Fleet Overview

Satellite Status

Mission Status

Anomalies

Ground Station Status

Replay Controls

Mission Timeline

Mission Health Visualization

Grafana remains available.

Mission Control becomes the primary operator interface.

---

# Phase 7

## Orbital Visualization

Create:

3D Earth Visualization

Capabilities:

* Real-time satellite tracking
* Orbit rendering
* Ground station rendering
* Coverage maps
* Historical playback

Display:

* Active anomalies
* Health state
* Communication links

Performance Goal:

1000+ satellites

Integrate with replay engine.

---

# Phase 8

## AI Incident Commander

Create:

apps/incident-commander

Purpose:

Assist mission operators during incidents.

Input:

* Telemetry
* Metrics
* Logs
* Traces
* Events
* Runbooks
* Mission history

Capabilities:

### Incident Summary

Generate:

* Executive summary
* Technical summary
* Impact assessment

---

### Timeline Generation

Generate:

Chronological incident timelines.

---

### Root Cause Analysis

Correlate:

* Telemetry
* Health changes
* Ground station failures
* System events

Provide:

* Confidence scores
* Supporting evidence

---

### Remediation Guidance

Generate:

* Recommended actions
* Recovery steps
* Runbook references

---

### Postmortem Generation

Generate:

* Markdown
* HTML
* PDF

Postmortems automatically.

---

# Observability Requirements

Every service must expose:

Metrics:

Prometheus

Logs:

Structured Logging

Traces:

OpenTelemetry

Required Dashboards:

* Fleet Health
* Satellite Health
* Ground Station Health
* Kafka Health
* Database Health
* Replay Activity
* AI Incident Activity

---

# Testing Requirements

Every feature requires:

Unit Tests

Integration Tests

End-to-End Tests

Load Tests

Failure Injection Tests

Coverage Target:

90%+

All CI pipelines must pass.

---

# Documentation Requirements

Every phase must include:

Architecture Document

Design Document

Sequence Diagrams

Component Diagrams

Deployment Diagrams

ER Diagrams

Operational Runbooks

Troubleshooting Guides

Benchmark Reports

All diagrams must be GitHub-renderable using Mermaid.

Documentation is a first-class deliverable.

---

# GitHub Repository Goals

The repository should demonstrate expertise in:

* Rust
* Kafka
* PostgreSQL
* TimescaleDB
* Prometheus
* Grafana
* OpenTelemetry
* Distributed Systems
* Observability Engineering
* SRE
* Mission Operations
* AI Systems
* Platform Engineering

The project should feel like a realistic mission operations platform rather than a generic monitoring application.

Every feature should reinforce that vision.

---

# Success Criteria

A senior engineer from:

* SpaceX
* NASA
* ESA
* ISRO
* Pixxel
* Skyroot
* Agnikul
* Grafana Labs
* Datadog
* Cloudflare
* Google

should be able to review the repository and immediately understand:

"This engineer understands observability, distributed systems, reliability engineering, and mission operations software."

Also create a docs/architecture/roadmap.md file and maintain a checklist for every phase. That gives a clear execution plan and gives future GitHub visitors a visible product roadmap rather than a collection of disconnected features.
