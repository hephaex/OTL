//! OTL API - REST server for knowledge management system
//!
//! Provides HTTP endpoints for:
//! - RAG queries
//! - Document management
//! - Knowledge graph operations
//! - HITL verification
//! - Authentication and authorization
//!
//! Author: hephaex@gmail.com

pub mod auth;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

use axum::Router;
use state::AppState;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::auth::register_handler,
        handlers::auth::login_handler,
        handlers::auth::refresh_handler,
        handlers::auth::logout_handler,
        handlers::auth::me_handler,
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
            auth::RegisterRequest,
            auth::LoginRequest,
            auth::RefreshRequest,
            auth::LogoutRequest,
            auth::AuthResponse,
            auth::UserInfo,
            handlers::auth::RegisterResponse,
            handlers::auth::LogoutResponse,
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
        (name = "auth", description = "Authentication and authorization"),
        (name = "query", description = "RAG query endpoints"),
        (name = "documents", description = "Document management"),
        (name = "graph", description = "Knowledge graph operations"),
        (name = "verify", description = "HITL verification"),
        (name = "health", description = "Health checks"),
    ),
    modifiers(&SecurityAddon),
    info(
        title = "OTL API",
        version = "1.0.0",
        description = "Ontology-based Knowledge System API",
        license(name = "Apache-2.0"),
        contact(name = "hephaex", email = "hephaex@gmail.com")
    )
)]
pub struct ApiDoc;

/// Security scheme for OpenAPI
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            let mut http = utoipa::openapi::security::Http::new(
                utoipa::openapi::security::HttpAuthScheme::Bearer,
            );
            http.bearer_format = Some("JWT".to_string());

            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(http),
            );
        }
    }
}

/// Create the main router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    // Configure CORS based on config
    let cors = if state.config.server.cors_origins.is_empty() {
        // Empty origins = allow all (for development)
        tracing::warn!(
            "CORS_ORIGINS not set, allowing all origins (not recommended for production)"
        );
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        // Use configured origins
        let origins: Vec<_> = state
            .config
            .server
            .cors_origins
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        tracing::info!("CORS configured with {} allowed origins", origins.len());
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(Any)
            .allow_headers(Any)
    };

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
        .route(
            "/metrics/prometheus",
            axum::routing::get(handlers::health::prometheus_metrics),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Create AppState for testing without requiring a real database
///
/// This function is available in test builds and creates a minimal
/// AppState that can be used for API endpoint testing without database connectivity.
///
/// Note: Tests that use database operations will fail with 500 errors.
/// For those tests, use a real test database with proper migrations.
#[cfg(any(test, feature = "test-utils"))]
pub fn create_test_state() -> Arc<AppState> {
    use otl_core::config::AppConfig;
    use sqlx::postgres::PgPoolOptions;

    // Create a minimal test config
    let config = AppConfig::default();

    // Create an unconnected pool (0 connections)
    // This works for testing routes that don't actually use the database
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://test:test@localhost/test")
        .expect("Failed to create test pool");

    Arc::new(AppState::new(config, pool))
}

/// Create a router for testing with a mock database pool
///
/// This function is available in test builds.
///
/// Note: Tests that use database operations will fail with 500 errors.
/// For those tests, use a real test database with proper migrations.
#[cfg(any(test, feature = "test-utils"))]
pub fn create_router_for_testing() -> Router {
    create_router(create_test_state())
}
