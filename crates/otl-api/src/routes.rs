//! API route definitions
//!
//! Author: hephaex@gmail.com

use crate::auth::middleware::auth_middleware;
use crate::handlers::{auth, documents, graph, query, verify};
// TODO: Re-enable rate limiting once tower_governor is updated to 0.8+
// use crate::middleware::rate_limit;
use crate::state::AppState;
use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

/// Create API v1 routes
pub fn api_routes() -> Router<Arc<AppState>> {
    // Auth routes (no authentication required)
    // TODO: Add rate limiting - 5 requests per minute per IP to prevent brute force attacks
    let auth_routes = Router::new()
        .route("/auth/register", post(auth::register_handler))
        .route("/auth/login", post(auth::login_handler))
        .route("/auth/refresh", post(auth::refresh_handler));
    // .layer(rate_limit::auth_rate_limit());

    // Streaming endpoints (authentication required)
    // TODO: Add rate limiting - 10 requests per minute per IP due to high resource usage
    let streaming_routes = Router::new()
        .route("/query/stream", post(query::query_stream_handler))
        .layer(middleware::from_fn(auth_middleware));
    // .layer(rate_limit::streaming_rate_limit());

    // Protected routes (authentication required)
    // TODO: Add rate limiting - 100 requests per minute per IP for normal API operations
    let protected_routes = Router::new()
        .route("/auth/logout", post(auth::logout_handler))
        .route("/auth/me", get(auth::me_handler))
        // Query endpoints
        .route("/query", post(query::query_handler))
        // Document endpoints
        .route("/documents", get(documents::list_documents))
        .route("/documents", post(documents::upload_document))
        .route("/documents/:id", get(documents::get_document))
        .route("/documents/:id", delete(documents::delete_document))
        // Graph endpoints
        .route("/graph/entities", get(graph::list_entities))
        .route("/graph/entities/:id", get(graph::get_entity))
        .route("/graph/search", post(graph::search_graph))
        // Ontology endpoints
        .route("/ontology", get(graph::get_ontology))
        .route("/ontology", put(graph::update_ontology))
        // Verification endpoints
        .route("/verify/pending", get(verify::list_pending))
        .route("/verify/:id/approve", post(verify::approve_extraction))
        .route("/verify/:id/reject", post(verify::reject_extraction))
        .route("/verify/stats", get(verify::get_stats))
        .layer(middleware::from_fn(auth_middleware));
    // .layer(rate_limit::api_rate_limit());

    // Combine routes
    Router::new()
        .merge(auth_routes)
        .merge(streaming_routes)
        .merge(protected_routes)
}
