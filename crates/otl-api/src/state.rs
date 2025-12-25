//! Application state management
//!
//! Author: hephaex@gmail.com

use otl_core::config::AppConfig;
use otl_core::{LlmClient, SearchBackend, User};
use otl_graph::SurrealDbStore;
use otl_rag::{HybridRagOrchestrator, RagConfig as OtlRagConfig};
use otl_vector::VectorSearchBackend;
use sqlx::PgPool;
use std::collections::HashMap;
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
    /// PostgreSQL connection pool
    pub db_pool: PgPool,
    /// RAG orchestrator (optional - initialized lazily)
    pub rag: RwLock<Option<Arc<HybridRagOrchestrator>>>,
    /// Vector search backend
    pub vector_store: RwLock<Option<Arc<dyn SearchBackend>>>,
    /// Vector search backend (concrete type for indexing)
    pub vector_backend: RwLock<Option<Arc<VectorSearchBackend>>>,
    /// Graph search backend
    pub graph_store: RwLock<Option<Arc<dyn SearchBackend>>>,
    /// Direct graph database access
    pub graph_db: RwLock<Option<Arc<SurrealDbStore>>>,
    /// LLM client
    pub llm_client: RwLock<Option<Arc<dyn LlmClient>>>,
    /// Request metrics by endpoint and status
    pub metrics: RwLock<HashMap<String, EndpointMetrics>>,
    /// Cache hit counter (if cache is enabled)
    pub cache_hits: AtomicU64,
    /// Cache miss counter (if cache is enabled)
    pub cache_misses: AtomicU64,
}

/// Metrics for a specific endpoint
#[derive(Debug, Clone, Default)]
pub struct EndpointMetrics {
    /// Total requests to this endpoint
    pub request_count: u64,
    /// Requests by status code
    pub status_counts: HashMap<u16, u64>,
    /// Total latency in microseconds (for calculating average)
    pub total_latency_us: u64,
    /// Request count for latency calculation
    pub latency_count: u64,
    /// Minimum latency in microseconds
    pub min_latency_us: u64,
    /// Maximum latency in microseconds
    pub max_latency_us: u64,
    /// Latency buckets for histogram (microseconds)
    pub latency_buckets: LatencyBuckets,
}

/// Histogram buckets for latency tracking
#[derive(Debug, Clone, Default)]
pub struct LatencyBuckets {
    /// < 10ms
    pub under_10ms: u64,
    /// 10-50ms
    pub ms_10_50: u64,
    /// 50-100ms
    pub ms_50_100: u64,
    /// 100-500ms
    pub ms_100_500: u64,
    /// 500ms-1s
    pub ms_500_1000: u64,
    /// > 1s
    pub over_1s: u64,
}

impl AppState {
    /// Create new application state with config and database pool
    pub fn new(config: AppConfig, db_pool: PgPool) -> Self {
        Self {
            config,
            db_pool,
            start_time: Instant::now(),
            request_count: AtomicU64::new(0),
            is_ready: AtomicBool::new(true),
            rag: RwLock::new(None),
            vector_store: RwLock::new(None),
            vector_backend: RwLock::new(None),
            graph_store: RwLock::new(None),
            graph_db: RwLock::new(None),
            llm_client: RwLock::new(None),
            metrics: RwLock::new(HashMap::new()),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
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

    /// Set the vector backend (concrete type) for document indexing
    pub async fn set_vector_backend(&self, backend: Arc<VectorSearchBackend>) {
        *self.vector_backend.write().await = Some(backend);
    }

    /// Set the graph database (concrete type) for entity operations
    pub async fn set_graph_db(&self, db: Arc<SurrealDbStore>) {
        *self.graph_db.write().await = Some(db);
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

    /// Record a request with latency and status
    pub async fn record_request(&self, endpoint: String, status_code: u16, latency_us: u64) {
        let mut metrics = self.metrics.write().await;
        let endpoint_metrics = metrics.entry(endpoint).or_default();

        endpoint_metrics.request_count += 1;
        *endpoint_metrics
            .status_counts
            .entry(status_code)
            .or_insert(0) += 1;

        // Update latency statistics
        endpoint_metrics.total_latency_us += latency_us;
        endpoint_metrics.latency_count += 1;

        if endpoint_metrics.min_latency_us == 0 || latency_us < endpoint_metrics.min_latency_us {
            endpoint_metrics.min_latency_us = latency_us;
        }

        if latency_us > endpoint_metrics.max_latency_us {
            endpoint_metrics.max_latency_us = latency_us;
        }

        // Update histogram buckets
        let latency_ms = latency_us / 1000;
        match latency_ms {
            0..=9 => endpoint_metrics.latency_buckets.under_10ms += 1,
            10..=49 => endpoint_metrics.latency_buckets.ms_10_50 += 1,
            50..=99 => endpoint_metrics.latency_buckets.ms_50_100 += 1,
            100..=499 => endpoint_metrics.latency_buckets.ms_100_500 += 1,
            500..=999 => endpoint_metrics.latency_buckets.ms_500_1000 += 1,
            _ => endpoint_metrics.latency_buckets.over_1s += 1,
        }
    }

    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::SeqCst);
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::SeqCst);
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (u64, u64) {
        let hits = self.cache_hits.load(Ordering::SeqCst);
        let misses = self.cache_misses.load(Ordering::SeqCst);
        (hits, misses)
    }
}
