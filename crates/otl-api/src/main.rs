//! OTL API Server
//!
//! REST API server for the OTL knowledge management system.
//!
//! Author: hephaex@gmail.com

use otl_api::{create_router, state::AppState};
use otl_core::config::AppConfig;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "otl_api=debug,tower_http=debug".into()),
        )
        .init();

    // Load configuration
    let config = AppConfig::from_env().unwrap_or_default();
    let host = std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080u16);

    let addr = format!("{host}:{port}");

    // Create application state
    let state = Arc::new(AppState::new(config));

    // Create router
    let app = create_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("OTL API Server starting on http://{}", addr);
    tracing::info!("Swagger UI available at http://{}/swagger-ui/", addr);
    tracing::info!("OpenAPI spec at http://{}/api-docs/openapi.json", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
