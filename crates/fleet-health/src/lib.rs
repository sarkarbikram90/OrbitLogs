use orbitlogs_models::HealthState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    pub state: HealthState,
    pub score: i32, // 0 to 100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteHealthSummary {
    pub id: String,
    pub overall_state: HealthState,
    pub overall_score: i32,  // 0 to 100
    pub fuel_remaining: f32, // 0 to 100
    pub cpu_usage: f32,      // 0 to 100
    pub memory_usage: f32,   // 0 to 100
    pub payload_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMetrics {
    pub availability: f64, // % of fleet that is online (GREEN or YELLOW)
    pub risk_score: f64,   // 0 to 100 (weighted degradation)
    pub utilization: f64,  // % cpu/mem/payload usage
    pub capacity: f64,     // % remaining capacity (e.g. fuel + overall margin)
    pub readiness: f64,    // % of fleet that is GREEN (fully ready)
    pub healthy_count: usize,
    pub warning_count: usize,
    pub critical_count: usize,
}

pub struct FleetHealthEngine;

impl FleetHealthEngine {
    pub fn calculate(satellites: &[SatelliteHealthSummary]) -> FleetMetrics {
        let total = satellites.len();
        if total == 0 {
            return FleetMetrics {
                availability: 100.0,
                risk_score: 0.0,
                utilization: 0.0,
                capacity: 100.0,
                readiness: 100.0,
                healthy_count: 0,
                warning_count: 0,
                critical_count: 0,
            };
        }

        let mut green = 0;
        let mut yellow = 0;
        let mut red = 0;
        let mut sum_fuel = 0.0;
        let mut sum_cpu = 0.0;
        let mut sum_mem = 0.0;
        let mut active_payloads = 0;
        let mut sum_score = 0.0;

        for sat in satellites {
            match sat.overall_state {
                HealthState::Green => green += 1,
                HealthState::Yellow => yellow += 1,
                HealthState::Red => red += 1,
            }
            sum_fuel += sat.fuel_remaining;
            sum_cpu += sat.cpu_usage;
            sum_mem += sat.memory_usage;
            if sat.payload_active {
                active_payloads += 1;
            }
            sum_score += sat.overall_score as f32;
        }

        let total_f = total as f64;
        let availability = ((green + yellow) as f64 / total_f) * 100.0;
        let readiness = (green as f64 / total_f) * 100.0;
        let avg_score = sum_score as f64 / total_f;
        let risk_score = 100.0 - avg_score; // Lower health score -> Higher risk

        // Average utilization of resources
        let avg_cpu = sum_cpu as f64 / total_f;
        let avg_mem = sum_mem as f64 / total_f;
        let payload_util = (active_payloads as f64 / total_f) * 100.0;
        let utilization = (avg_cpu + avg_mem + payload_util) / 3.0;

        // Capacity: blend of remaining fuel and status score
        let avg_fuel = sum_fuel as f64 / total_f;
        let capacity = (avg_fuel + avg_score) / 2.0;

        FleetMetrics {
            availability,
            risk_score,
            utilization,
            capacity,
            readiness,
            healthy_count: green,
            warning_count: yellow,
            critical_count: red,
        }
    }
}

// =========================================================================
// Advanced Traits & Generics (P1 #1, #2, #7, #10, #11, #12)
// =========================================================================

/// Trait for types that can provide telemetry readings.
///
/// The generic constraint `T: TelemetrySource` ensures we can only assess
/// health from validated telemetry — not raw bytes or unvalidated JSON.
pub trait TelemetrySource {
    fn battery_voltage(&self) -> Option<f64>;
    fn temperature(&self) -> Option<f64>;
    fn fuel_remaining(&self) -> Option<f32>;
    fn cpu_usage(&self) -> Option<f32>;
    fn packet_loss(&self) -> Option<f32>;
}

/// Trait for types that carry a timestamp.
///
/// Combined with `TelemetrySource` via trait composition in function bounds.
pub trait HasTimestamp {
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc>;
}

