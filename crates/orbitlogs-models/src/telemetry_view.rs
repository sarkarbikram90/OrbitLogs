use chrono::{DateTime, Utc};
use serde::Deserialize;

// =========================================================================
// Zero-Copy Telemetry View (P0 #2 — Lifetime Annotations)
// =========================================================================
//
// A borrowed view into telemetry data for hot-path processing.
// Instead of deserializing into fully-owned TelemetryRecord (19+ heap
// strings), this borrows directly from the JSON byte buffer.
//
// Key Rust features demonstrated:
// - Explicit lifetime annotations ('buf)
// - Zero-copy deserialization via #[serde(borrow)]
// - Borrowed string references (&'buf str)

/// Zero-copy view into a telemetry record for hot-path processing.
///
/// Only includes the fields needed for health assessment — no heap
/// allocations for string fields when deserializing from a byte buffer.
///
/// # Lifetime
/// `'buf` is the lifetime of the underlying byte buffer (e.g., the Kafka
/// message payload). The view cannot outlive the buffer it borrows from.
///
/// # Examples
/// ```
/// use orbitlogs_models::telemetry_view::TelemetryView;
///
/// let json = br#"{"satellite_id":"SAT-0001","timestamp":"2026-01-01T00:00:00Z","battery_voltage":13.8,"temperature":28.5}"#;
/// let view = TelemetryView::from_json(json).unwrap();
/// assert_eq!(view.satellite_id, "SAT-0001");
/// ```
#[derive(Debug, Deserialize)]
pub struct TelemetryView<'buf> {
    /// Borrowed satellite ID — no heap allocation.
    #[serde(borrow)]
    pub satellite_id: &'buf str,

    pub timestamp: DateTime<Utc>,

    #[serde(default)]
    pub battery_voltage: Option<f64>,

    #[serde(default)]
    pub temperature: Option<f64>,

    #[serde(default)]
    pub fuel_remaining: Option<f32>,

    #[serde(default)]
    pub cpu_usage: Option<f32>,

    #[serde(default)]
    pub memory_usage: Option<f32>,

    #[serde(default)]
    pub packet_loss: Option<f32>,

    #[serde(default)]
    pub signal_strength: Option<f32>,

    #[serde(default)]
    pub orientation_pitch: Option<f32>,
}

impl<'buf> TelemetryView<'buf> {
    /// Parse a borrowed view directly from a JSON byte slice.
    ///
    /// The returned view borrows string fields from `bytes` — no heap
    /// allocations for satellite_id or other string fields.
    pub fn from_json(bytes: &'buf [u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Quick health check: returns true if any field is in a critical range.
    pub fn is_critical(&self) -> bool {
        self.battery_voltage.map_or(false, |v| v < 11.0)
            || self.temperature.map_or(false, |t| t > 75.0)
            || self.fuel_remaining.map_or(false, |f| f < 10.0)
            || self.cpu_usage.map_or(false, |c| c > 95.0)
            || self.packet_loss.map_or(false, |p| p > 20.0)
    }

    /// Compute a simple health score (0-100) from the borrowed view.
    pub fn quick_health_score(&self) -> i32 {
        let mut score = 100i32;

        if let Some(v) = self.battery_voltage {
            if v < 11.0 {
                score -= 30;
            } else if v < 12.5 {
                score -= 10;
            }
        }

        if let Some(t) = self.temperature {
            if t > 75.0 {
                score -= 30;
            } else if t > 45.0 {
                score -= 10;
            }
        }

        if let Some(f) = self.fuel_remaining {
            if f < 10.0 {
                score -= 20;
            } else if f < 20.0 {
                score -= 10;
            }
        }

        if let Some(c) = self.cpu_usage {
            if c > 95.0 {
                score -= 15;
            } else if c > 80.0 {
                score -= 5;
            }
        }

        score.max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_view() {
        let json = br#"{"satellite_id":"SAT-0001","timestamp":"2026-01-01T00:00:00Z"}"#;
        let view = TelemetryView::from_json(json).unwrap();
        assert_eq!(view.satellite_id, "SAT-0001");
        assert!(view.battery_voltage.is_none());
        assert!(view.temperature.is_none());
    }

    #[test]
    fn parse_full_view() {
        let json = br#"{
            "satellite_id": "SAT-0002",
            "timestamp": "2026-06-15T12:00:00Z",
            "battery_voltage": 13.8,
            "temperature": 28.5,
            "fuel_remaining": 85.0,
            "cpu_usage": 25.0,
            "memory_usage": 40.0,
            "packet_loss": 0.1,
            "signal_strength": -68.0,
            "orientation_pitch": 1.5
        }"#;
        let view = TelemetryView::from_json(json).unwrap();
        assert_eq!(view.satellite_id, "SAT-0002");
        assert_eq!(view.battery_voltage, Some(13.8));
        assert_eq!(view.temperature, Some(28.5));
        assert!(!view.is_critical());
    }

    #[test]
    fn critical_detection() {
        let json = br#"{
            "satellite_id": "SAT-0003",
            "timestamp": "2026-06-15T12:00:00Z",
            "battery_voltage": 10.5,
            "temperature": 80.0,
            "fuel_remaining": 5.0
        }"#;
        let view = TelemetryView::from_json(json).unwrap();
        assert!(view.is_critical());
    }

    #[test]
    fn health_score_nominal() {
        let json = br#"{
            "satellite_id": "SAT-0001",
            "timestamp": "2026-06-15T12:00:00Z",
            "battery_voltage": 14.0,
            "temperature": 30.0,
            "fuel_remaining": 90.0,
            "cpu_usage": 25.0
        }"#;
        let view = TelemetryView::from_json(json).unwrap();
        assert_eq!(view.quick_health_score(), 100);
    }

    #[test]
    fn health_score_degraded() {
        let json = br#"{
            "satellite_id": "SAT-0001",
            "timestamp": "2026-06-15T12:00:00Z",
            "battery_voltage": 10.0,
            "temperature": 80.0,
            "fuel_remaining": 5.0,
            "cpu_usage": 96.0
        }"#;
        let view = TelemetryView::from_json(json).unwrap();
        // -30 (battery) -30 (temp) -20 (fuel) -15 (cpu) = 5
        assert_eq!(view.quick_health_score(), 5);
    }

    #[test]
    fn zero_copy_borrows_from_buffer() {
        // This test verifies the lifetime constraint: the view borrows
        // from the buffer and cannot outlive it.
        let buffer = br#"{"satellite_id":"SAT-0005","timestamp":"2026-01-01T00:00:00Z"}"#;
        let view = TelemetryView::from_json(buffer).unwrap();
        // satellite_id is a &str borrowed from buffer — no String allocation
        assert_eq!(view.satellite_id, "SAT-0005");
    }

    #[test]
    fn view_ignores_unknown_fields() {
        // TelemetryView only cares about health-relevant fields.
        // Extra fields in the JSON are silently ignored (no deny_unknown_fields).
        let json = br#"{
            "satellite_id": "SAT-0001",
            "timestamp": "2026-01-01T00:00:00Z",
            "battery_voltage": 13.8,
            "some_extra_field": "should be ignored",
            "event_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;
        let view = TelemetryView::from_json(json).unwrap();
        assert_eq!(view.satellite_id, "SAT-0001");
    }
}
