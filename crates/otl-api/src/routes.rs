//! API route definitions
//!
//! Author: hephaex@gmail.com

use crate::auth::middleware::auth_middleware;
use crate::handlers::{auth, documents, graph, query, verify};
use crate::state::AppState;
use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

/// Create API v1 routes
pub fn api_routes() -> Router<Arc<AppState>> {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/auth/register", post(auth::register_handler))
        .route("/auth/login", post(auth::login_handler))
        .route("/auth/refresh", post(auth::refresh_handler));

    // Protected routes (authentication required)
    let protected_routes = Router::new()
        .route("/auth/logout", post(auth::logout_handler))
        .route("/auth/me", get(auth::me_handler))
        // Query endpoints
        .route("/query", post(query::query_handler))
        .route("/query/stream", post(query::query_stream_handler))
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

    // Combine routes
    Router::new().merge(public_routes).merge(protected_routes)
}
