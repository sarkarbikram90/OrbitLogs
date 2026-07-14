# OrbitLogs: Observability Design Specifications
### Version: v4.5.0 (Latest)

This document outlines the metrics registries, Prometheus scrape configurations, distributed tracing spans, and Grafana visualization frameworks of **OrbitLogs**.

---

## 1. System Metrics Registry

Every service exposes a standard Prometheus scrape endpoint `/metrics` containing custom system metrics:

| Metric Name | Type | Labels | Description |
| :--- | :--- | :--- | :--- |
| `telemetry_received_total` | Counter | `satellite_id`, `status` | Total HTTP API request validation counts |
| `telemetry_processed_total` | Counter | `satellite_id` | Successfully consumed and persisted event counts |
| `telemetry_invalid_total` | Counter | `reason` | Count of rejected payloads, labeled by specific error details |
| `processing_latency` | Histogram | *None* | Latency of basic HTTP parsing and validation layers |
| `kafka_publish_latency` | Histogram | *None* | Network latency of publishing raw messages to Kafka |
| `database_insert_latency` | Histogram | *None* | Latency of batch writing records to TimescaleDB |
| `consumer_lag_messages` | Gauge | `topic`, `partition` | Partition processing delay (alerts at >1000 messages) |

---

## 2. Distributed Tracing Flow

Distributed tracing is instrumented natively in Rust via **OpenTelemetry**:
- **Tracer Provider**: Exposes tracing registry using a batch processor, sending traces to the **OTel Collector** via OTLP gRPC (`otel-collector:4317`).
- **Exporter Backend**: OTel Collector batches and exports spans to **Jaeger** (`jaeger:16686`), providing visual analysis of request lifecycles.
- **Trace Propagation**: W3C headers inject and extract `traceparent` parameters over Kafka RecordHeaders, ensuring correlation spans link simulator posts through Kafka to TimescaleDB.

---

## 3. Grafana Visualizations (HSL Color System)

Grafana dashboards are provisioned dynamically with a premium, visually engaging **dark-mode aesthetic** using curated HSL color maps:
- **Success Metrics (Throughput)**: harmonized soft greens representing healthy packet flow.
- **Degraded States (Latency Spikes)**: warning orange representing queue backpressure or slow writes.
- **Critical Anomaly Events (Thermal Spikes, Low Fuel)**: vibrant deep crimson to grab operator focus immediately.
