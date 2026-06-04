use axum::{
    error_handling::HandleErrorLayer,
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post},
    BoxError, Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::info;

pub mod handlers;
pub mod producer;

use handlers::AppState;

async fn handle_rate_limit_error(err: BoxError) -> (StatusCode, String) {
    (
        StatusCode::TOO_MANY_REQUESTS,
        serde_json::json!({
            "error": "Rate limit exceeded",
            "details": vec![err.to_string()]
        })
        .to_string(),
    )
}

pub async fn start_server(port: u16, state: AppState) -> Result<(), anyhow::Error> {
    let state_arc = Arc::new(state);

    let rate_limit_layer = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(handle_rate_limit_error))
        .buffer(100)
        .rate_limit(100, Duration::from_secs(1));

    let app = Router::new()
        .route(
            "/telemetry",
            post(handlers::ingest_telemetry).layer(rate_limit_layer),
        )
        .route("/health", get(handlers::health_check))
        .route("/readiness", get(handlers::readiness_check))
        .route("/liveness", get(handlers::liveness_check))
        .route("/metrics", get(handlers::metrics_endpoint))
        .layer(DefaultBodyLimit::max(10_240)) // 10KB max payload
        .with_state(state_arc);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    info!("HTTP Ingestion Server running on http://localhost:{}", port);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("HTTP Ingestion Server shut down gracefully.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
