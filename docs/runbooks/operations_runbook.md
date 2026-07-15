# OrbitLogs: Operations & Incident Runbook
### Version: v4.5.0 (Latest)

This document contains step-by-step troubleshooting procedures, maintenance guides, and mitigation strategies for common operational incidents on the **OrbitLogs** telemetry platform.

---

## 1. Incident: High Kafka Consumer Lag

### Symptom
- PromQL alert triggers: `HighConsumerLag` (triggers when partition lag exceeds 1000 messages for 5 minutes).
- Telemetry database write timestamps lag behind real-time.

### Triage Procedures
1. **Identify Lagging Partitions**:
   Log into a Kafka broker container and inspect consumer group partition states:
   ```bash
   docker exec -it orbitlogs-kafka-1 kafka-consumer-groups --bootstrap-server localhost:9092 --describe --group orbit-ops-group
   ```
2. **Review DB Ingestion Health**:
   Verify if consumer loops are throwing persistent DB write errors (look for `Database insert attempt X failed` in logs).
3. **Verify DB Connections**:
   Check active connection footprints in the TimescaleDB container:
   ```sql
   SELECT count(*), state FROM pg_stat_activity GROUP BY state;
   ```

### Mitigations
*   **Scale Consumer Threads**: Increase thread workers in the consumer binary settings.
*   **Horizontal Scaling**: If lag is spread across all partitions, scale up the consumer replica count to divide partition allocations.

---

## 2. Incident: TimescaleDB Disk Space Exhaustion

### Symptom
- Disk utilization exceeds 90%.
- Latency spikes as database writes block on disk access limits.

### Triage Procedures
1. **Review Hypertable Sizes**:
   Check chunk sizes on active hypertables:
   ```sql
   SELECT chunk_name, pg_size_pretty(total_bytes) FROM chunk_relation_size('telemetry');
   ```
2. **Verify Compression Health**:
   Check if compression policies are actively compressing chunks:
   ```sql
   SELECT * FROM hypertable_compression_stats('telemetry');
   ```

### Mitigations
*   **Force Manual Compression**:
   Run manual compression on historical database chunks:
   ```sql
   SELECT compress_chunk(c) FROM show_chunks('telemetry', older_than => INTERVAL '7 days') c;
   ```
*   **Prune Historical Data**:
   If disk limits remain critical, temporarily drop telemetry chunks older than 30 days:
   ```sql
   SELECT drop_chunks('telemetry', INTERVAL '30 days');
   ```

---

## 3. Incident: TLE Expiration or Expired Pass Predictions

### Symptom
- Timeline logs show `TLE_EXPIRED` warnings for active satellites.
- Real-time ground station pass calculations yield null predictions.

### Triage Procedures
1. **Query Latest TLE Records**:
   Check epoch timestamps in the database:
   ```sql
   SELECT satellite_id, epoch, created_at FROM tle_records ORDER BY epoch DESC LIMIT 10;
   ```
2. **Verify TLE Ingestion Pipeline**:
   Ensure TLE import endpoints are receiving valid data formats:
   ```bash
   curl -i -X POST http://localhost:8095/api/tle/import \
     -H "Content-Type: application/json" \
     -d '{"satellite_id": "SAT-0001", "line1": "...", "line2": "..."}'
   ```

### Mitigations
*   Uplink a fresh TLE set from Space-Track coordinates to trigger the automatic pass scheduler calculations.