/// Assess satellite health using generic trait bounds and where clauses (P1 #1, #11).
///
/// The bounds guarantee `source` has valid telemetry fields and a timestamp —
/// checked at compile time, not runtime.
pub fn assess_health<T>(source: &T) -> HealthState
where
    T: TelemetrySource + HasTimestamp,
{
    let score = compute_health_score(source);

    if score >= 70 {
        HealthState::Green
    } else if score >= 40 {
        HealthState::Yellow
    } else {
        HealthState::Red
    }
}

/// Compute a 0-100 health score from any TelemetrySource.
fn compute_health_score<T: TelemetrySource>(source: &T) -> i32 {
    let mut score = 100i32;

    if let Some(v) = source.battery_voltage() {
        if v < 11.0 {
            score -= 30;
        } else if v < 12.5 {
            score -= 10;
        }
    }

    if let Some(t) = source.temperature() {
        if t > 75.0 {
            score -= 30;
        } else if t > 45.0 {
            score -= 10;
        }
    }

    if let Some(f) = source.fuel_remaining() {
        if f < 10.0 {
            score -= 20;
        } else if f < 20.0 {
            score -= 10;
        }
    }

    if let Some(c) = source.cpu_usage() {
        if c > 95.0 {
            score -= 15;
        } else if c > 80.0 {
            score -= 5;
        }
    }

    if let Some(p) = source.packet_loss() {
        if p > 15.0 {
            score -= 15;
        } else if p > 5.0 {
            score -= 5;
        }
    }

    score.max(0)
}

/// Return an iterator over active satellites — lazy, zero-allocation (P1 #7 — RPIT).
///
/// Instead of returning `Vec<&SatelliteHealthSummary>` (eager, allocating),
/// this returns an opaque `impl Iterator` that filters lazily.
pub fn active_satellites(
    satellites: &[SatelliteHealthSummary],
) -> impl Iterator<Item = &SatelliteHealthSummary> + '_ {
    satellites.iter().filter(|s| {
        matches!(
            s.overall_state,
            HealthState::Green | HealthState::Yellow
        )
    })
}

/// Evaluate telemetry with a user-supplied anomaly detector (P1 #10 — Fn bounds).
///
/// Accepts a closure for custom anomaly detection rules. The `Fn` bound
/// allows the closure to be called multiple times without consuming it.
pub fn evaluate_with_detector<F>(
    satellites: &[SatelliteHealthSummary],
    anomaly_detector: F,
) -> Vec<String>
where
    F: Fn(&SatelliteHealthSummary) -> Option<String>,
{
    let mut anomalies = Vec::new();
    for sat in satellites {
        if let Some(anomaly) = anomaly_detector(sat) {
            anomalies.push(anomaly);
        }
    }
    anomalies
}

/// Process a batch of health summaries with complex generic bounds (P1 #11 — where clauses).
///
/// Demonstrates `IntoIterator`, associated type bounds, and `?Sized`.
pub fn process_batch<I, F>(
    records: I,
    processor: &F,
) -> Vec<HealthState>
where
    I: IntoIterator<Item = SatelliteHealthSummary>,
    I::IntoIter: ExactSizeIterator,
    F: Fn(&SatelliteHealthSummary) -> HealthState + ?Sized,
{
    let iter = records.into_iter();
    let mut results = Vec::with_capacity(iter.len());
    for record in iter {
        results.push(processor(&record));
    }
    results
}

// =========================================================================
// Manual Future Implementation (P1 #12)
// =========================================================================
//
// Understanding Pin/Future/Poll is crucial for async Rust mastery.
// This is an educational implementation — in production you'd use async/await.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A manually implemented Future that performs a health check.
///
/// Demonstrates the mechanics behind `async`/`.await`:
/// - `Future` trait implementation
/// - `Pin<&mut Self>` for self-referential safety
/// - `Poll::Pending` vs `Poll::Ready`
/// - `Context` and `Waker`
///
/// This future yields `Pending` once (simulating an async operation),
/// then `Ready` with the computed health state.
pub struct HealthCheckFuture {
    satellite_id: String,
    score: i32,
    polled_once: bool,
}

