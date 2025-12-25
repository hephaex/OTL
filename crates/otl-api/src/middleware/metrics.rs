//! Metrics tracking middleware
//!
//! Tracks request latency, counts, and status codes for Prometheus metrics
//!
//! Author: hephaex@gmail.com

use crate::state::AppState;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

/// Metrics tracking middleware
///
/// Records:
/// - Request count per endpoint
/// - Request latency distribution
/// - Response status codes
pub async fn metrics_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let path = request.uri().path().to_string();

    // Normalize the path for metrics (remove IDs)
    let endpoint = normalize_endpoint(&path);

    // Run the request
    let response = next.run(request).await;

    // Record metrics
    let latency_us = start.elapsed().as_micros() as u64;
    let status = response.status();

    // Record asynchronously to avoid blocking
    let state_clone = state.clone();
    let endpoint_clone = endpoint.clone();
    tokio::spawn(async move {
        state_clone
            .record_request(endpoint_clone, status.as_u16(), latency_us)
            .await;
    });

    response
}

/// Normalize endpoint paths for consistent metrics
///
/// Replaces UUID/ID segments with placeholders to group similar endpoints
fn normalize_endpoint(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = segments
        .iter()
        .map(|seg| {
            // Replace UUIDs with :id
            if is_uuid(seg) || is_numeric(seg) {
                ":id".to_string()
            } else {
                (*seg).to_string()
            }
        })
        .collect();

    normalized.join("/")
}

/// Check if a string looks like a UUID
fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.chars()
            .enumerate()
            .all(|(i, c)| match i {
                8 | 13 | 18 | 23 => c == '-',
                _ => c.is_ascii_hexdigit(),
            })
}

/// Check if a string is numeric (likely an ID)
fn is_numeric(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_endpoint() {
        assert_eq!(
            normalize_endpoint("/api/v1/documents/123"),
            "/api/v1/documents/:id"
        );
        assert_eq!(
            normalize_endpoint("/api/v1/documents/550e8400-e29b-41d4-a716-446655440000"),
            "/api/v1/documents/:id"
        );
        assert_eq!(
            normalize_endpoint("/api/v1/query"),
            "/api/v1/query"
        );
        assert_eq!(
            normalize_endpoint("/health"),
            "/health"
        );
    }

    #[test]
    fn test_is_uuid() {
        assert!(is_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_uuid("not-a-uuid"));
        assert!(!is_uuid("123"));
    }

    #[test]
    fn test_is_numeric() {
        assert!(is_numeric("123"));
        assert!(is_numeric("999"));
        assert!(!is_numeric("abc"));
        assert!(!is_numeric("12a"));
        assert!(!is_numeric(""));
    }
}
