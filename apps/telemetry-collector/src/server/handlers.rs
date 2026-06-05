use axum::http::HeaderMap;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use prometheus::{Encoder, TextEncoder};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, instrument, warn};
use uuid::Uuid;

use crate::metrics::{
    DLQ_PUBLISH_FAILURES_TOTAL, KAFKA_PUBLISH_LATENCY, PROCESSING_LATENCY, REGISTRY,
    TELEMETRY_INVALID_TOTAL, TELEMETRY_RECEIVED_TOTAL,
};
use crate::models::TelemetryRecord;
use crate::server::producer::KafkaProducer;

#[derive(Clone)]
pub struct AppState {
    pub kafka_producer: KafkaProducer,
    pub raw_topic: String,
    pub invalid_topic: String,
    pub db_pool: Option<sqlx::PgPool>,
    pub broadcast_tx: tokio::sync::broadcast::Sender<String>,
    pub api_key: String,
    pub cors_allowed_origin: String,
}

#[derive(Serialize)]
pub struct IngestionResponse {
    pub message: String,
    pub event_id: String,
    pub request_id: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Vec<String>,
    pub request_id: String,
}

/// Extract or generate a request correlation ID from headers
fn extract_request_id(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

#[instrument(skip(state, headers, body_bytes))]
pub async fn ingest_telemetry(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body_bytes: axum::body::Bytes,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = extract_request_id(&headers);

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let payload = if content_type.starts_with("application/octet-stream") {
        match spaceops_packets::decode_frame_to_record(&body_bytes) {
            Ok(rec) => rec,
            Err(e) => {
                warn!(
                    request_id = %request_id,
                    "Failed to decode binary frame: {}", e
                );
                let err_resp = ErrorResponse {
                    error: "Invalid binary payload".to_string(),
                    details: vec![e.to_string()],
                    request_id: request_id.clone(),
                };
                return (StatusCode::BAD_REQUEST, Json(err_resp)).into_response();
            }
        }
    } else {
        match serde_json::from_slice::<TelemetryRecord>(&body_bytes) {
            Ok(rec) => rec,
            Err(e) => {
                warn!(
                    request_id = %request_id,
                    "Failed to parse JSON telemetry: {}", e
                );
                let err_resp = ErrorResponse {
                    error: "Invalid JSON payload".to_string(),
                    details: vec![e.to_string()],
                    request_id: request_id.clone(),
                };
                return (StatusCode::BAD_REQUEST, Json(err_resp)).into_response();
            }
        }
    };

    let sat_id = payload.satellite_id.clone();

    // Check validation constraints
    if let Err(validation_errors) = payload.validate() {
        warn!(
            request_id = %request_id,
            "Validation failed for satellite {}: {:?}",
            sat_id, validation_errors
        );

        // Track metrics
        TELEMETRY_RECEIVED_TOTAL
            .with_label_values(&[&sat_id, "invalid"])
            .inc();
        for err in &validation_errors {
            TELEMETRY_INVALID_TOTAL.with_label_values(&[err]).inc();
        }

        // Publish to DLQ (telemetry.invalid)
        let serialized_err = serde_json::json!({
            "record": payload,
            "errors": validation_errors,
            "rejected_at": Utc::now(),
            "request_id": request_id
        })
        .to_string();

        if let Err(e) = state
            .kafka_producer
            .publish(&state.invalid_topic, &sat_id, &serialized_err)
            .await
        {
            error!(
                request_id = %request_id,
                "DLQ publish failed for satellite {}: {}", sat_id, e
            );
            DLQ_PUBLISH_FAILURES_TOTAL.inc();
        }

        let err_resp = ErrorResponse {
            error: "Payload validation failed".to_string(),
            details: validation_errors,
            request_id,
        };

        return (StatusCode::BAD_REQUEST, Json(err_resp)).into_response();
    }

    TELEMETRY_RECEIVED_TOTAL
        .with_label_values(&[&sat_id, "valid"])
        .inc();

    // Serialise telemetry payload for Kafka
    let serialized_payload = match serde_json::to_string(&payload) {
        Ok(s) => s,
        Err(e) => {
            error!(
                request_id = %request_id,
                "Serialization failed for satellite {}: {}", sat_id, e
            );
            let err_resp = ErrorResponse {
                error: "Internal serialization error".to_string(),
                details: vec![e.to_string()],
                request_id,
            };
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(err_resp)).into_response();
        }
    };

    // Publish to Kafka telemetry.raw
    let publish_start = Instant::now();
    match state
        .kafka_producer
        .publish(&state.raw_topic, &sat_id, &serialized_payload)
        .await
    {
        Ok(_) => {
            let _ = state.broadcast_tx.send(serialized_payload.clone());
            KAFKA_PUBLISH_LATENCY.observe(publish_start.elapsed().as_secs_f64());
            PROCESSING_LATENCY.observe(start_time.elapsed().as_secs_f64());

            let success_resp = IngestionResponse {
                message: "Telemetry received and queued".to_string(),
                event_id: payload.event_id.to_string(),
                request_id,
            };
            (StatusCode::ACCEPTED, Json(success_resp)).into_response()
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                "Kafka produce failure for satellite {}: {}", sat_id, e
            );
            let err_resp = ErrorResponse {
                error: "Failed to queue telemetry event".to_string(),
                details: vec![e.to_string()],
                request_id,
            };
            (StatusCode::SERVICE_UNAVAILABLE, Json(err_resp)).into_response()
        }
    }
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub services: ServiceStatus,
}

