use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::info;

pub mod handlers;
pub mod producer;

use handlers::AppState;

pub async fn start_server(port: u16, state: AppState) -> Result<(), anyhow::Error> {
    let state_arc = Arc::new(state);

    let app = Router::new()
        .nest(
            "/telemetry",
            Router::new()
                .route(
                    "/",
                    post(handlers::ingest_telemetry).layer(axum::middleware::from_fn_with_state(
                        state_arc.clone(),
                        rate_limit_middleware,
                    )),
                )
                .route(
                    "/tles/ingest",
                    post(handlers::ingest_tles),
                )
                .layer(axum::middleware::from_fn_with_state(
                    state_arc.clone(),
                    auth_middleware,
                )),
        )
        .route("/ws", get(handlers::ws_handler))
        .route("/health", get(handlers::health_check))
        .route("/readiness", get(handlers::readiness_check))
        .route("/liveness", get(handlers::liveness_check))
        .route("/metrics", get(handlers::metrics_endpoint))
        .layer(axum::middleware::from_fn_with_state(
            state_arc.clone(),
            cors_middleware,
        ))
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

pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, std::convert::Infallible> {
    // Extract client identifier (token or IP address)
    let client_id = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Fallback to query param token
            if let Some(query) = req.uri().query() {
                for pair in query.split('&') {
                    let mut parts = pair.splitn(2, '=');
                    if let (Some("token"), Some(val)) = (parts.next(), parts.next()) {
                        return val.to_string();
                    }
                }
            }
            "anonymous".to_string()
        });

    let now = Instant::now();
    let limit_exceeded = {
        let entry = state.rate_limiter.entry(client_id).or_insert_with(|| {
            std::sync::Mutex::new(crate::server::handlers::TokenBucket {
                tokens: 100.0,
                last_refill: now,
            })
        });
        let mut bucket = entry.lock().unwrap();

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * 100.0).min(100.0);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            false
        } else {
            true
        }
    };

    if limit_exceeded {
        let body = serde_json::json!({
            "error": "Rate limit exceeded",
            "details": vec!["Per-client rate limit of 100 requests per second exceeded".to_string()]
        })
        .to_string();
        let mut response = axum::response::Response::new(axum::body::Body::from(body));
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        if let Ok(origin_val) = axum::http::HeaderValue::from_str(&state.cors_allowed_origin) {
            response
                .headers_mut()
                .insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
        }
        Ok(response)
    } else {
        Ok(next.run(req).await)
    }
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, std::convert::Infallible> {
    let mut token_opt = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    if token_opt.is_none() {
        if let Some(query) = req.uri().query() {
            for pair in query.split('&') {
                let mut parts = pair.splitn(2, '=');
                if let (Some("token"), Some(val)) = (parts.next(), parts.next()) {
                    token_opt = Some(val.to_string());
                    break;
                }
            }
        }
    }

    let authenticated = match token_opt {
        Some(ref t) => orbitlogs_models::auth::secure_compare(t, &state.api_key),
        None => false,
    };

    if authenticated {
        Ok(next.run(req).await)
    } else {
        let body = serde_json::json!({
            "error": "Unauthorized",
            "details": vec!["Missing or invalid authorization token"]
        })
        .to_string();
        let mut response = axum::response::Response::new(axum::body::Body::from(body));
        *response.status_mut() = StatusCode::UNAUTHORIZED;
        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        if let Ok(origin_val) = axum::http::HeaderValue::from_str(&state.cors_allowed_origin) {
            response
                .headers_mut()
                .insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
        }
        Ok(response)
    }
}

pub async fn cors_middleware(
    State(state): State<Arc<AppState>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = request.method().clone();
    let allowed_origin = state.cors_allowed_origin.clone();

    if method == axum::http::Method::OPTIONS {
        return axum::response::Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("access-control-allow-origin", &allowed_origin)
            .header("access-control-allow-methods", "GET, POST, OPTIONS")
            .header(
                "access-control-allow-headers",
                "content-type, x-request-id, authorization",
            )
            .body(axum::body::Body::empty())
            .unwrap();
    }

    let mut response = next.run(request).await;
    if let Ok(origin_val) = axum::http::HeaderValue::from_str(&allowed_origin) {
        response
            .headers_mut()
            .insert(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
    }
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_METHODS,
        axum::http::HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    response.headers_mut().insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS,
        axum::http::HeaderValue::from_static("content-type, x-request-id, authorization"),
    );
    response
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
