use std::net::SocketAddr;

use axum::{routing::get, Router};
use prometheus::{Encoder, TextEncoder};
use tracing::{error, info};

use crate::metrics::REGISTRY;

pub async fn start_metrics_server(port: u16) {
    let app = Router::new()
        .route(
            "/metrics",
            get(|| async {
                let encoder = TextEncoder::new();
                let metric_families = REGISTRY.gather();
                let mut buffer = Vec::new();
                encoder.encode(&metric_families, &mut buffer).unwrap();

                axum::response::Response::builder()
                    .status(axum::http::StatusCode::OK)
                    .header(axum::http::header::CONTENT_TYPE, encoder.format_type())
                    .body(axum::body::Body::from(buffer))
                    .unwrap()
            }),
        )
        .route(
            "/health",
            get(|| async {
                axum::response::Response::builder()
                    .status(axum::http::StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"status":"UP"}"#))
                    .unwrap()
            }),
        )
        .route(
            "/readiness",
            get(|| async {
                axum::response::Response::builder()
                    .status(axum::http::StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"ready":true}"#))
                    .unwrap()
            }),
        )
        .route(
            "/liveness",
            get(|| async {
                axum::response::Response::builder()
                    .status(axum::http::StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"alive":true}"#))
                    .unwrap()
            }),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(
                "Failed to bind Space OTel Collector metrics server to port {}: {}",
                port, e
            );
            return;
        }
    };
    info!(
        "Space OTel Collector metrics server running on http://localhost:{}/metrics",
        port
    );
    if let Err(e) = axum::serve(listener, app).await {
        error!("Metrics server error: {}", e);
    }
}
