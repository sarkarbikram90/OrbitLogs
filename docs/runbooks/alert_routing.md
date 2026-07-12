# Alertmanager: Alert Routing Configurations

This runbook documents the routing strategy, group intervals, receiver endpoints, and webhook payload structure configured within the **Alertmanager** system.

---

## 1. Alert Routing Tree

All alerts ingested by Prometheus are forwarded to Alertmanager, which evaluates them against the routing tree defined in `alertmanager.yml`:

```yaml
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'webhook-placeholder'

receivers:
  - name: 'webhook-placeholder'
    webhook_configs:
      - url: 'http://localhost:5001/webhook'
```

### Routing Parameters
- **`group_by: ['alertname']`**: Alerts with the same name are aggregated into a single group notification.
- **`group_wait: 10s`**: Wait time before sending a notification to batch incoming alerts.
- **`group_interval: 10s`**: Minimum interval between consecutive alerts within the same group.
- **`repeat_interval: 1h`**: Time to wait before repeating an alert notification that is already active.
- **`receiver: 'webhook-placeholder'`**: Default receiver target.

---

## 2. Webhook Target Configuration

The target receiver for all notifications is configured as an HTTP POST webhook:
- **Webhook Endpoint**: `http://localhost:5001/webhook`
- **Purpose**: Integrates alerts with team chat protocols, notification channels, and custom pager gateways.

---

## 3. Webhook Payload Structure

When Alertmanager triggers a webhook receiver, it posts a JSON payload matching the following schema:

```json
{
  "receiver": "webhook-placeholder",
  "status": "firing",
  "alerts": [
    {
      "status": "firing",
      "labels": {
        "alertname": "HighConsumerLag",
        "topic": "telemetry.raw",
        "group": "telemetry-consumer-group",
        "severity": "critical"
      },
      "annotations": {
        "summary": "High consumer lag detected",
        "description": "Kafka consumer group telemetry-consumer-group is lagging on topic telemetry.raw by more than 1000 messages."
      },
      "startsAt": "2026-07-12T02:00:00Z",
      "endsAt": "0001-01-01T00:00:00Z",
      "generatorURL": "http://prometheus:9090/graph?g0.expr=consumer_lag..."
    }
  ],
  "groupLabels": {
    "alertname": "HighConsumerLag"
  },
  "commonLabels": {
    "alertname": "HighConsumerLag",
    "severity": "critical"
  },
  "commonAnnotations": {
    "summary": "High consumer lag detected"
  },
  "externalURL": "http://alertmanager:9093",
  "version": "4"
}
```
