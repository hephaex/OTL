//! API route definitions
//!
//! Author: hephaex@gmail.com

use crate::handlers::{documents, graph, query, verify};
use crate::state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

/// Create API v1 routes
pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
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
}
