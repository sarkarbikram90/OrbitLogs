

use axum::{
    body::Body,
    extract::DefaultBodyLimit,
    http::{Request, StatusCode},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use telemetry_collector::metrics;
use telemetry_collector::models::TelemetryRecord;
use telemetry_collector::server::handlers::AppState;

async fn mock_ingest_telemetry(
    _headers: axum::http::HeaderMap,
    _body: String,
) -> impl axum::response::IntoResponse {
    StatusCode::ACCEPTED
}

/// Create a test application without real Kafka/DB dependencies.
/// This allows HTTP-layer integration testing without infrastructure.
fn create_test_app() -> Router {
    // Register metrics (idempotent in tests due to lazy_static)
    let _ = std::panic::catch_unwind(metrics::register_metrics);

    let kafka_producer = telemetry_collector::server::producer::KafkaProducer::new(
        "localhost:9092",
        None,
        None,
        None,
        None,
        None,
    )
    .expect("failed to create mock kafka producer for test");

    let (broadcast_tx, _) = tokio::sync::broadcast::channel(100);

    let state = Arc::new(AppState {
        kafka_producer,
        raw_topic: "telemetry.raw".to_string(),
        invalid_topic: "telemetry.invalid".to_string(),
        db_pool: None,
        broadcast_tx,
        api_key: "secret-orbitlogs-key-2026".to_string(),
        cors_allowed_origin: "http://localhost:3000".to_string(),
        rate_limiter: std::sync::Arc::new(dashmap::DashMap::new()),
    });

    Router::new()
        .route(
            "/telemetry",
            post(mock_ingest_telemetry).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                telemetry_collector::server::rate_limit_middleware,
            )),
        )
        .route(
            "/telemetry_real",
            post(telemetry_collector::server::handlers::ingest_telemetry),
        )
        .nest(
            "/telemetry_authed",
            Router::new()
                .route(
                    "/",
                    post(telemetry_collector::server::handlers::ingest_telemetry),
                )
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    telemetry_collector::server::auth_middleware,
                )),
        )
        .route(
            "/health",
            get(telemetry_collector::server::handlers::health_check),
        )
        .route(
            "/readiness",
            get(telemetry_collector::server::handlers::readiness_check),
        )
        .route(
            "/liveness",
            get(telemetry_collector::server::handlers::liveness_check),
        )
        .route(
            "/metrics",
            get(telemetry_collector::server::handlers::metrics_endpoint),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            telemetry_collector::server::cors_middleware,
        ))
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
        subsystem_id: "unified".to_string(),
        mission_id: "unknown".to_string(),
        timestamp: Utc::now(),
        battery_voltage: Some(14.2),
        battery_current: Some(1.2),
        solar_panel_output: Some(120.0),
        cpu_usage: Some(12.5),
        memory_usage: Some(25.1),
        temperature: Some(24.5),
        orientation_pitch: Some(0.1),
        orientation_roll: Some(-0.2),
        orientation_yaw: Some(0.0),
        velocity: Some(7.67),
        altitude: Some(521.2),
        fuel_remaining: Some(98.4),
        communication_latency: Some(22.1),
        packet_loss: Some(0.0),
        signal_strength: Some(-60.0),
        payload_power_draw: Some(15.0),
        payload_active: Some(true),
        event_id: Uuid::new_v4(),
    };

    // 1. Verify schema constraints are met
    assert!(
        raw_payload.validate().is_ok(),
        "Telemetry validation must pass for realistic mock data"
    );

    // 2. Verify invalid ranges are rejected
    let mut invalid_payload = raw_payload.clone();
    invalid_payload.battery_voltage = Some(150.0); // Above limit
    assert!(
        invalid_payload.validate().is_err(),
        "Telemetry validation must fail for voltages exceeding bounds"
    );
}