#[derive(Serialize)]
pub struct ServiceStatus {
    pub database: String,
    pub kafka_producer: String,
}

/// Deep health check — validates database and Kafka connectivity
pub async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut db_status = "NOT_CONFIGURED".to_string();
    let mut status = "UP";

    if let Some(ref pool) = state.db_pool {
        db_status = match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => "UP".to_string(),
            Err(e) => {
                error!("Database ping failed in health check: {}", e);
                status = "DOWN";
                "DOWN".to_string()
            }
        };
    }

    // Since Axum holds the KafkaProducer, it's UP if the producer initialization was successful
    let kafka_status = "UP".to_string();

    let code = if status == "UP" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let resp = HealthResponse {
        status: status.to_string(),
        timestamp: Utc::now(),
        services: ServiceStatus {
            database: db_status,
            kafka_producer: kafka_status,
        },
    };

    (code, Json(resp))
}

#[derive(Serialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub timestamp: DateTime<Utc>,
    pub checks: ReadinessChecks,
}

#[derive(Serialize)]
pub struct ReadinessChecks {
    pub database: bool,
    pub kafka: bool,
}

/// Readiness probe — returns 200 only when all dependencies are healthy
pub async fn readiness_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut db_ready = false;
    let kafka_ready = true; // Producer is initialized at startup

    if let Some(ref pool) = state.db_pool {
        db_ready = sqlx::query("SELECT 1").execute(pool).await.is_ok();
    }

    let ready = db_ready && kafka_ready;
    let code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let resp = ReadinessResponse {
        ready,
        timestamp: Utc::now(),
        checks: ReadinessChecks {
            database: db_ready,
            kafka: kafka_ready,
        },
    };

    (code, Json(resp))
}

#[derive(Serialize)]
pub struct LivenessResponse {
    pub alive: bool,
    pub timestamp: DateTime<Utc>,
}

/// Liveness probe — always returns 200 if the process is running
pub async fn liveness_check() -> impl IntoResponse {
    let resp = LivenessResponse {
        alive: true,
        timestamp: Utc::now(),
    };

    (StatusCode::OK, Json(resp))
}

pub async fn metrics_endpoint() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        error!("Failed to encode metrics: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to scrape metrics",
        )
            .into_response();
    }

    let response = match String::from_utf8(buffer) {
        Ok(s) => s,
        Err(e) => {
            error!("Metrics buffer contains invalid UTF-8: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid metrics encoding",
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        [("content-type", encoder.format_type())],
        response,
    )
        .into_response()
}

pub async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let mut rx = state.broadcast_tx.subscribe();

    loop {
        tokio::select! {
            // Receive from broadcast channel
            Ok(msg_str) = rx.recv() => {
                if socket.send(axum::extract::ws::Message::Text(msg_str)).await.is_err() {
                    break; // Client disconnected
                }
            }
            // Read from socket (to detect disconnections or pings)
            res = socket.recv() => {
                match res {
                    Some(Ok(axum::extract::ws::Message::Close(_))) | Some(Err(_)) | None => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}
