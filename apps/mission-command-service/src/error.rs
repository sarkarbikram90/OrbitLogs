use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use orbitlogs_models::ErrorCode;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Kafka error: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Command {0} not found")]
    NotFound(uuid::Uuid),

    #[error("Invalid state transition: {from} -> {to}")]
    InvalidTransition {
        from: orbitlogs_models::CommandStatus,
        to: orbitlogs_models::CommandStatus,
    },

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Too many login attempts. Please try again later.")]
    TooManyRequests,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Domain(#[from] orbitlogs_models::DomainError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::InternalError,
                self.to_string(),
            ),
            AppError::Kafka(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::InternalError,
                self.to_string(),
            ),
            AppError::Auth(msg) => (
                StatusCode::UNAUTHORIZED,
                ErrorCode::Unauthorized,
                msg.clone(),
            ),
            AppError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                ErrorCode::Forbidden,
                msg.clone(),
            ),
            AppError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                ErrorCode::NotFound,
                self.to_string(),
            ),
            AppError::InvalidTransition { .. } => (
                StatusCode::BAD_REQUEST,
                ErrorCode::CommandInvalidState,
                self.to_string(),
            ),
            AppError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorCode::ValidationFailed,
                msg.clone(),
            ),
            AppError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::RateLimited,
                self.to_string(),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::InternalError,
                msg.clone(),
            ),
            AppError::Domain(e) => match e {
                orbitlogs_models::DomainError::NotFound { .. } => (
                    StatusCode::NOT_FOUND,
                    ErrorCode::NotFound,
                    e.to_string(),
                ),
                orbitlogs_models::DomainError::Validation(_)
                | orbitlogs_models::DomainError::InvalidSatelliteId(_)
                | orbitlogs_models::DomainError::InvalidStationId(_)
                | orbitlogs_models::DomainError::InvalidMissionId(_) => (
                    StatusCode::BAD_REQUEST,
                    ErrorCode::ValidationFailed,
                    e.to_string(),
                ),
            },
        };

        let body = Json(serde_json::json!({
            "code": code.to_string(),
            "message": message,
        }));

        (status, body).into_response()
    }
}

