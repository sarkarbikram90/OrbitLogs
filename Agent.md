# OrbitLogs V3 – Satellite Mission Operations Platform

You are a Staff Level engineer at SpaceX Starlink.

## Mission

OrbitLogs currently provides:

* Satellite telemetry simulation
* CCSDS packet generation
* Kafka streaming
* Mission Health Engine
* Ground Station Simulator
* Mission Timeline
* Mission Replay
* Mission Command Service
* Digital Twin
* OpenTelemetry
* AI Incident Commander
* Grafana Dashboards
* Three.js Mission Control UI

These systems are production-ready and MUST remain unchanged.

The objective of V3 is to evolve OrbitLogs into a realistic Mission Operations Platform used by satellite operators, Earth Observation companies, Space Situational Awareness providers, and mission control centers.

All additions must be implemented as new services and modules.

Do not remove or rewrite existing functionality.

---

# FEATURE GROUP 1

ORBITAL MECHANICS ENGINE

Create a dedicated Orbit Propagation Service.

Responsibilities:

* Parse TLE files
* Orbit propagation
* Future position prediction
* Ground track generation
* Visibility calculations

Support:

* SGP4
* TLE ingestion
* Orbit updates

APIs:

GET /orbit/{satellite}

GET /orbit/{satellite}/future

GET /orbit/{satellite}/ground-track

Store:

* current position
* velocity vectors
* orbital elements

Display orbit path on Mission Control UI.

---

# FEATURE GROUP 2

PASS PREDICTION ENGINE

Implement realistic pass prediction.

Calculate:

* Next pass
* AOS
* LOS
* Duration
* Maximum elevation

Ground stations:

* Svalbard
* Troll
* Kiruna
* McMurdo

Additional:

* Bangalore
* Hyderabad
* Delhi
* Siliguri

Output:

{
"station":"Bangalore",
"aos":"timestamp",
"los":"timestamp",
"duration":"minutes",
"max_elevation":"degrees"
}

Expose APIs.

Visualize upcoming passes.

---

# FEATURE GROUP 3

GROUND STATION OPERATIONS

Create Ground Operations Center.

Track:

* antenna availability
* RF utilization
* contact windows
* uplink success
* downlink success

Generate:

GROUND_CONTACT_STARTED

GROUND_CONTACT_ENDED

UPLINK_FAILURE

DOWNLINK_FAILURE

CONTACT_MISSED

Store all events.

---

# FEATURE GROUP 4

MISSION PLANNER

Create planning system.

Users should schedule:

* payload activation
* image capture
* software update
* orbit maneuver
* command execution

Workflow:

Draft

Review

Approved

Scheduled

Executed

Cancelled

Provide calendar-style UI.

---

# FEATURE GROUP 5

SPACE SITUATIONAL AWARENESS

Create SSA Service.

Simulate:

* debris objects
* neighboring satellites
* conjunction events

Features:

Collision probability

Nearest object

Conjunction warnings

Distance calculations

Generate:

CONJUNCTION_ALERT

HIGH_RISK_CONJUNCTION

Store history.

Display risk score.

---

# FEATURE GROUP 6

FLIGHT DYNAMICS SERVICE

Calculate:

* orbital drift
* altitude decay
* fuel consumption
* maneuver effectiveness

Create recommendations.

Example:

"Satellite ORB-001 requires station keeping within 5 days."

Provide APIs.

---

# FEATURE GROUP 7

AUTONOMOUS MISSION OPERATIONS

Create AI Mission Director.

Responsibilities:

Analyze:

Telemetry

Ground station state

Commands

Incidents

Anomalies

Timeline

Recommend actions.

Examples:

"Battery degradation detected."

"Recommend safe mode."

"Recommend contact with Bangalore station."

"Recommend thermal subsystem inspection."

Use RAG architecture.

---

# FEATURE GROUP 8

COMMAND EXECUTION WORKFLOW

Upgrade command service.

Current:

Operator -> Command

New:

Operator

↓

Flight Controller

↓

Mission Director

↓

Approval

↓

Uplink

↓

Execution

↓

Verification

↓

Mission Timeline

Track:

command latency

success rate

verification status

---

# FEATURE GROUP 9

DIGITAL TWIN 2.0

Enhance Digital Twin.

Twin should model:

battery

solar panels

thermal state

communications

payload state

orbit state

fuel state

Commanding twin should impact future telemetry.

Failures should propagate realistically.

---

# FEATURE GROUP 10

MISSION EVENT BUS

Create dedicated events.

Topics:

mission.events

mission.timeline

mission.commands

mission.contacts

mission.alerts

mission.incidents

mission.anomalies

mission.planning

mission.ssa

mission.flight_dynamics

Standardize schemas.

---

# FEATURE GROUP 11

OPERATOR TRAINING MODE

Create simulator mode.

Inject:

battery failure

thermal runaway

communication outage

attitude drift

ground station outage

payload malfunction

Operators must respond.

Track:

response time

corrective actions

recovery success

Generate scorecard.

---

# FEATURE GROUP 12

SPACECRAFT HEALTH SCORING

Create composite health score.

Subsystems:

Power

Thermal

ADCS

Comms

Payload

Flight Computer

Propulsion

Output:

0-100 health score

Health trend

Remaining useful life

Failure prediction

---

# FEATURE GROUP 13

MISSION POSTMORTEM GENERATOR

After incident:

Automatically generate:

Timeline

Root Cause

Impacted Systems

Commands Executed

Telemetry Summary

Recommended Fixes

Use AI Incident Commander.

Export PDF.

---

# FEATURE GROUP 14

ORBITLOGS KNOWLEDGE GRAPH

Create graph relationships.

Satellite

Ground Station

Command

Alert

Incident

Operator

Mission

Anomaly

Use graph database.

Support impact analysis.

Examples:

"What incidents affected ORB-001?"

"What commands preceded anomaly A-123?"

---

# FEATURE GROUP 15

MISSION CONTROL UI V3

Create screens:

Fleet Overview

Mission Planner

Ground Operations

Pass Schedule

SSA Dashboard

Flight Dynamics

Mission Timeline

Incident Center

Command Center

Digital Twin

Training Simulator

AI Mission Director

Provide executive dashboard and operator dashboard.

---

# NON-FUNCTIONAL REQUIREMENTS

Maintain:

Rust

Kafka

TimescaleDB

OpenTelemetry

Prometheus

Grafana

Jaeger

React

Docker

Kubernetes

GitHub Actions

All new services must be observable.

All services must emit:

Metrics

Logs

Traces

All workflows must appear in Mission Timeline.

All services must support local development and Docker Compose deployment.

Goal:

OrbitLogs should resemble a modern commercial Mission Operations Center capable of operating thousands of satellites while demonstrating expertise in:

* Satellite Operations
* Ground Segment Operations
* Flight Dynamics
* Orbital Mechanics
* Space Situational Awareness
* Distributed Systems
* Observability
* AI-Assisted Operations
* Rust Engineering
