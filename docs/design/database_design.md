# OrbitLogs: TimescaleDB Database Design & Hardening
### Version: v4.5.0 (Latest)

This document details the database schema, time-series optimizations, hypertable configurations, compression indices, and data retention policies of **OrbitLogs**.

---

## 1. Relational Schema & Hypertables

OrbitLogs utilizes **TimescaleDB** to split telemetry and events into time-partitioned hypertables, while maintaining relational integrity for metadata tables.

```mermaid
erDiagram
    satellites {
        varchar_50 id PK
        varchar_100 name
        varchar_50 model
        timestamp launch_date
        varchar_20 status
        timestamp created_at
    }
    telemetry {
        timestamp timestamp PK
        varchar_50 satellite_id FK
        double battery_voltage
        double battery_current
        double solar_panel_output
        real cpu_usage
        real memory_usage
        double temperature
        real orientation_pitch
        real orientation_roll
        real orientation_yaw
        double velocity
        double altitude
        real fuel_remaining
        real communication_latency
        real packet_loss
        real signal_strength
        uuid event_id
    }
    events {
        timestamp timestamp PK
        varchar_50 satellite_id FK
        varchar_50 event_type
        varchar_20 severity
        text message
        jsonb details
    }
    orbit_positions {
        timestamp timestamp PK
        varchar_50 satellite_id FK
        double latitude
        double longitude
        double altitude
        double velocity
        double orbital_period
        double inclination
        double eccentricity
    }
    tle_records {
        uuid id PK
        varchar_50 satellite_id
        timestamp epoch
        text line1
        text line2
        double inclination
        double raan
        double eccentricity
        double arg_of_perigee
        double mean_anomaly
        double mean_motion
        double semi_major_axis
    }
    pass_predictions {
        uuid id PK
        varchar_50 satellite_id
        varchar_50 station_id
        timestamp aos
        timestamp los
        double duration
        double max_elevation
    }
    command_verifications {
        uuid id PK
        uuid command_id
        varchar_50 satellite_id
        varchar_50 command_type
        varchar_50 expected_telemetry_field
        varchar_50 expected_telemetry_value
        varchar_50 status
        timestamp verified_at
    }

    satellites ||--o{ telemetry : "emits"
    satellites ||--o{ events : "logs"
    satellites ||--o{ orbit_positions : "occupies"
    satellites ||--o{ conjunction_events : "threatens"

    conjunction_events {
        timestamp timestamp PK
        varchar_50 primary_satellite_id FK
        varchar_50 secondary_object_id
        timestamp tca
        double miss_distance_km
        double radial_miss_km
        double intrack_miss_km
        double crosstrack_miss_km
        double probability_of_collision
        varchar_20 risk_level
    }

    conjunction_maneuvers {
        uuid id PK
        varchar_50 satellite_id FK
        uuid conjunction_event_id
        double delta_v_magnitude
        double fuel_cost_kg
        varchar_50 status
        timestamp recommended_at
    }

    contact_schedules {
        uuid id PK
        varchar_50 satellite_id
        varchar_50 station_id
        timestamp start_time
        timestamp end_time
        varchar_20 priority
        varchar_20 status
    }
```

---

## 2. Hypertable Time-Partitioning

The following high-throughput tables are converted to TimescaleDB hypertables:
*   `telemetry` (Partitioned on `timestamp`, interval: `1 day`)
*   `events` (Partitioned on `timestamp`, interval: `1 day`)
*   `orbit_positions` (Partitioned on `timestamp`, interval: `1 day`)

By keeping chunk intervals to 1 day, index lookups remain small enough to fit within available RAM, preventing disk-thrashing bottlenecks during telemetry inserts.

---

## 3. High-Ratio Compression Policy

To optimize disk footprint, TimescaleDB compression is configured:
*   **Segmentation Key**: `satellite_id` (data is grouped together per satellite).
*   **Ordering Key**: `timestamp DESC` (enables delta-delta compression).
*   **Policy**: Automatically compresses chunks older than **7 days**.
*   **Results**: Reduces disk footprints by **90%**, optimizing historical analysis.

---

## 4. Automatic Retention Policies

To protect against storage blowout, data retention policies are enforced:
*   **Telemetry Retention**: Automatically drops chunks older than **90 days**.
*   **Events Retention**: Drops chunks older than **180 days** to preserve incident logs for SRE audits.
*   **Orbit Positions Retention**: Automatically drops chunks older than **30 days** (relying on raw TLE files for long-term orbital regressions).

---

## 5. Continuous Aggregates

To prevent CPU exhaustion during heavy Grafana dashboard queries, TimescaleDB **Continuous Aggregates** pre-calculate hourly averages of key telemetry metrics:

*   `hourly_battery_avg`: Average `battery_voltage` per satellite per hour.
*   `hourly_solar_avg`: Average `solar_panel_output` per satellite per hour.
*   `hourly_cpu_avg`: Average `cpu_usage` per satellite per hour.
*   `hourly_temperature_avg`: Average `temperature` per satellite per hour.
*   `hourly_altitude_avg`: Average `altitude` per satellite per hour.

These materialized views are defined in migration `20260101000011_continuous_aggregates.sql` and refreshed automatically by TimescaleDB policies.
