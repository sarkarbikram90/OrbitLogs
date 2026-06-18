# OrbitLogs: Apache Kafka Topology & Design
### Version: v4.0.0 (Latest)

This document details the event-streaming design, topic architectures, partition strategies, and durability policies of the **Apache Kafka** event backbone of **OrbitLogs**.

---

## 1. Topic Specifications & Retention

The platform operates several topics to decouple telemetry collection, state propagation, and operational verification loops:

| Topic Name | Partitions | Key Strategy | Retention Period | Description |
| :--- | :--- | :--- | :--- | :--- |
| **`telemetry.raw`** | `3` | `satellite_id` | `7 days` (168h) | Raw ingested stream awaiting database persistence |
| **`telemetry.processed`** | `3` | `satellite_id` | `30 days` (720h) | Successfully processed, validated and anomaly-checked stream |
| **`telemetry.invalid`** | `1` | `None` (Null) | `14 days` (336h) | Dead-letter queue containing bad payloads & failure logs |
| **`mission.health`** | `3` | `satellite_id` | `30 days` (720h) | Fleet Health Engine status output |
| **`groundstation.health`** | `3` | `station_id` | `7 days` (168h) | Ground station signal and pass parameters |
| **`command.events`** | `3` | `satellite_id` | `30 days` (720h) | Flight commands in pipeline (`CREATED`/`QUEUED`/etc.) |
| **`timeline.events`** | `3` | `satellite_id` | `30 days` (720h) | Unified event stream for operational analysis |
| **`orbit.state`** | `3` | `satellite_id` | `7 days` (168h) | Real-time propagated SGP4 orbital positions |
| **`command.verification.events`**| `3` | `satellite_id` | `30 days` (720h) | Telemetry matches for command completion |

---

## 2. Key-Based Partitioning & Order Guarantees

In time-series satellite telemetry, out-of-order execution ruins metrics calculations and state-tracking histories.
- **Problem**: If round-robin partitioning is used, telemetry packets from a single satellite (`SAT-0001`) land on different partitions and are processed by different consumer threads in arbitrary order.
- **Solution**: OrbitLogs enforces **key-based partitioning** using the `satellite_id` as the message key.
- **Guarantee**: Kafka hashes the message key and routes all messages with the same key to the same partition:
  $$\text{Partition} = \text{Hash}(\text{satellite\_id}) \pmod{\text{Total Partitions}}$$
  This guarantees that all packets for a specific satellite are consumed and written to the database in exact chronological sequence.

---

## 3. mTLS Network Transport Hardening

Kafka is configured to run on a 3-node KRaft cluster with full **Mutual TLS (mTLS)** validation:
*   A PKCS12 keystore/truststore generator container automatically runs on start.
*   Brokers enforce SSL with mutual client authentication (`ssl.client.auth=required`).
*   Internal client connections utilize PEM CA, certificate, and key configurations via `KafkaConfigBuilder` in `spaceops-models`.

---

## 4. Delivery Semantics & Manual Offset Commits

To prevent telemetry loss during service crashes or database failures, OrbitLogs implements **At-Least-Once Delivery** semantics.

*   **Auto Commit Disabled**: `enable.auto.commit` is set to `false` in the Kafka consumer config. This prevents the consumer from automatically committing offsets before telemetry is safely stored.
*   **Manual Offset Management**: The consumer manually commits partition offsets (`CommitMode::Async`) only *after*:
    1. Telemetry is successfully persisted to the TimescaleDB database.
    2. Or, if the payload is invalid and rejected, after it has been safely routed to the Dead-Letter Queue (DLQ) topic `telemetry.invalid`.
*   **Handling DB/DLQ Failures**: If writing to the database or routing to the DLQ fails, the offset is *not* committed. When the consumer restarts or partition rebalancing occurs, Kafka will redeliver these messages, preventing any data loss.
