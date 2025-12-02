//! OTL API - REST/gRPC server
//!
//! Provides HTTP endpoints for querying the knowledge system.

pub mod handlers;
pub mod middleware;
pub mod routes;

use axum::{routing::get, Router};

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/v1/query", axum::routing::post(handlers::query))
}
