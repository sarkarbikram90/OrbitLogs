# Open SpaceOps: Apache Kafka Topology & Design

This document details the event-streaming design, topic architectures, partition strategies, and durability policies of the **Apache Kafka** event backbone.

---

## 1. Topic Specifications & Retention

The platform operates three core topics to segregate telemetry processing stages:

| Topic Name | Partitions | Key Strategy | Retention Period | Description |
| :--- | :--- | :--- | :--- | :--- |
| **`telemetry.raw`** | `3` | `satellite_id` | `7 days` (168h) | Raw ingested stream awaiting database persistence |
| **`telemetry.processed`** | `3` | `satellite_id` | `30 days` (720h) | Successfully processed, validated and anomaly-checked stream |
| **`telemetry.invalid`** | `1` | `None` (Null) | `14 days` (336h) | Dead-letter queue containing bad payloads & failure logs |

---

## 2. Key-Based Partitioning & Order Guarantees

In time-series satellite telemetry, out-of-order execution ruins metrics calculations and state-tracking histories.
- **Problem**: If round-robin partitioning is used, telemetry packets from a single satellite (`SAT-0001`) land on different partitions and are processed by different consumer threads in arbitrary order.
- **Solution**: Open SpaceOps enforces **key-based partitioning** using the `satellite_id` as the message key.
- **Guarantee**: Kafka hashes the message key and routes all messages with the same key to the same partition:
  $$\text{Partition} = \text{Hash}(\text{satellite\_id}) \pmod{\text{Total Partitions}}$$
  This guarantees that all packets for a specific satellite are consumed and written to the database in exact chronological sequence.

---

## 3. Scaling & Consumer Groups

We utilize consumer groups to scale performance horizontally:
- **Consumer Group**: `telemetry-consumer-group`.
- **Scaling Limit**: With 3 partitions on `telemetry.raw`, we can scale up to **3 concurrent consumer processes** in parallel. Each consumer process will handle exactly 1 partition.
- **Rebalance**: If a consumer process dies, the Kafka broker automatically detects the heart-beat failure and reassigns its partition to the surviving consumers, ensuring zero downtime.
