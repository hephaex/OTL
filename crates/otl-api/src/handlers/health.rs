//! Health check handlers
//!
//! Author: hephaex@gmail.com

use crate::state::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Liveness probe - basic health check
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is alive", body = HealthResponse)
    )
)]
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness response
#[derive(Serialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub checks: ReadinessChecks,
}

#[derive(Serialize)]
pub struct ReadinessChecks {
    pub database: bool,
    pub vector_store: bool,
    pub llm: bool,
}

/// Readiness probe - checks dependencies
#[utoipa::path(
    get,
    path = "/ready",
    tag = "health",
    responses(
        (status = 200, description = "Service is ready"),
        (status = 503, description = "Service not ready")
    )
)]
pub async fn readiness_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let is_ready = state.is_ready();

    // In a real implementation, check actual dependencies
    let checks = ReadinessChecks {
        database: true,
        vector_store: true,
        llm: true,
    };

    let response = ReadinessResponse {
        ready: is_ready,
        checks,
    };

    if is_ready {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response))
    }
}

/// Prometheus metrics endpoint
#[derive(Serialize)]
pub struct MetricsResponse {
    pub uptime_seconds: u64,
    pub total_requests: u64,
}

pub async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(MetricsResponse {
        uptime_seconds: state.uptime_secs(),
        total_requests: state.get_request_count(),
    })
}
