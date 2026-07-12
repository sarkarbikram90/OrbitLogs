# OrbitLogs: Kafka Event Payloads & Schemas

This document defines the JSON payload schemas, serialization formats, partition keys, and structures for the event streams routed through the **Apache Kafka** event backbone of **OrbitLogs**.

---

## 1. `telemetry.raw` & `telemetry.processed`
- **Serialization**: JSON
- **Key Strategy**: `satellite_id` (enforces order guarantee per satellite)
- **JSON Payload Schema**:
```json
{
  "satellite_id": "SAT-0001",
  "subsystem_id": "ADCS",
  "mission_id": "MISSION-CONSTELLATION",
  "timestamp": "2026-07-12T02:00:00Z",
  "battery_voltage": 13.8,
  "battery_current": 1.2,
  "solar_panel_output": 120.0,
  "cpu_usage": 15.0,
  "memory_usage": 30.0,
  "temperature": 25.0,
  "orientation_pitch": 0.0,
  "orientation_roll": 0.0,
  "orientation_yaw": 0.0,
  "velocity": 7.6,
  "altitude": 550.0,
  "fuel_remaining": 85.0,
  "communication_latency": 25.0,
  "packet_loss": 0.1,
  "signal_strength": -65.0,
  "payload_power_draw": 15.0,
  "payload_active": true,
  "event_id": "a9a8b123-d890-4cba-abcd-1234567890ab"
}
```

---

## 2. `telemetry.invalid` (Dead Letter Queue)
- **Serialization**: JSON (Fallback) or raw string payload
- **Key Strategy**: None
- **Schema**: Contains the raw unparseable input along with verification diagnostics details and timestamp.
```json
{
  "raw_payload": "0x00011234... (unparseable binary or malformed JSON)",
  "error_reason": "CRC verification failed",
  "timestamp": "2026-07-12T02:00:01Z"
}
```

---

## 3. `mission.health`
- **Serialization**: JSON
- **Key Strategy**: `satellite_id`
- **JSON Payload Schema**:
```json
{
  "satellite_id": "SAT-0001",
  "overall_state": "GREEN",
  "overall_score": 98,
  "explanation": "All subsystems nominal. Battery charging cycle active.",
  "subsystems": {
    "eps": "NOMINAL",
    "tcs": "NOMINAL",
    "adcs": "NOMINAL",
    "ttc": "NOMINAL",
    "obc": "NOMINAL",
    "propulsion": "NOMINAL",
    "payload": "NOMINAL"
  },
  "timestamp": "2026-07-12T02:00:02Z"
}
```

---

## 4. `groundstation.health`
- **Serialization**: JSON
- **Key Strategy**: `station_id`
- **JSON Payload Schema**:
```json
{
  "timestamp": "2026-07-12T02:00:00Z",
  "station_id": "GS-0001",
  "station_name": "Svalbard Ground Station (Norway)",
  "status": "ONLINE",
  "antenna_health": 100.0,
  "weather_state": "CLEAR",
  "congestion": 15.0,
  "latency": 42.0,
  "bandwidth": 1000.0,
  "explanation": "All ground station systems operational.",
  "backhaul_health": 100.0,
  "packet_loss": 0.0,
  "signal_quality": 100.0,
  "maintenance": false
}
```

---

## 5. `command.events`
- **Serialization**: JSON
- **Key Strategy**: `satellite_id`
- **JSON Payload Schema**:
```json
{
  "command_id": "d7ef5b8e-3c0f-4889-b0ee-659f4296ab42",
  "satellite_id": "SAT-0001",
  "command_type": "ENABLE_PAYLOAD",
  "status": "APPROVED",
  "params": {
    "duty_cycle": 75.0
  },
  "operator_id": "OP-99",
  "approved_by": "FD-01",
  "timestamp": "2026-07-12T02:00:05Z",
  "trace_id": "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
}
```

---

## 6. `timeline.events`
- **Serialization**: JSON
- **Key Strategy**: `satellite_id`
- **JSON Payload Schema**:
```json
{
  "timestamp": "2026-07-12T02:00:10Z",
  "event_id": "f5127ef3-cf90-4cda-921a-d4c382103abc",
  "source": "COMMAND",
  "satellite_id": "SAT-0001",
  "event_type": "COMMAND_VERIFIED",
  "severity": "INFO",
  "message": "Command ENABLE_PAYLOAD successfully executed on satellite SAT-0001.",
  "details": {
    "command_id": "d7ef5b8e-3c0f-4889-b0ee-659f4296ab42"
  }
}
```

---

## 7. `orbit.state`
- **Serialization**: JSON
- **Key Strategy**: `satellite_id`
- **JSON Payload Schema**:
```json
{
  "event_type": "orbit.position.updated",
  "satellite_id": "SAT-0001",
  "timestamp": "2026-07-12T02:00:00Z",
  "latitude": 52.5200,
  "longitude": 13.4050,
  "altitude": 550.25,
  "velocity": 7.66
}
```

---

## 8. `command.verification.events`
- **Serialization**: JSON
- **Key Strategy**: `satellite_id`
- **JSON Payload Schema**:
```json
{
  "verification_id": "ca1278ef-1a0c-456b-bcde-3d4ef567890a",
  "command_id": "d7ef5b8e-3c0f-4889-b0ee-659f4296ab42",
  "satellite_id": "SAT-0001",
  "status": "VERIFIED",
  "reason": "Payload telemetry verified payload_active == true",
  "timestamp": "2026-07-12T02:00:10Z"
}
```
