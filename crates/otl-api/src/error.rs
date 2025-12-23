//! API error handling
//!
//! Author: hephaex@gmail.com

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// API error response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// Error code
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn not_found(resource: &str) -> Self {
        Self::new("NOT_FOUND", format!("{resource} not found"))
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("BAD_REQUEST", message)
    }

    pub fn unauthorized() -> Self {
        Self::new("UNAUTHORIZED", "Authentication required")
    }

    pub fn forbidden() -> Self {
        Self::new("FORBIDDEN", "Access denied")
    }

    pub fn internal_error() -> Self {
        Self::new("INTERNAL_ERROR", "Internal server error")
    }
}

/// Application error type
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized,
    Forbidden(String),
    Internal(String),
    Database(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, ApiError::not_found(&msg)),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, ApiError::bad_request(msg)),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, ApiError::unauthorized()),
            AppError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                ApiError::new("FORBIDDEN", msg)
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::internal_error().with_details(msg),
            ),
            AppError::Database(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::new("DATABASE_ERROR", "Database operation failed").with_details(msg),
            ),
        };

        (status, Json(error)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<otl_core::OtlError> for AppError {
    fn from(err: otl_core::OtlError) -> Self {
        use otl_core::OtlError;

        match err {
            OtlError::NotFound(msg) => AppError::NotFound(msg),
            OtlError::AccessDenied { reason } => AppError::Forbidden(reason),
            OtlError::InvalidOntology(msg) => AppError::BadRequest(format!("Invalid ontology: {msg}")),
            OtlError::ValidationError(msg) => AppError::BadRequest(msg),
            OtlError::DatabaseError(msg) => AppError::Database(msg),
            OtlError::SearchError(msg) => AppError::Internal(format!("Search error: {msg}")),
            OtlError::LlmError(msg) => AppError::Internal(format!("LLM error: {msg}")),
            OtlError::ConfigError(msg) => AppError::Internal(format!("Configuration error: {msg}")),
            OtlError::Other(err) => AppError::Internal(err.to_string()),
        }
    }
}
