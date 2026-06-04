use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

pub mod handlers;
pub mod producer;

use handlers::AppState;

pub async fn start_server(port: u16, state: AppState) -> Result<(), anyhow::Error> {
    let state_arc = Arc::new(state);

    let app = Router::new()
        .route("/telemetry", post(handlers::ingest_telemetry))
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
