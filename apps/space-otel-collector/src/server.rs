// Copyright 2026 OrbitLogs Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
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
