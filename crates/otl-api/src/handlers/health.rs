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
    let (cache_hits, cache_misses) = state.get_cache_stats();

    // Database pool metrics
    let db_pool = &state.db_pool;
    let pool_size = db_pool.size();
    let pool_idle = db_pool.num_idle();

    // Build basic metrics
    let mut output = String::new();

    // Server info
    output.push_str("# HELP otl_uptime_seconds Time since server start\n");
    output.push_str("# TYPE otl_uptime_seconds gauge\n");
    output.push_str(&format!("otl_uptime_seconds {uptime}\n\n"));

    output.push_str("# HELP otl_requests_total Total number of HTTP requests\n");
    output.push_str("# TYPE otl_requests_total counter\n");
    output.push_str(&format!("otl_requests_total {total_requests}\n\n"));

    output.push_str("# HELP otl_rag_enabled Whether RAG is initialized\n");
    output.push_str("# TYPE otl_rag_enabled gauge\n");
    output.push_str(&format!("otl_rag_enabled {}\n\n", if has_rag { 1 } else { 0 }));

    output.push_str("# HELP otl_build_info Build information\n");
    output.push_str("# TYPE otl_build_info gauge\n");
    output.push_str(&format!(
        "otl_build_info{{version=\"{}\"}} 1\n\n",
        env!("CARGO_PKG_VERSION")
    ));

    // Cache metrics
    output.push_str("# HELP otl_cache_hits_total Total number of cache hits\n");
    output.push_str("# TYPE otl_cache_hits_total counter\n");
    output.push_str(&format!("otl_cache_hits_total {cache_hits}\n\n"));

    output.push_str("# HELP otl_cache_misses_total Total number of cache misses\n");
    output.push_str("# TYPE otl_cache_misses_total counter\n");
    output.push_str(&format!("otl_cache_misses_total {cache_misses}\n\n"));

    let total_cache = cache_hits + cache_misses;
    if total_cache > 0 {
        let hit_rate = cache_hits as f64 / total_cache as f64;
        output.push_str("# HELP otl_cache_hit_rate Cache hit rate (0.0 to 1.0)\n");
        output.push_str("# TYPE otl_cache_hit_rate gauge\n");
        output.push_str(&format!("otl_cache_hit_rate {hit_rate:.4}\n\n"));
    }

    // Database pool metrics
    output.push_str("# HELP otl_db_pool_connections_active Active database connections\n");
    output.push_str("# TYPE otl_db_pool_connections_active gauge\n");
    output.push_str(&format!(
        "otl_db_pool_connections_active {}\n\n",
        pool_size.saturating_sub(pool_idle as u32)
    ));

    output.push_str("# HELP otl_db_pool_connections_idle Idle database connections\n");
    output.push_str("# TYPE otl_db_pool_connections_idle gauge\n");
    output.push_str(&format!("otl_db_pool_connections_idle {pool_idle}\n\n"));

    output.push_str("# HELP otl_db_pool_connections_total Total database pool size\n");
    output.push_str("# TYPE otl_db_pool_connections_total gauge\n");
    output.push_str(&format!("otl_db_pool_connections_total {pool_size}\n\n"));

    // Per-endpoint metrics
    let metrics = state.metrics.read().await;

    // Request counts by endpoint and status
    output.push_str("# HELP otl_http_requests_total HTTP requests by endpoint and status\n");
    output.push_str("# TYPE otl_http_requests_total counter\n");
    for (endpoint, endpoint_metrics) in metrics.iter() {
        for (status, count) in &endpoint_metrics.status_counts {
            output.push_str(&format!(
                "otl_http_requests_total{{endpoint=\"{}\",status=\"{}\"}} {}\n",
                endpoint, status, count
            ));
        }
    }
    output.push('\n');

    // Request latency histogram
    output.push_str("# HELP otl_http_request_duration_seconds HTTP request latency\n");
    output.push_str("# TYPE otl_http_request_duration_seconds histogram\n");
    for (endpoint, endpoint_metrics) in metrics.iter() {
        if endpoint_metrics.latency_count > 0 {
            let _avg_latency_s = (endpoint_metrics.total_latency_us as f64)
                / (endpoint_metrics.latency_count as f64)
                / 1_000_000.0;

            // Histogram buckets
            let mut cumulative = 0u64;
            for (le, count) in [
                ("0.01", endpoint_metrics.latency_buckets.under_10ms),
                ("0.05", endpoint_metrics.latency_buckets.ms_10_50),
                ("0.1", endpoint_metrics.latency_buckets.ms_50_100),
                ("0.5", endpoint_metrics.latency_buckets.ms_100_500),
                ("1.0", endpoint_metrics.latency_buckets.ms_500_1000),
            ] {
                cumulative += count;
                output.push_str(&format!(
                    "otl_http_request_duration_seconds_bucket{{endpoint=\"{}\",le=\"{}\"}} {}\n",
                    endpoint, le, cumulative
                ));
            }

            // +Inf bucket
            cumulative += endpoint_metrics.latency_buckets.over_1s;
            output.push_str(&format!(
                "otl_http_request_duration_seconds_bucket{{endpoint=\"{}\",le=\"+Inf\"}} {}\n",
                endpoint, cumulative
            ));

            // Sum and count
            let total_sum_s =
                (endpoint_metrics.total_latency_us as f64) / 1_000_000.0;
            output.push_str(&format!(
                "otl_http_request_duration_seconds_sum{{endpoint=\"{}\"}} {:.6}\n",
                endpoint, total_sum_s
            ));
            output.push_str(&format!(
                "otl_http_request_duration_seconds_count{{endpoint=\"{}\"}} {}\n",
                endpoint, endpoint_metrics.latency_count
            ));
        }
    }
    output.push('\n');

    // Latency quantiles (approximated from buckets)
    output.push_str("# HELP otl_http_request_duration_seconds_summary HTTP request latency summary\n");
    output.push_str("# TYPE otl_http_request_duration_seconds_summary summary\n");
    for (endpoint, endpoint_metrics) in metrics.iter() {
        if endpoint_metrics.latency_count > 0 {
            let _avg_s = (endpoint_metrics.total_latency_us as f64)
                / (endpoint_metrics.latency_count as f64)
                / 1_000_000.0;
            let _min_s = (endpoint_metrics.min_latency_us as f64) / 1_000_000.0;
            let _max_s = (endpoint_metrics.max_latency_us as f64) / 1_000_000.0;

            // Approximate percentiles from histogram buckets
            let total = endpoint_metrics.latency_count;
            let p50_threshold = total / 2;
            let p90_threshold = (total * 9) / 10;
            let p99_threshold = (total * 99) / 100;

            let (p50, p90, p99) = calculate_percentiles(endpoint_metrics, p50_threshold, p90_threshold, p99_threshold);

            output.push_str(&format!(
                "otl_http_request_duration_seconds_summary{{endpoint=\"{}\",quantile=\"0.5\"}} {:.6}\n",
                endpoint, p50
            ));
            output.push_str(&format!(
                "otl_http_request_duration_seconds_summary{{endpoint=\"{}\",quantile=\"0.9\"}} {:.6}\n",
                endpoint, p90
            ));
            output.push_str(&format!(
                "otl_http_request_duration_seconds_summary{{endpoint=\"{}\",quantile=\"0.99\"}} {:.6}\n",
                endpoint, p99
            ));
        }
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        output,
    )
}

