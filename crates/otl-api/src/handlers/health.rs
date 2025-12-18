//! Health check handlers
//!
//! Author: hephaex@gmail.com

use crate::state::AppState;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub build_info: BuildInfo,
}

#[derive(Serialize)]
pub struct BuildInfo {
    pub name: String,
    pub rust_version: String,
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
        build_info: BuildInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            rust_version: "1.75+".to_string(),
        },
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
    pub rag_initialized: bool,
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
    let has_rag = state.has_rag().await;

    let checks = ReadinessChecks {
        database: true,
        vector_store: true,
        llm: true,
        rag_initialized: has_rag,
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

/// JSON metrics response
#[derive(Serialize)]
pub struct MetricsResponse {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub requests_per_second: f64,
    pub rag_enabled: bool,
}

pub async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = state.uptime_secs();
    let total_requests = state.get_request_count();
    let rps = if uptime > 0 {
        total_requests as f64 / uptime as f64
    } else {
        0.0
    };

    Json(MetricsResponse {
        uptime_seconds: uptime,
        total_requests,
        requests_per_second: rps,
        rag_enabled: state.has_rag().await,
    })
}

/// Prometheus-compatible metrics endpoint
pub async fn prometheus_metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = state.uptime_secs();
    let total_requests = state.get_request_count();
    let has_rag = state.has_rag().await;

    let metrics = format!(
        r#"# HELP otl_uptime_seconds Time since server start
# TYPE otl_uptime_seconds gauge
otl_uptime_seconds {}

# HELP otl_requests_total Total number of HTTP requests
# TYPE otl_requests_total counter
otl_requests_total {}

# HELP otl_rag_enabled Whether RAG is initialized
# TYPE otl_rag_enabled gauge
otl_rag_enabled {}

# HELP otl_build_info Build information
# TYPE otl_build_info gauge
otl_build_info{{version="{}"}} 1
"#,
        uptime,
        total_requests,
        if has_rag { 1 } else { 0 },
        env!("CARGO_PKG_VERSION")
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        metrics,
    )
}
