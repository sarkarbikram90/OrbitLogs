use axum::{
    body::Body,
    extract::DefaultBodyLimit,
    http::{Request, StatusCode},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use telemetry_collector::metrics;
use telemetry_collector::models::TelemetryRecord;
use telemetry_collector::server::handlers::{AppState, IngestionResponse, ErrorResponse};

/// Create a test application without real Kafka/DB dependencies.
/// This allows HTTP-layer integration testing without infrastructure.
fn create_test_app() -> Router {
    // Register metrics (idempotent in tests due to lazy_static)
    let _ = std::panic::catch_unwind(|| metrics::register_metrics());

    // We cannot create a real KafkaProducer without a broker,
    // so we test only the routes that don't require Kafka (health, readiness, liveness, metrics).
    // For full ingestion tests, we validate the TelemetryRecord parsing and validation.
    let state = Arc::new(AppState {
        kafka_producer: unsafe { std::mem::zeroed() }, // Will not be called in health tests
        raw_topic: "telemetry.raw".to_string(),
        invalid_topic: "telemetry.invalid".to_string(),
        db_pool: None,
    });

    Router::new()
        .route("/health", get(telemetry_collector::server::handlers::health_check))
        .route("/readiness", get(telemetry_collector::server::handlers::readiness_check))
        .route("/liveness", get(telemetry_collector::server::handlers::liveness_check))
        .route("/metrics", get(telemetry_collector::server::handlers::metrics_endpoint))
        .layer(DefaultBodyLimit::max(10_240))
        .with_state(state)
}

// ============================================================
// Validation Tests (TelemetryRecord)
// ============================================================

#[test]
fn test_e2e_telemetry_flow_validation() {
    let raw_payload = TelemetryRecord {
        satellite_id: "SAT-TEST-99".to_string(),
        timestamp: Utc::now(),
        battery_voltage: 14.2,
        battery_current: 1.2,
        solar_panel_output: 120.0,
        cpu_usage: 12.5,
        memory_usage: 25.1,
        temperature: 24.5,
        orientation_pitch: 0.1,
        orientation_roll: -0.2,
        orientation_yaw: 0.0,
        velocity: 7.67,
        altitude: 521.2,
        fuel_remaining: 98.4,
        communication_latency: 22.1,
        packet_loss: 0.0,
        signal_strength: -60.0,
        event_id: Uuid::new_v4(),
    };

    // 1. Verify schema constraints are met
    assert!(
        raw_payload.validate().is_ok(),
        "Telemetry validation must pass for realistic mock data"
    );

    // 2. Verify invalid ranges are rejected
    let mut invalid_payload = raw_payload.clone();
    invalid_payload.battery_voltage = 150.0; // Above limit
    assert!(
        invalid_payload.validate().is_err(),
        "Telemetry validation must fail for voltages exceeding bounds"
    );
}

#[test]
fn test_validation_multiple_errors_collected() {
    let rec = TelemetryRecord {
        satellite_id: "".to_string(),
        timestamp: Utc::now(),
        battery_voltage: -5.0,
        battery_current: 0.0,
        solar_panel_output: 0.0,
        cpu_usage: 150.0,
        memory_usage: -10.0,
        temperature: 0.0,
        orientation_pitch: 0.0,
        orientation_roll: 0.0,
        orientation_yaw: 0.0,
        velocity: 0.0,
        altitude: 0.0,
        fuel_remaining: 200.0,
        communication_latency: 0.0,
        packet_loss: 0.0,
        signal_strength: 0.0,
        event_id: Uuid::new_v4(),
    };

    let result = rec.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    // Should have errors for: satellite_id, battery_voltage, cpu_usage, memory_usage, fuel_remaining
    assert!(
        errors.len() >= 5,
        "Expected at least 5 validation errors, got {}: {:?}",
        errors.len(),
        errors
    );
}

#[test]
fn test_serde_deny_unknown_fields_integration() {
    let json_with_extra = r#"{
        "satellite_id": "SAT-0001",
        "timestamp": "2026-01-01T00:00:00Z",
        "battery_voltage": 13.8,
        "battery_current": 2.1,
        "solar_panel_output": 110.0,
        "cpu_usage": 25.4,
        "memory_usage": 45.1,
        "temperature": 28.5,
        "orientation_pitch": 1.2,
        "orientation_roll": -2.3,
        "orientation_yaw": 0.5,
        "velocity": 7.67,
        "altitude": 520.4,
        "fuel_remaining": 92.5,
        "communication_latency": 35.1,
        "packet_loss": 0.1,
        "signal_strength": -68.0,
        "event_id": "550e8400-e29b-41d4-a716-446655440000",
        "malicious_field": "should_be_rejected"
    }"#;

    let result: Result<TelemetryRecord, _> = serde_json::from_str(json_with_extra);
    assert!(
        result.is_err(),
        "Deserialization must reject unknown fields"
    );
}

// ============================================================
// HTTP Endpoint Integration Tests
// ============================================================

#[tokio::test]
async fn test_liveness_endpoint() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/liveness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Without DB configured, health returns UP (database shows NOT_CONFIGURED)
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_readiness_endpoint_without_db() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Without DB, readiness should report not ready
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