/// Calculate approximate percentiles from histogram buckets
fn calculate_percentiles(
    metrics: &crate::state::EndpointMetrics,
    p50: u64,
    p90: u64,
    p99: u64,
) -> (f64, f64, f64) {
    let mut cumulative = 0u64;
    let mut p50_val = 0.005; // default to midpoint of first bucket
    let mut p90_val = 0.005;
    let mut p99_val = 0.005;

    // under 10ms bucket
    cumulative += metrics.latency_buckets.under_10ms;
    if cumulative >= p50 && p50_val == 0.005 {
        p50_val = 0.005;
    }
    if cumulative >= p90 && p90_val == 0.005 {
        p90_val = 0.005;
    }
    if cumulative >= p99 && p99_val == 0.005 {
        p99_val = 0.005;
    }

    // 10-50ms bucket
    cumulative += metrics.latency_buckets.ms_10_50;
    if cumulative >= p50 && p50_val == 0.005 {
        p50_val = 0.03;
    }
    if cumulative >= p90 && p90_val == 0.005 {
        p90_val = 0.03;
    }
    if cumulative >= p99 && p99_val == 0.005 {
        p99_val = 0.03;
    }

    // 50-100ms bucket
    cumulative += metrics.latency_buckets.ms_50_100;
    if cumulative >= p50 && p50_val == 0.03 {
        p50_val = 0.075;
    }
    if cumulative >= p90 && p90_val == 0.03 {
        p90_val = 0.075;
    }
    if cumulative >= p99 && p99_val == 0.03 {
        p99_val = 0.075;
    }

    // 100-500ms bucket
    cumulative += metrics.latency_buckets.ms_100_500;
    if cumulative >= p50 && p50_val == 0.075 {
        p50_val = 0.3;
    }
    if cumulative >= p90 && p90_val == 0.075 {
        p90_val = 0.3;
    }
    if cumulative >= p99 && p99_val == 0.075 {
        p99_val = 0.3;
    }

    // 500-1000ms bucket
    cumulative += metrics.latency_buckets.ms_500_1000;
    if cumulative >= p50 && p50_val == 0.3 {
        p50_val = 0.75;
    }
    if cumulative >= p90 && p90_val == 0.3 {
        p90_val = 0.75;
    }
    if cumulative >= p99 && p99_val == 0.3 {
        p99_val = 0.75;
    }

    // over 1s bucket
    if cumulative < p50 {
        p50_val = 1.5;
    }
    if cumulative < p90 {
        p90_val = 1.5;
    }
    if cumulative < p99 {
        p99_val = 1.5;
    }

    (p50_val, p90_val, p99_val)
}
