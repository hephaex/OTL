//! OTL API Server
//!
//! REST API server for the OTL knowledge management system.
//!
//! Author: hephaex@gmail.com

use otl_api::{create_router, state::AppState};
use otl_core::config::AppConfig;
use otl_graph::GraphSearchBackend;
use otl_rag::llm::create_llm_client;
use otl_vector::embedding::create_embedding_client;
use otl_vector::VectorSearchBackend;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "otl_api=info,tower_http=info".into()),
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
    let state = Arc::new(AppState::new(config.clone()));

    // Initialize RAG pipeline components
    let mut rag_initialized = false;

    // 1. Initialize LLM client
    let llm_client = match create_llm_client(&config.llm) {
        Ok(client) => {
            tracing::info!(
                "LLM client initialized: {:?} with model {}",
                config.llm.provider,
                config.llm.model
            );
            Some(Arc::from(client))
        }
        Err(e) => {
            tracing::warn!("Failed to initialize LLM client: {}", e);
            None
        }
    };

    // 2. Initialize Embedding client
    let embedding_client = match create_embedding_client(&config.llm) {
        Ok(client) => {
            tracing::info!(
                "Embedding client initialized with dimension {}",
                client.dimension()
            );
            Some(Arc::from(client))
        }
        Err(e) => {
            tracing::warn!("Failed to initialize embedding client: {}", e);
            None
        }
    };

    // 3. Initialize Vector Store (Qdrant)
    let vector_store = if let Some(emb_client) = embedding_client {
        match VectorSearchBackend::from_config(&config.database, emb_client).await {
            Ok(store) => {
                // Initialize collection
                if let Err(e) = store.init().await {
                    tracing::warn!("Failed to init Qdrant collection: {}", e);
                } else {
                    tracing::info!("Vector store (Qdrant) initialized");
                }
                let store_arc = Arc::new(store);

                // Set the concrete backend for document indexing
                state.set_vector_backend(store_arc.clone()).await;

                Some(store_arc as Arc<dyn otl_core::SearchBackend>)
            }
            Err(e) => {
                tracing::warn!("Failed to connect to Qdrant: {}", e);
                None
            }
        }
    } else {
        None
    };

    // 4. Initialize Graph Store (SurrealDB)
    let graph_store = match GraphSearchBackend::new(&config.database).await {
        Ok(store) => {
            tracing::info!("Graph store (SurrealDB) initialized");
            Some(Arc::new(store) as Arc<dyn otl_core::SearchBackend>)
        }
        Err(e) => {
            tracing::warn!("Failed to connect to SurrealDB: {}", e);
            None
        }
    };

    // 5. Initialize full RAG pipeline if all components are available
    if let (Some(llm), Some(vs), Some(gs)) = (
        llm_client.clone(),
        vector_store.clone(),
        graph_store.clone(),
    ) {
        state.initialize_rag(vs, gs, llm).await;
        rag_initialized = true;
        tracing::info!("RAG pipeline fully initialized");
    } else {
        // At least set LLM client for streaming
        if let Some(llm) = llm_client {
            *state.llm_client.write().await = Some(llm);
            tracing::info!("LLM client set for streaming (RAG not fully initialized)");
        }
    }

    // Create router
    let app = create_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("OTL API Server starting on http://{}", addr);
    tracing::info!("Swagger UI available at http://{}/swagger-ui/", addr);
    tracing::info!("OpenAPI spec at http://{}/api-docs/openapi.json", addr);
    tracing::info!("RAG initialized: {}", rag_initialized);

    axum::serve(listener, app).await?;

    Ok(())
}
