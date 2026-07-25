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

//! Structured error types for the Digital Twin Engine.
//!
//! Every handler returns `Result<T, AppError>`, and `AppError` implements
//! `IntoResponse` so Axum can serialise it as a consistent JSON envelope:
//!
//! ```json
//! {
//!   "error_code": "ERR_NOT_FOUND",
//!   "message":    "Satellite 'SAT-XXXX' has no active digital twin",
//!   "details":    []
//! }
//! ```

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Unified error type for all Digital Twin Engine handlers.
#[derive(Debug)]
pub enum AppError {
    /// The requested satellite twin does not exist in the in-memory store.
    NotFound { entity: &'static str, id: String },

    /// The request failed authentication (missing or invalid bearer token).
    Unauthorized { detail: String },

    /// The request body or parameters failed validation.
    InvalidInput { field: String, reason: String },

    /// An internal database error occurred.
    Database(sqlx::Error),

    /// A catch-all for unexpected internal errors.
    Internal(anyhow::Error),
}

/// Wire-format for all error responses.
#[derive(Serialize)]
struct ErrorBody {
    error_code: String,
    message: String,
    details: Vec<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message, details) = match self {
            AppError::NotFound { entity, id } => (
                StatusCode::NOT_FOUND,
                "ERR_NOT_FOUND",
                format!("{entity} '{id}' not found"),
                vec![],
            ),
            AppError::Unauthorized { detail } => (
                StatusCode::UNAUTHORIZED,
                "ERR_UNAUTHORIZED",
                "Authentication required".to_string(),
                vec![detail],
            ),
            AppError::InvalidInput { field, reason } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "ERR_VALIDATION_FAILED",
                format!("Invalid input for field '{field}'"),
                vec![reason],
            ),
            AppError::Database(e) => {
                tracing::error!(error = %e, "Database error in digital twin engine");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "ERR_INTERNAL_ERROR",
                    "An internal database error occurred".to_string(),
                    vec![],
                )
            }
            AppError::Internal(e) => {
                tracing::error!(error = %e, "Internal error in digital twin engine");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "ERR_INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                    vec![],
                )
            }
        };

        let body = ErrorBody {
            error_code: error_code.to_string(),
            message,
            details,
        };

        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e)
    }
}