impl HealthCheckFuture {
    /// Create a new health check future for the given satellite and score.
    pub fn new(satellite_id: String, score: i32) -> Self {
        Self {
            satellite_id,
            score,
            polled_once: false,
        }
    }
}

impl Future for HealthCheckFuture {
    type Output = (String, HealthState);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.polled_once {
            // First poll: simulate async work by yielding Pending.
            // We must wake the waker so the executor knows to poll us again.
            self.polled_once = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            // Second poll: computation is "complete".
            let state = if self.score >= 70 {
                HealthState::Green
            } else if self.score >= 40 {
                HealthState::Yellow
            } else {
                HealthState::Red
            };
            Poll::Ready((self.satellite_id.clone(), state))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_fleet() {
        let metrics = FleetHealthEngine::calculate(&[]);
        assert_eq!(metrics.availability, 100.0);
        assert_eq!(metrics.risk_score, 0.0);
        assert_eq!(metrics.healthy_count, 0);
        assert_eq!(metrics.warning_count, 0);
        assert_eq!(metrics.critical_count, 0);
    }

    #[test]
    fn test_all_green_fleet() {
        let fleet = vec![
            SatelliteHealthSummary {
                id: "SAT-0001".to_string(),
                overall_state: HealthState::Green,
                overall_score: 100,
                fuel_remaining: 90.0,
                cpu_usage: 20.0,
                memory_usage: 40.0,
                payload_active: true,
            },
            SatelliteHealthSummary {
                id: "SAT-0002".to_string(),
                overall_state: HealthState::Green,
                overall_score: 100,
                fuel_remaining: 80.0,
                cpu_usage: 30.0,
                memory_usage: 50.0,
                payload_active: true,
            },
        ];

        let metrics = FleetHealthEngine::calculate(&fleet);
        assert_eq!(metrics.availability, 100.0);
        assert_eq!(metrics.readiness, 100.0);
        assert_eq!(metrics.risk_score, 0.0);
        assert_eq!(metrics.healthy_count, 2);
        assert_eq!(metrics.warning_count, 0);
        assert_eq!(metrics.critical_count, 0);
        assert_eq!(metrics.capacity, (85.0 + 100.0) / 2.0);
    }

    #[test]
    fn test_all_red_fleet() {
        let fleet = vec![SatelliteHealthSummary {
            id: "SAT-0001".to_string(),
            overall_state: HealthState::Red,
            overall_score: 0,
            fuel_remaining: 10.0,
            cpu_usage: 95.0,
            memory_usage: 90.0,
            payload_active: false,
        }];

        let metrics = FleetHealthEngine::calculate(&fleet);
        assert_eq!(metrics.availability, 0.0);
        assert_eq!(metrics.readiness, 0.0);
        assert_eq!(metrics.risk_score, 100.0);
        assert_eq!(metrics.healthy_count, 0);
        assert_eq!(metrics.warning_count, 0);
        assert_eq!(metrics.critical_count, 1);
    }

    #[test]
    fn test_mixed_fleet() {
        let fleet = vec![
            SatelliteHealthSummary {
                id: "SAT-0001".to_string(),
                overall_state: HealthState::Green,
                overall_score: 90,
                fuel_remaining: 80.0,
                cpu_usage: 20.0,
                memory_usage: 30.0,
                payload_active: true,
            },
            SatelliteHealthSummary {
                id: "SAT-0002".to_string(),
                overall_state: HealthState::Yellow,
                overall_score: 60,
                fuel_remaining: 50.0,
                cpu_usage: 50.0,
                memory_usage: 60.0,
                payload_active: true,
            },
            SatelliteHealthSummary {
                id: "SAT-0003".to_string(),
                overall_state: HealthState::Red,
                overall_score: 30,
                fuel_remaining: 20.0,
                cpu_usage: 80.0,
                memory_usage: 90.0,
                payload_active: false,
            },
        ];

        let metrics = FleetHealthEngine::calculate(&fleet);
        assert_eq!(metrics.healthy_count, 1);
        assert_eq!(metrics.warning_count, 1);
        assert_eq!(metrics.critical_count, 1);
        // availability = (1 + 1) / 3 * 100 = 66.666...
        assert!((metrics.availability - 66.6666).abs() < 0.01);
        // readiness = 1 / 3 * 100 = 33.333...
        assert!((metrics.readiness - 33.3333).abs() < 0.01);
        // avg_score = (90 + 60 + 30)/3 = 60; risk_score = 100 - 60 = 40
        assert_eq!(metrics.risk_score, 40.0);
    }

    struct MockTelemetry {
        timestamp: chrono::DateTime<chrono::Utc>,
        battery_voltage: Option<f64>,
        temperature: Option<f64>,
        fuel_remaining: Option<f32>,
        cpu_usage: Option<f32>,
        packet_loss: Option<f32>,
    }

    impl TelemetrySource for MockTelemetry {
        fn battery_voltage(&self) -> Option<f64> { self.battery_voltage }
        fn temperature(&self) -> Option<f64> { self.temperature }
        fn fuel_remaining(&self) -> Option<f32> { self.fuel_remaining }
        fn cpu_usage(&self) -> Option<f32> { self.cpu_usage }
        fn packet_loss(&self) -> Option<f32> { self.packet_loss }
    }

    impl HasTimestamp for MockTelemetry {
        fn timestamp(&self) -> chrono::DateTime<chrono::Utc> { self.timestamp }
    }

    #[test]
    fn test_assess_health_generic() {
        let t = MockTelemetry {
            timestamp: chrono::Utc::now(),
            battery_voltage: Some(13.5),
            temperature: Some(35.0),
            fuel_remaining: Some(90.0),
            cpu_usage: Some(15.0),
            packet_loss: Some(0.0),
        };
        assert_eq!(assess_health(&t), HealthState::Green);
    }

    #[test]
    fn test_active_satellites_rpit() {
        let fleet = vec![
            SatelliteHealthSummary {
                id: "SAT-0001".to_string(),
                overall_state: HealthState::Green,
                overall_score: 90,
                fuel_remaining: 80.0,
                cpu_usage: 20.0,
                memory_usage: 30.0,
                payload_active: true,
            },
            SatelliteHealthSummary {
                id: "SAT-0002".to_string(),
                overall_state: HealthState::Red,
                overall_score: 30,
                fuel_remaining: 20.0,
                cpu_usage: 80.0,
                memory_usage: 90.0,
                payload_active: false,
            },
        ];
        let mut active = active_satellites(&fleet);
        let first = active.next().unwrap();
        assert_eq!(first.id, "SAT-0001");
        assert!(active.next().is_none());
    }

    #[test]
    fn test_evaluate_with_detector_fn_bounds() {
        let fleet = vec![
            SatelliteHealthSummary {
                id: "SAT-0001".to_string(),
                overall_state: HealthState::Green,
                overall_score: 90,
                fuel_remaining: 80.0,
                cpu_usage: 20.0,
                memory_usage: 30.0,
                payload_active: true,
            },
        ];
        let anomalies = evaluate_with_detector(&fleet, |s| {
            if s.fuel_remaining > 50.0 {
                None
            } else {
                Some(format!("Low fuel on {}", s.id))
            }
        });
        assert!(anomalies.is_empty());
    }

    #[tokio::test]
    async fn test_manual_future() {
        let fut = HealthCheckFuture::new("SAT-0001".to_string(), 85);
        let (id, state) = fut.await;
        assert_eq!(id, "SAT-0001");
        assert_eq!(state, HealthState::Green);
    }
}