#[test]
fn test_validation_multiple_errors_collected() {
    let rec = TelemetryRecord {
        satellite_id: "".to_string(),
        subsystem_id: "unified".to_string(),
        mission_id: "unknown".to_string(),
        timestamp: Utc::now(),
        battery_voltage: Some(-5.0),
        battery_current: Some(0.0),
        solar_panel_output: Some(0.0),
        cpu_usage: Some(150.0),
        memory_usage: Some(-10.0),
        temperature: Some(0.0),
        orientation_pitch: Some(0.0),
        orientation_roll: Some(0.0),
        orientation_yaw: Some(0.0),
        velocity: Some(0.0),
        altitude: Some(0.0),
        fuel_remaining: Some(200.0),
        communication_latency: Some(0.0),
        packet_loss: Some(0.0),
        signal_strength: Some(0.0),
        payload_power_draw: Some(0.0),
        payload_active: Some(false),
        event_id: Uuid::new_v4(),
    };

    let result = rec.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    // Should have errors for: satellite_id, battery_voltage, cpu_usage, memory_usage, fuel_remaining
    assert!(
        errors.len() >= 5,
        "Expected at least 5 validation errors, received {}: {:?}",
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

#[tokio::test]
async fn test_oversized_payload_rejection() {
    let app = create_test_app();

    // Create an oversized body (11 KB)
    let body = vec![b'a'; 11 * 1024];

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/telemetry")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_rate_limiting() {
    let app = create_test_app();

    // Send 150 requests concurrently.
    // Since the limit is 100 requests per second, some of these should get a 429 status code.
    let mut set = tokio::task::JoinSet::new();

    for _ in 0..150 {
        let mut app = app.clone();
        set.spawn(async move {
            use tower::Service;
            app.call(
                Request::builder()
                    .method("POST")
                    .uri("/telemetry")
                    .header("Content-Type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
        });
    }

    let mut hit_rate_limit = false;
    while let Some(res) = set.join_next().await {
        let response = res.unwrap().unwrap();
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            hit_rate_limit = true;
        }
    }

    assert!(
        hit_rate_limit,
        "Expected at least one request to be rate limited (429)"
    );
}

#[tokio::test]
async fn test_real_handler_binary_invalid_crc() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/telemetry_real")
                .header("Content-Type", "application/octet-stream")
                .body(Body::from(vec![0x00; 10]))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_real_handler_binary_valid() {
    let app = create_test_app();

    let rec = TelemetryRecord {
        satellite_id: "SAT-0001".to_string(),
        subsystem_id: "power".to_string(),
        mission_id: "unknown".to_string(),
        timestamp: Utc::now(),
        battery_voltage: Some(13.8),
        battery_current: Some(2.1),
        solar_panel_output: Some(110.0),
        cpu_usage: None,
        memory_usage: None,
        temperature: None,
        orientation_pitch: None,
        orientation_roll: None,
        orientation_yaw: None,
        velocity: None,
        altitude: None,
        fuel_remaining: None,
        communication_latency: None,
        packet_loss: None,
        signal_strength: None,
        payload_power_draw: None,
        payload_active: None,
        event_id: Uuid::new_v4(),
    };

    let binary_data = orbitlogs_packets::encode_record_to_frame(&rec, 1).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/telemetry_real")
                .header("Content-Type", "application/octet-stream")
                .body(Body::from(binary_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // The handler will try to publish to Kafka. Since Kafka is not running, it will fail to publish
    // and return 503 (SERVICE_UNAVAILABLE). But this proves that the request passed parsing,
    // decoding, and validation successfully!
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_auth_middleware_rejected_missing() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/telemetry_authed")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_rejected_invalid() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/telemetry_authed")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer invalid-key-12345")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_accepted_header() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/telemetry_authed")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer secret-orbitlogs-key-2026")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    // With correct auth, it proceeds to the inner handler which parses the empty JSON,
    // fails parsing with a 400 Bad Request (since empty JSON is invalid TelemetryRecord).
    // This proves that it bypassed the auth middleware!
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_auth_middleware_accepted_query_param() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/telemetry_authed?token=secret-orbitlogs-key-2026")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cors_middleware_allowed_origin() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/telemetry_real")
                .header("Origin", "http://localhost:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .unwrap()
            .to_str()
            .unwrap(),
        "http://localhost:3000"
    );
}
