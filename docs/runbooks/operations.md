# Open SpaceOps: Operations & Incident Runbook

This document contains step-by-step troubleshooting procedures, maintenance guides, and mitigation strategies for common operational incidents on the **Open SpaceOps** telemetry platform.

---

## 1. Incident: High Kafka Consumer Lag

### Symptom
- PromQL alert triggers: `sum(kafka_consumergroup_lag) > 5000`.
- Telemetry database write timestamps lag behind real-time.

### Triage Procedures
1. **Identify Lagging Partitions**:
   Log into the Kafka container and inspect the partition offsets:
   ```bash
   docker exec -it spaceops-kafka kafka-consumer-groups --bootstrap-server localhost:9092 --describe --group telemetry-consumer-group
   ```
2. **Review DB Ingestion Health**:
   Check if the consumer is repeatedly failing database writes and retrying (look for `Database insert attempt X failed` in container logs).
3. **Check Connection Pools**:
   Query PG active connections inside the DB container:
   ```sql
   SELECT count(*), state FROM pg_stat_activity GROUP BY state;
   ```

### Mitigations
- **Vertical Scaling**: Spawn more thread workers in the consumer binary configuration.
- **Horizontal Scaling**: If lag is across all 3 partitions, spawn up to 3 replica container instances of `telemetry-collector` running in `run-consumer` mode to divide the load.

---

## 2. Incident: TimescaleDB Disk Space Exhaustion

### Symptom
- Disk utilization exceeds 90%.
- Insert latency spikes as writes block on storage limits.

### Triage Procedures
1. **Review Hypertable Sizes**:
   Connect to TimescaleDB and check chunk footprints:
   ```sql
   SELECT chunk_name, pg_size_pretty(total_bytes) FROM chunk_relation_size('telemetry');
   ```
2. **Verify Compression Health**:
   Check if compression policies are actively compressing chunks:
   ```sql
   SELECT * FROM hypertable_compression_stats('telemetry');
   ```

### Mitigations
- **Force Immediate Compression**:
   Run manual TimescaleDB compression on uncompressed historical chunks:
   ```sql
   SELECT compress_chunk(c) FROM show_chunks('telemetry', older_than => INTERVAL '7 days') c;
   ```
- **Prune Old Data**:
   If disk space remains critical, temporarily drop telemetry older than 30 days:
   ```sql
   SELECT drop_chunks('telemetry', INTERVAL '30 days');
   ```

---

## 3. Incident: Analyzing the Dead Letter Queue (DLQ)

### Symptom
- `telemetry_invalid_total` counter increases rapidly.
- Clients receive `400 Bad Request` responses on POST endpoints.

### Triage Procedures
1. **Inspect `telemetry.invalid` Messages**:
   Read quarantined messages from the invalid queue to analyze validation failure reasons:
   ```bash
   docker exec -it spaceops-kafka kafka-console-consumer --bootstrap-server localhost:9092 --topic telemetry.invalid --from-beginning --max-messages 10
   ```
2. **Correlate with Logs**:
   Look for `Validation failed for satellite X` log messages in the `telemetry-collector` container stream. Common failures include future-dated timestamps or parameter bounds violations.
