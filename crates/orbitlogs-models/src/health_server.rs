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

use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct HealthStateHandle {
    ready: Arc<AtomicBool>,
}

impl HealthStateHandle {
    pub fn new() -> Self {
        Self {
            ready: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn set_ready(&self, ready: bool) {
        self.ready.store(ready, Ordering::SeqCst);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }
}

impl Default for HealthStateHandle {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HealthServer {
    handle: HealthStateHandle,
}

impl HealthServer {
    pub fn new() -> Self {
        Self {
            handle: HealthStateHandle::new(),
        }
    }

    pub fn handle(&self) -> HealthStateHandle {
        self.handle.clone()
    }

    pub async fn run(&self, port: u16) -> Result<(), anyhow::Error> {
        let handle = self.handle.clone();

        let app = Router::new()
            .route(
                "/healthz",
                get(|| async {
                    (StatusCode::OK, Json(serde_json::json!({ "status": "UP" }))).into_response()
                }),
            )
            .route(
                "/readyz",
                get(move || {
                    let handle = handle.clone();
                    async move {
                        if handle.is_ready() {
                            (StatusCode::OK, Json(serde_json::json!({ "status": "READY" }))).into_response()
                        } else {
                            (
                                StatusCode::SERVICE_UNAVAILABLE,
                                Json(serde_json::json!({ "status": "NOT_READY" })),
                            )
                                .into_response()
                        }
                    }
                }),
            );

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        info!("Kubernetes health probe server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

impl Default for HealthServer {
    fn default() -> Self {
        Self::new()
    }
}
