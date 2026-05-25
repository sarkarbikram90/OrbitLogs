use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use prometheus::{Encoder, TextEncoder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, instrument, warn};

use crate::metrics::{
    DATABASE_INSERT_LATENCY, KAFKA_PUBLISH_LATENCY, PROCESSING_LATENCY, REGISTRY,
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
}

#[derive(Serialize)]
pub struct IngestionResponse {
    pub message: String,
    pub event_id: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Vec<String>,
}

#[instrument(skip(state, payload))]
pub async fn ingest_telemetry(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TelemetryRecord>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let sat_id = payload.satellite_id.clone();

    // Check validation constraints
    if let Err(validation_errors) = payload.validate() {
        warn!(
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
            "rejected_at": Utc::now()
        })
        .to_string();

        let _ = state
            .kafka_producer
            .publish(&state.invalid_topic, &sat_id, &serialized_err)
            .await;

        let err_resp = ErrorResponse {
            error: "Payload validation failed".to_string(),
            details: validation_errors,
        };

        return (StatusCode::BAD_REQUEST, Json(err_resp)).into_response();
    }

    TELEMETRY_RECEIVED_TOTAL
        .with_label_values(&[&sat_id, "valid"])
        .inc();

    // Serialise telemetry payload
    let serialized_payload = match serde_json::to_string(&payload) {
        Ok(s) => s,
        Err(e) => {
            error!("Serialization failed for satellite {}: {}", sat_id, e);
            let err_resp = ErrorResponse {
                error: "Internal serialization error".to_string(),
                details: vec![e.to_string()],
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
            KAFKA_PUBLISH_LATENCY.observe(publish_start.elapsed().as_secs_f64());
            PROCESSING_LATENCY.observe(start_time.elapsed().as_secs_f64());

            let success_resp = IngestionResponse {
                message: "Telemetry received and queued".to_string(),
                event_id: payload.event_id.to_string(),
            };
            (StatusCode::ACCEPTED, Json(success_resp)).into_response()
        }
        Err(e) => {
            error!("Kafka produce failure for satellite {}: {}", sat_id, e);
            let err_resp = ErrorResponse {
                error: "Failed to queue telemetry event".to_string(),
                details: vec![e.to_string()],
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
