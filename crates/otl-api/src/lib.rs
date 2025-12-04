//! OTL API - REST server for knowledge management system
//!
//! Provides HTTP endpoints for:
//! - RAG queries
//! - Document management
//! - Knowledge graph operations
//! - HITL verification
//!
//! Author: hephaex@gmail.com

pub mod error;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

use axum::Router;
use state::AppState;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::query::query_handler,
        handlers::query::query_stream_handler,
        handlers::documents::list_documents,
        handlers::documents::get_document,
        handlers::documents::upload_document,
        handlers::documents::delete_document,
        handlers::graph::list_entities,
        handlers::graph::get_entity,
        handlers::graph::search_graph,
        handlers::verify::list_pending,
        handlers::verify::approve_extraction,
        handlers::verify::reject_extraction,
        handlers::health::health_check,
        handlers::health::readiness_check,
    ),
    components(
        schemas(
            handlers::query::QueryRequest,
            handlers::query::QueryResponse,
            handlers::query::Citation,
            handlers::documents::DocumentInfo,
            handlers::documents::DocumentListResponse,
            handlers::documents::UploadDocumentRequest,
            handlers::graph::EntityInfo,
            handlers::graph::RelationInfo,
            handlers::graph::GraphSearchRequest,
            handlers::graph::GraphSearchResponse,
            handlers::verify::PendingExtraction,
            handlers::verify::VerifyAction,
            error::ApiError,
        )
    ),
    tags(
        (name = "query", description = "RAG query endpoints"),
        (name = "documents", description = "Document management"),
        (name = "graph", description = "Knowledge graph operations"),
        (name = "verify", description = "HITL verification"),
        (name = "health", description = "Health checks"),
    ),
    info(
        title = "OTL API",
        version = "1.0.0",
        description = "Ontology-based Knowledge System API",
        license(name = "Apache-2.0"),
        contact(name = "hephaex", email = "hephaex@gmail.com")
    )
)]
pub struct ApiDoc;

/// Create the main router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", routes::api_routes())
        .route(
            "/health",
            axum::routing::get(handlers::health::health_check),
        )
        .route(
            "/ready",
            axum::routing::get(handlers::health::readiness_check),
        )
        .route("/metrics", axum::routing::get(handlers::health::metrics))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Create router with default state (for testing)
pub fn create_router_default() -> Router {
    let state = Arc::new(AppState::default());
    create_router(state)
}
