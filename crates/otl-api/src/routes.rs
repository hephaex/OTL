//! API route definitions
//!
//! Author: hephaex@gmail.com

use crate::auth::middleware::auth_middleware;
use crate::handlers::{auth, documents, graph, ontology, query, verify};
use crate::middleware::rate_limit;
use crate::state::AppState;
use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

/// Create API v1 routes with rate limiting
pub fn api_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let rate_limit_config = &state.config.server.rate_limit;

    // Auth routes (no authentication required)
    // Rate limiting: 5 requests per minute per IP to prevent brute force attacks
    let auth_routes = if rate_limit_config.enabled {
        Router::new()
            .route("/auth/register", post(auth::register_handler))
            .route("/auth/login", post(auth::login_handler))
            .route("/auth/refresh", post(auth::refresh_handler))
            .layer(rate_limit::auth_rate_limit_layer(rate_limit_config))
    } else {
        Router::new()
            .route("/auth/register", post(auth::register_handler))
            .route("/auth/login", post(auth::login_handler))
            .route("/auth/refresh", post(auth::refresh_handler))
    };

    // Streaming endpoints (authentication required)
    // Rate limiting: 10 requests per minute per IP due to high resource usage
    let streaming_routes = if rate_limit_config.enabled {
        Router::new()
            .route("/query/stream", post(query::query_stream_handler))
            .layer(middleware::from_fn(auth_middleware))
            .layer(rate_limit::streaming_rate_limit_layer(rate_limit_config))
    } else {
        Router::new()
            .route("/query/stream", post(query::query_stream_handler))
            .layer(middleware::from_fn(auth_middleware))
    };

    // Protected routes (authentication required)
    // Rate limiting: 100 requests per minute per IP for normal API operations
    let protected_routes = if rate_limit_config.enabled {
        Router::new()
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
            // Ontology endpoints (legacy)
            .route("/ontology", get(graph::get_ontology))
            .route("/ontology", put(graph::update_ontology))
            // Ontology management endpoints (new)
            .route("/ontology/schema", get(ontology::get_current_schema))
            .route("/ontology/schema", post(ontology::create_schema))
            .route("/ontology/schema/:version", get(ontology::get_schema_version))
            .route("/ontology/schema/versions", get(ontology::list_schema_versions))
            .route("/ontology/schema/diff", get(ontology::compare_schemas))
            .route("/ontology/schema/export", get(ontology::export_schema))
            .route("/ontology/schema/import", post(ontology::import_schema))
            .route("/ontology/schema/validate", post(ontology::validate_schema))
            // Verification endpoints
            .route("/verify/pending", get(verify::list_pending))
            .route("/verify/:id/approve", post(verify::approve_extraction))
            .route("/verify/:id/reject", post(verify::reject_extraction))
            .route("/verify/batch/approve", post(verify::batch_approve))
            .route("/verify/batch/reject", post(verify::batch_reject))
            .route("/verify/stats", get(verify::get_stats))
            .route("/verify/metrics", get(verify::get_quality_metrics))
            .layer(middleware::from_fn(auth_middleware))
            .layer(rate_limit::api_rate_limit_layer(rate_limit_config))
    } else {
        Router::new()
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
            // Ontology endpoints (legacy)
            .route("/ontology", get(graph::get_ontology))
            .route("/ontology", put(graph::update_ontology))
            // Ontology management endpoints (new)
            .route("/ontology/schema", get(ontology::get_current_schema))
            .route("/ontology/schema", post(ontology::create_schema))
            .route("/ontology/schema/:version", get(ontology::get_schema_version))
            .route("/ontology/schema/versions", get(ontology::list_schema_versions))
            .route("/ontology/schema/diff", get(ontology::compare_schemas))
            .route("/ontology/schema/export", get(ontology::export_schema))
            .route("/ontology/schema/import", post(ontology::import_schema))
            .route("/ontology/schema/validate", post(ontology::validate_schema))
            // Verification endpoints
            .route("/verify/pending", get(verify::list_pending))
            .route("/verify/:id/approve", post(verify::approve_extraction))
            .route("/verify/:id/reject", post(verify::reject_extraction))
            .route("/verify/batch/approve", post(verify::batch_approve))
            .route("/verify/batch/reject", post(verify::batch_reject))
            .route("/verify/stats", get(verify::get_stats))
            .route("/verify/metrics", get(verify::get_quality_metrics))
            .layer(middleware::from_fn(auth_middleware))
    };

    // Combine routes
    Router::new()
        .merge(auth_routes)
        .merge(streaming_routes)
        .merge(protected_routes)
}
