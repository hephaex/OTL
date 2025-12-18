//! Application state management
//!
//! Author: hephaex@gmail.com

use otl_core::config::AppConfig;
use otl_core::{LlmClient, SearchBackend, User};
use otl_rag::{HybridRagOrchestrator, RagConfig as OtlRagConfig};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Application state shared across handlers
pub struct AppState {
    /// Application configuration
    pub config: AppConfig,
    /// Server start time
    pub start_time: Instant,
    /// Request counter
    pub request_count: AtomicU64,
    /// Ready status
    pub is_ready: AtomicBool,
    /// RAG orchestrator (optional - initialized lazily)
    pub rag: RwLock<Option<Arc<HybridRagOrchestrator>>>,
    /// Vector search backend
    pub vector_store: RwLock<Option<Arc<dyn SearchBackend>>>,
    /// Graph search backend
    pub graph_store: RwLock<Option<Arc<dyn SearchBackend>>>,
    /// LLM client
    pub llm_client: RwLock<Option<Arc<dyn LlmClient>>>,
}

impl AppState {
    /// Create new application state with config
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
            request_count: AtomicU64::new(0),
            is_ready: AtomicBool::new(true),
            rag: RwLock::new(None),
            vector_store: RwLock::new(None),
            graph_store: RwLock::new(None),
            llm_client: RwLock::new(None),
        }
    }

    /// Increment request counter
    pub fn increment_requests(&self) -> u64 {
        self.request_count.fetch_add(1, Ordering::SeqCst)
    }

    /// Get total request count
    pub fn get_request_count(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Check if service is ready
    pub fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::SeqCst)
    }

    /// Set ready status
    pub fn set_ready(&self, ready: bool) {
        self.is_ready.store(ready, Ordering::SeqCst);
    }

    /// Initialize RAG orchestrator with provided backends
    pub async fn initialize_rag(
        &self,
        vector_store: Arc<dyn SearchBackend>,
        graph_store: Arc<dyn SearchBackend>,
        llm_client: Arc<dyn LlmClient>,
    ) {
        let rag_config = OtlRagConfig::default();
        let orchestrator = HybridRagOrchestrator::new(
            vector_store.clone(),
            graph_store.clone(),
            llm_client.clone(),
            rag_config,
        );

        *self.vector_store.write().await = Some(vector_store);
        *self.graph_store.write().await = Some(graph_store);
        *self.llm_client.write().await = Some(llm_client);
        *self.rag.write().await = Some(Arc::new(orchestrator));
    }

    /// Get RAG orchestrator if initialized
    pub async fn get_rag(&self) -> Option<Arc<HybridRagOrchestrator>> {
        self.rag.read().await.clone()
    }

    /// Check if RAG is initialized
    pub async fn has_rag(&self) -> bool {
        self.rag.read().await.is_some()
    }

    /// Get default user for API requests (can be extended with auth)
    pub fn get_default_user(&self, user_id: Option<&str>) -> User {
        match user_id {
            Some(id) => User::internal(id, vec!["EMPLOYEE".to_string()]),
            None => User::internal("api_user", vec!["EMPLOYEE".to_string()]),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(AppConfig::default())
    }
}
