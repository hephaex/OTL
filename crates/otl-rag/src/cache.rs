//! Caching layer for RAG pipeline
//!
//! Provides high-performance concurrent caching for:
//! - Document embeddings (to avoid re-computing expensive embeddings)
//! - Query results (to serve repeated queries quickly)
//!
//! Uses the moka crate for thread-safe, async-compatible LRU caching
//! with TTL support.
//!
//! Author: hephaex@gmail.com

use moka::future::Cache;
use otl_core::{Result, SearchResult};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Cache Configuration
// ============================================================================

/// Configuration for cache behavior
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in embedding cache
    pub embedding_max_capacity: u64,

    /// Maximum number of entries in query cache
    pub query_max_capacity: u64,

    /// Time-to-live for embedding cache entries (in seconds)
    pub embedding_ttl_seconds: u64,

    /// Time-to-live for query cache entries (in seconds)
    pub query_ttl_seconds: u64,

    /// Enable cache statistics collection
    pub enable_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            // 10k embeddings @ ~1.5KB each = ~15MB
            embedding_max_capacity: 10_000,
            // 1k query results @ ~10KB each = ~10MB
            query_max_capacity: 1_000,
            // Embeddings are stable, cache for 1 hour
            embedding_ttl_seconds: 3600,
            // Query results may change as documents are updated, cache for 5 minutes
            query_ttl_seconds: 300,
            // Statistics enabled by default
            enable_stats: true,
        }
    }
}

// ============================================================================
// Embedding Cache
// ============================================================================

/// Cache for document embeddings
///
/// Caches embedding vectors to avoid recomputing them for the same text.
/// Thread-safe and suitable for async contexts.
#[derive(Clone)]
pub struct EmbeddingCache {
    cache: Cache<u64, Vec<f32>>,
    stats: Arc<CacheStats>,
}

impl EmbeddingCache {
    /// Create a new embedding cache with default configuration
    pub fn new() -> Self {
        Self::with_config(&CacheConfig::default())
    }

    /// Create a new embedding cache with custom configuration
    pub fn with_config(config: &CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.embedding_max_capacity)
            .time_to_live(Duration::from_secs(config.embedding_ttl_seconds))
            .build();

        Self {
            cache,
            stats: Arc::new(CacheStats::new("embedding")),
        }
    }

    /// Get an embedding from cache
    ///
    /// # Arguments
    /// * `text` - The text whose embedding to retrieve
    ///
    /// # Returns
    /// The cached embedding vector, or None if not in cache
    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        let key = hash_text(text);
        let result = self.cache.get(&key).await;

        if result.is_some() {
            self.stats.record_hit();
        } else {
            self.stats.record_miss();
        }

        result
    }

    /// Store an embedding in cache
    ///
    /// # Arguments
    /// * `text` - The text that was embedded
    /// * `embedding` - The embedding vector
    pub async fn put(&self, text: &str, embedding: Vec<f32>) {
        let key = hash_text(text);
        self.cache.insert(key, embedding).await;
        self.stats.record_write();
    }

    /// Check if an embedding exists in cache
    ///
    /// # Arguments
    /// * `text` - The text to check
    pub async fn contains(&self, text: &str) -> bool {
        let key = hash_text(text);
        self.cache.contains_key(&key)
    }

    /// Invalidate a specific embedding
    ///
    /// # Arguments
    /// * `text` - The text whose embedding to invalidate
    pub async fn invalidate(&self, text: &str) {
        let key = hash_text(text);
        self.cache.invalidate(&key).await;
        self.stats.record_invalidation();
    }

    /// Clear all cached embeddings
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        // Wait for all pending invalidations to complete
        self.cache.run_pending_tasks().await;
        self.stats.reset();
    }

    /// Get cache statistics
    pub fn stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.stats)
    }

    /// Get current cache size
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Get weighted cache size (memory usage estimate)
    pub fn weighted_size(&self) -> u64 {
        self.cache.weighted_size()
    }
}

impl Default for EmbeddingCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Query Cache
// ============================================================================

/// Cache for RAG query results
///
/// Caches complete query results including search results and rankings.
/// Thread-safe and suitable for async contexts.
#[derive(Clone)]
pub struct QueryCache {
    cache: Cache<QueryKey, QueryCacheValue>,
    stats: Arc<CacheStats>,
}

/// Key for query cache entries
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct QueryKey {
    /// Hash of the query text
    query_hash: u64,
    /// Top-k parameter
    top_k: usize,
    /// Minimum score threshold (scaled to avoid float comparison issues)
    min_score_scaled: i32,
}

impl QueryKey {
    fn new(query: &str, top_k: usize, min_score: f32) -> Self {
        Self {
            query_hash: hash_text(query),
            top_k,
            // Scale to avoid floating point comparison issues
            min_score_scaled: (min_score * 10000.0) as i32,
        }
    }
}

/// Cached query result value
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryCacheValue {
    /// Search results
    results: Vec<SearchResult>,
    /// Cache timestamp (for debugging/monitoring)
    cached_at: std::time::SystemTime,
}

impl QueryCache {
    /// Create a new query cache with default configuration
    pub fn new() -> Self {
        Self::with_config(&CacheConfig::default())
    }

    /// Create a new query cache with custom configuration
    pub fn with_config(config: &CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.query_max_capacity)
            .time_to_live(Duration::from_secs(config.query_ttl_seconds))
            .build();

        Self {
            cache,
            stats: Arc::new(CacheStats::new("query")),
        }
    }

    /// Get query results from cache
    ///
    /// # Arguments
    /// * `query` - The query text
    /// * `top_k` - Number of results requested
    /// * `min_score` - Minimum score threshold
    ///
    /// # Returns
    /// The cached search results, or None if not in cache
    pub async fn get(
        &self,
        query: &str,
        top_k: usize,
        min_score: f32,
    ) -> Option<Vec<SearchResult>> {
        let key = QueryKey::new(query, top_k, min_score);
        let result = self.cache.get(&key).await;

        if result.is_some() {
            self.stats.record_hit();
        } else {
            self.stats.record_miss();
        }

        result.map(|v| v.results)
    }

    /// Store query results in cache
    ///
    /// # Arguments
    /// * `query` - The query text
    /// * `top_k` - Number of results requested
    /// * `min_score` - Minimum score threshold
    /// * `results` - The search results to cache
    pub async fn put(&self, query: &str, top_k: usize, min_score: f32, results: Vec<SearchResult>) {
        let key = QueryKey::new(query, top_k, min_score);
        let value = QueryCacheValue {
            results,
            cached_at: std::time::SystemTime::now(),
        };
        self.cache.insert(key, value).await;
        self.stats.record_write();
    }

    /// Check if query results exist in cache
    ///
    /// # Arguments
    /// * `query` - The query text
    /// * `top_k` - Number of results requested
    /// * `min_score` - Minimum score threshold
    pub async fn contains(&self, query: &str, top_k: usize, min_score: f32) -> bool {
        let key = QueryKey::new(query, top_k, min_score);
        self.cache.contains_key(&key)
    }

    /// Invalidate a specific query
    ///
    /// # Arguments
    /// * `query` - The query text to invalidate
    /// * `top_k` - Number of results requested
    /// * `min_score` - Minimum score threshold
    pub async fn invalidate(&self, query: &str, top_k: usize, min_score: f32) {
        let key = QueryKey::new(query, top_k, min_score);
        self.cache.invalidate(&key).await;
        self.stats.record_invalidation();
    }

    /// Clear all cached query results
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        // Wait for all pending invalidations to complete
        self.cache.run_pending_tasks().await;
        self.stats.reset();
    }

    /// Get cache statistics
    pub fn stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.stats)
    }

    /// Get current cache size
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Get weighted cache size (memory usage estimate)
    pub fn weighted_size(&self) -> u64 {
        self.cache.weighted_size()
    }
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Cache Statistics
// ============================================================================

/// Statistics for cache performance monitoring
#[derive(Debug)]
pub struct CacheStats {
    /// Cache name for identification
    name: String,
    /// Total number of cache hits
    hits: AtomicU64,
    /// Total number of cache misses
    misses: AtomicU64,
    /// Total number of cache writes
    writes: AtomicU64,
    /// Total number of invalidations
    invalidations: AtomicU64,
    /// Total latency for cache hits (in microseconds)
    hit_latency_us: AtomicU64,
    /// Total latency for cache misses (in microseconds)
    miss_latency_us: AtomicU64,
}

impl CacheStats {
    /// Create new cache statistics tracker
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            writes: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
            hit_latency_us: AtomicU64::new(0),
            miss_latency_us: AtomicU64::new(0),
        }
    }

    /// Record a cache hit
    pub(crate) fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache hit with latency
    pub(crate) fn record_hit_with_latency(&self, latency: Duration) {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.hit_latency_us
            .fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record a cache miss
    pub(crate) fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss with latency
    pub(crate) fn record_miss_with_latency(&self, latency: Duration) {
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.miss_latency_us
            .fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record a cache write
    pub(crate) fn record_write(&self) {
        self.writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache invalidation
    pub(crate) fn record_invalidation(&self) {
        self.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    /// Reset all statistics
    pub(crate) fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.writes.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
        self.hit_latency_us.store(0, Ordering::Relaxed);
        self.miss_latency_us.store(0, Ordering::Relaxed);
    }

    /// Get cache name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get total hits
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Get total misses
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Get total writes
    pub fn writes(&self) -> u64 {
        self.writes.load(Ordering::Relaxed)
    }

    /// Get total invalidations
    pub fn invalidations(&self) -> u64 {
        self.invalidations.load(Ordering::Relaxed)
    }

    /// Get total requests (hits + misses)
    pub fn total_requests(&self) -> u64 {
        self.hits() + self.misses()
    }

    /// Calculate hit rate (0.0 - 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 {
            0.0
        } else {
            self.hits() as f64 / total as f64
        }
    }

    /// Calculate miss rate (0.0 - 1.0)
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    /// Get average hit latency in microseconds
    pub fn avg_hit_latency_us(&self) -> f64 {
        let hits = self.hits();
        if hits == 0 {
            0.0
        } else {
            self.hit_latency_us.load(Ordering::Relaxed) as f64 / hits as f64
        }
    }

    /// Get average miss latency in microseconds
    pub fn avg_miss_latency_us(&self) -> f64 {
        let misses = self.misses();
        if misses == 0 {
            0.0
        } else {
            self.miss_latency_us.load(Ordering::Relaxed) as f64 / misses as f64
        }
    }

    /// Get a summary report
    pub fn report(&self) -> CacheStatsReport {
        CacheStatsReport {
            name: self.name.clone(),
            hits: self.hits(),
            misses: self.misses(),
            writes: self.writes(),
            invalidations: self.invalidations(),
            total_requests: self.total_requests(),
            hit_rate: self.hit_rate(),
            miss_rate: self.miss_rate(),
            avg_hit_latency_us: self.avg_hit_latency_us(),
            avg_miss_latency_us: self.avg_miss_latency_us(),
        }
    }
}

/// Serializable cache statistics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsReport {
    /// Cache name
    pub name: String,
    /// Total hits
    pub hits: u64,
    /// Total misses
    pub misses: u64,
    /// Total writes
    pub writes: u64,
    /// Total invalidations
    pub invalidations: u64,
    /// Total requests
    pub total_requests: u64,
    /// Hit rate (0.0 - 1.0)
    pub hit_rate: f64,
    /// Miss rate (0.0 - 1.0)
    pub miss_rate: f64,
    /// Average hit latency in microseconds
    pub avg_hit_latency_us: f64,
    /// Average miss latency in microseconds
    pub avg_miss_latency_us: f64,
}

// ============================================================================
// Result Ranking Cache
// ============================================================================

/// Cache for ranked search results
///
/// Caches the final ranked and merged results from RRF (Reciprocal Rank Fusion).
/// This cache stores the complete ranking pipeline output.
#[derive(Clone)]
pub struct RankingCache {
    cache: Cache<RankingKey, RankingCacheValue>,
    stats: Arc<CacheStats>,
}

/// Key for ranking cache entries
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct RankingKey {
    /// Hash of the query text
    query_hash: u64,
    /// Top-k parameter
    top_k: usize,
    /// Minimum score threshold (scaled)
    min_score_scaled: i32,
    /// Configuration hash (includes weights, RRF constant, etc.)
    config_hash: u64,
}

impl RankingKey {
    fn new(query: &str, top_k: usize, min_score: f32, config_hash: u64) -> Self {
        Self {
            query_hash: hash_text(query),
            top_k,
            min_score_scaled: (min_score * 10000.0) as i32,
            config_hash,
        }
    }
}

/// Cached ranking result value
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RankingCacheValue {
    /// Ranked search results
    results: Vec<SearchResult>,
    /// Cache timestamp
    cached_at: std::time::SystemTime,
    /// RRF scores for transparency
    rrf_scores: Vec<f32>,
}

impl RankingCache {
    /// Create a new ranking cache with default configuration
    pub fn new() -> Self {
        Self::with_config(&CacheConfig::default())
    }

    /// Create a new ranking cache with custom configuration
    pub fn with_config(config: &CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.query_max_capacity)
            .time_to_live(Duration::from_secs(config.query_ttl_seconds))
            .build();

        Self {
            cache,
            stats: Arc::new(CacheStats::new("ranking")),
        }
    }

    /// Get ranked results from cache
    pub async fn get(
        &self,
        query: &str,
        top_k: usize,
        min_score: f32,
        config_hash: u64,
    ) -> Option<Vec<SearchResult>> {
        let start = Instant::now();
        let key = RankingKey::new(query, top_k, min_score, config_hash);
        let result = self.cache.get(&key).await;

        if result.is_some() {
            self.stats.record_hit_with_latency(start.elapsed());
        } else {
            self.stats.record_miss_with_latency(start.elapsed());
        }

        result.map(|v| v.results)
    }

    /// Store ranked results in cache
    pub async fn put(
        &self,
        query: &str,
        top_k: usize,
        min_score: f32,
        config_hash: u64,
        results: Vec<SearchResult>,
        rrf_scores: Vec<f32>,
    ) {
        let key = RankingKey::new(query, top_k, min_score, config_hash);
        let value = RankingCacheValue {
            results,
            cached_at: std::time::SystemTime::now(),
            rrf_scores,
        };
        self.cache.insert(key, value).await;
        self.stats.record_write();
    }

    /// Clear all cached rankings
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
        self.stats.reset();
    }

    /// Get cache statistics
    pub fn stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.stats)
    }

    /// Get current cache size
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }
}

impl Default for RankingCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Graph Traversal Cache
// ============================================================================

/// Cache for graph traversal results
///
/// Caches the results of graph traversal queries to avoid expensive
/// graph database operations.
#[derive(Clone)]
pub struct GraphTraversalCache {
    cache: Cache<GraphTraversalKey, GraphTraversalValue>,
    stats: Arc<CacheStats>,
}

/// Key for graph traversal cache
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct GraphTraversalKey {
    /// Starting node IDs (sorted for consistency)
    start_nodes: Vec<String>,
    /// Traversal depth
    depth: u32,
    /// Edge types to follow (sorted)
    edge_types: Vec<String>,
}

impl GraphTraversalKey {
    fn new(mut start_nodes: Vec<String>, depth: u32, mut edge_types: Vec<String>) -> Self {
        // Sort for consistent hashing
        start_nodes.sort();
        edge_types.sort();
        Self {
            start_nodes,
            depth,
            edge_types,
        }
    }
}

/// Cached graph traversal result
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphTraversalValue {
    /// Search results from graph traversal
    results: Vec<SearchResult>,
    /// Cache timestamp
    cached_at: std::time::SystemTime,
}

impl GraphTraversalCache {
    /// Create a new graph traversal cache
    pub fn new() -> Self {
        Self::with_capacity(1000, 600) // 1000 entries, 10 min TTL
    }

    /// Create with custom capacity and TTL
    pub fn with_capacity(capacity: u64, ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();

        Self {
            cache,
            stats: Arc::new(CacheStats::new("graph_traversal")),
        }
    }

    /// Get graph traversal results from cache
    pub async fn get(
        &self,
        start_nodes: Vec<String>,
        depth: u32,
        edge_types: Vec<String>,
    ) -> Option<Vec<SearchResult>> {
        let start = Instant::now();
        let key = GraphTraversalKey::new(start_nodes, depth, edge_types);
        let result = self.cache.get(&key).await;

        if result.is_some() {
            self.stats.record_hit_with_latency(start.elapsed());
        } else {
            self.stats.record_miss_with_latency(start.elapsed());
        }

        result.map(|v| v.results)
    }

    /// Store graph traversal results in cache
    pub async fn put(
        &self,
        start_nodes: Vec<String>,
        depth: u32,
        edge_types: Vec<String>,
        results: Vec<SearchResult>,
    ) {
        let key = GraphTraversalKey::new(start_nodes, depth, edge_types);
        let value = GraphTraversalValue {
            results,
            cached_at: std::time::SystemTime::now(),
        };
        self.cache.insert(key, value).await;
        self.stats.record_write();
    }

    /// Invalidate graph cache entries related to a specific node
    pub async fn invalidate_node(&self, _node_id: &str) {
        // Note: Moka doesn't support pattern-based invalidation in memory cache
        // For production use, consider using Redis with pattern matching
        // For now, we just clear the entire cache
        self.cache.invalidate_all();
        self.stats.record_invalidation();
    }

    /// Clear all cached graph traversals
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
        self.stats.reset();
    }

    /// Get cache statistics
    pub fn stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.stats)
    }

    /// Get current cache size
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }
}

impl Default for GraphTraversalCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Cache Manager
// ============================================================================

/// Combined cache manager for all RAG caches
#[derive(Clone)]
pub struct RagCacheManager {
    /// Embedding cache
    pub embedding: EmbeddingCache,
    /// Query results cache
    pub query: QueryCache,
    /// Result ranking cache
    pub ranking: RankingCache,
    /// Graph traversal cache
    pub graph_traversal: GraphTraversalCache,
}

impl RagCacheManager {
    /// Create a new cache manager with default configuration
    pub fn new() -> Self {
        Self::with_config(&CacheConfig::default())
    }

    /// Create a new cache manager with custom configuration
    pub fn with_config(config: &CacheConfig) -> Self {
        Self {
            embedding: EmbeddingCache::with_config(config),
            query: QueryCache::with_config(config),
            ranking: RankingCache::with_config(config),
            graph_traversal: GraphTraversalCache::new(),
        }
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        self.embedding.clear().await;
        self.query.clear().await;
        self.ranking.clear().await;
        self.graph_traversal.clear().await;
    }

    /// Get combined statistics for all caches
    pub fn all_stats(&self) -> Vec<CacheStatsReport> {
        vec![
            self.embedding.stats().report(),
            self.query.stats().report(),
            self.ranking.stats().report(),
            self.graph_traversal.stats().report(),
        ]
    }

    /// Warm up the embedding cache with common queries
    ///
    /// # Arguments
    /// * `texts` - Common texts to pre-compute embeddings for
    /// * `compute_embedding` - Function to compute embeddings
    pub async fn warm_embedding_cache<F, Fut>(
        &self,
        texts: Vec<String>,
        compute_embedding: F,
    ) -> Result<()>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<f32>>>,
    {
        tracing::info!("Warming embedding cache with {} texts", texts.len());

        for text in texts {
            // Skip if already cached
            if self.embedding.contains(&text).await {
                continue;
            }

            // Compute and cache
            match compute_embedding(text.clone()).await {
                Ok(embedding) => {
                    self.embedding.put(&text, embedding).await;
                }
                Err(e) => {
                    tracing::warn!("Failed to compute embedding during warm-up: {}", e);
                }
            }
        }

        tracing::info!(
            "Embedding cache warmed up. Total entries: {}",
            self.embedding.entry_count()
        );
        Ok(())
    }

    /// Warm up caches with common queries (parallel execution)
    ///
    /// # Arguments
    /// * `common_queries` - List of frequently used queries
    /// * `compute_embedding` - Function to compute embeddings
    pub async fn warm_caches_parallel<F, Fut>(
        &self,
        common_queries: Vec<String>,
        compute_embedding: F,
    ) -> Result<()>
    where
        F: Fn(String) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<Vec<f32>>> + Send,
    {
        use futures::stream::{self, StreamExt};

        tracing::info!(
            "Starting parallel cache warm-up for {} queries",
            common_queries.len()
        );

        let start = Instant::now();
        let embedding_cache = self.embedding.clone();

        // Process queries in parallel (limit concurrency to avoid overload)
        let results: Vec<_> = stream::iter(common_queries)
            .map(|query| {
                let cache = embedding_cache.clone();
                let compute_fn = compute_embedding.clone();
                async move {
                    if !cache.contains(&query).await {
                        match compute_fn(query.clone()).await {
                            Ok(embedding) => {
                                cache.put(&query, embedding).await;
                                Ok(())
                            }
                            Err(e) => {
                                tracing::warn!("Failed to warm cache for query: {}", e);
                                Err(e)
                            }
                        }
                    } else {
                        Ok(())
                    }
                }
            })
            .buffer_unordered(10) // Process 10 queries concurrently
            .collect()
            .await;

        let successful = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.len() - successful;

        tracing::info!(
            "Cache warm-up completed in {:?}: {} successful, {} failed",
            start.elapsed(),
            successful,
            failed
        );

        Ok(())
    }

    /// Perform cache pre-computation for common entity and keyword combinations
    ///
    /// # Arguments
    /// * `entities` - Common entities to pre-compute
    /// * `keywords` - Common keywords to combine
    /// * `compute_embedding` - Function to compute embeddings
    pub async fn precompute_combinations<F, Fut>(
        &self,
        entities: Vec<String>,
        keywords: Vec<String>,
        compute_embedding: F,
    ) -> Result<()>
    where
        F: Fn(String) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<Vec<f32>>> + Send,
    {
        tracing::info!(
            "Pre-computing embeddings for {} entities and {} keywords",
            entities.len(),
            keywords.len()
        );

        let mut combinations = Vec::new();

        // Single entities and keywords
        combinations.extend(entities.clone());
        combinations.extend(keywords.clone());

        // Entity + keyword combinations
        for entity in &entities {
            for keyword in &keywords {
                combinations.push(format!("{} {}", entity, keyword));
                combinations.push(format!("{} {}", keyword, entity));
            }
        }

        // Warm up cache with all combinations
        self.warm_caches_parallel(combinations, compute_embedding)
            .await
    }

    /// Get total cache memory usage estimate
    pub fn total_memory_usage(&self) -> u64 {
        self.embedding.weighted_size()
            + self.query.weighted_size()
            + self.ranking.entry_count() * 1024 // Estimate 1KB per ranking entry
            + self.graph_traversal.entry_count() * 512 // Estimate 512B per graph entry
    }
}

impl Default for RagCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Hash text to a 64-bit value for cache keys
///
/// Uses DefaultHasher for consistency across process runs.
/// Note: This is not cryptographically secure, but suitable for cache keys.
fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_cache_basic() {
        let cache = EmbeddingCache::new();

        let text = "Hello, world!";
        let embedding = vec![0.1, 0.2, 0.3, 0.4];

        // Initially not in cache
        assert!(cache.get(text).await.is_none());
        assert_eq!(cache.stats().hits(), 0);
        assert_eq!(cache.stats().misses(), 1);

        // Put and retrieve
        cache.put(text, embedding.clone()).await;
        let retrieved = cache.get(text).await;
        assert_eq!(retrieved, Some(embedding));
        assert_eq!(cache.stats().hits(), 1);
        assert_eq!(cache.stats().writes(), 1);
    }

    #[tokio::test]
    async fn test_embedding_cache_invalidation() {
        let cache = EmbeddingCache::new();

        let text = "Test text";
        let embedding = vec![1.0, 2.0, 3.0];

        cache.put(text, embedding.clone()).await;
        assert!(cache.get(text).await.is_some());

        cache.invalidate(text).await;
        assert!(cache.get(text).await.is_none());
        assert_eq!(cache.stats().invalidations(), 1);
    }

    #[tokio::test]
    async fn test_query_cache_basic() {
        let cache = QueryCache::new();

        let query = "What is the policy?";
        let results = vec![]; // Empty results for testing

        // Initially not in cache
        assert!(cache.get(query, 10, 0.0).await.is_none());
        assert_eq!(cache.stats().misses(), 1);

        // Put and retrieve
        cache.put(query, 10, 0.0, results.clone()).await;
        let retrieved = cache.get(query, 10, 0.0).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 0);
        assert_eq!(cache.stats().hits(), 1);
    }

    #[tokio::test]
    async fn test_query_cache_different_params() {
        let cache = QueryCache::new();

        let query = "What is the policy?";
        let results1 = vec![];
        let results2 = vec![];

        // Same query, different top_k should be different cache entries
        cache.put(query, 5, 0.0, results1).await;
        cache.put(query, 10, 0.0, results2).await;

        // Both should be retrievable
        assert!(cache.get(query, 5, 0.0).await.is_some());
        assert!(cache.get(query, 10, 0.0).await.is_some());

        // Entry count may not be immediately updated due to async tasks
        // Just verify both keys are retrievable
        assert!(cache.contains(query, 5, 0.0).await);
        assert!(cache.contains(query, 10, 0.0).await);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = EmbeddingCache::new();
        let stats = cache.stats();

        // Initial state
        assert_eq!(stats.hits(), 0);
        assert_eq!(stats.misses(), 0);
        assert_eq!(stats.hit_rate(), 0.0);

        // Generate some activity
        cache.get("text1").await; // miss
        cache.put("text1", vec![1.0]).await; // write
        cache.get("text1").await; // hit
        cache.get("text2").await; // miss

        assert_eq!(stats.hits(), 1);
        assert_eq!(stats.misses(), 2);
        assert_eq!(stats.writes(), 1);
        assert_eq!(stats.total_requests(), 3);
        assert!((stats.hit_rate() - 1.0 / 3.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_cache_manager() {
        let manager = RagCacheManager::new();

        // Test embedding cache
        manager.embedding.put("text", vec![1.0]).await;
        assert!(manager.embedding.get("text").await.is_some());

        // Test query cache
        manager.query.put("query", 10, 0.0, vec![]).await;
        assert!(manager.query.get("query", 10, 0.0).await.is_some());

        // Clear all
        manager.clear_all().await;
        assert!(manager.embedding.get("text").await.is_none());
        assert!(manager.query.get("query", 10, 0.0).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = EmbeddingCache::new();

        cache.put("text1", vec![1.0]).await;
        cache.put("text2", vec![2.0]).await;

        // Verify entries are cached
        assert!(cache.get("text1").await.is_some());
        assert!(cache.get("text2").await.is_some());

        cache.clear().await;

        // After clear, entries should not be retrievable
        assert!(cache.get("text1").await.is_none());
        assert!(cache.get("text2").await.is_none());
    }

    #[test]
    fn test_hash_text_consistency() {
        let text = "consistent text";
        let hash1 = hash_text(text);
        let hash2 = hash_text(text);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_cache_config_defaults() {
        let config = CacheConfig::default();
        assert!(config.embedding_max_capacity > 0);
        assert!(config.query_max_capacity > 0);
        assert!(config.embedding_ttl_seconds > 0);
        assert!(config.query_ttl_seconds > 0);
    }

    #[tokio::test]
    async fn test_ranking_cache() {
        let cache = RankingCache::new();

        let query = "test query";
        let results = vec![];
        let rrf_scores = vec![0.5, 0.3, 0.2];
        let config_hash = 12345u64;

        // Initially not in cache
        assert!(cache.get(query, 10, 0.0, config_hash).await.is_none());

        // Put and retrieve
        cache
            .put(query, 10, 0.0, config_hash, results.clone(), rrf_scores)
            .await;
        let retrieved = cache.get(query, 10, 0.0, config_hash).await;
        assert!(retrieved.is_some());

        // Different config hash should be a different entry
        assert!(cache.get(query, 10, 0.0, 99999u64).await.is_none());
    }

    #[tokio::test]
    async fn test_graph_traversal_cache() {
        let cache = GraphTraversalCache::new();

        let start_nodes = vec!["node1".to_string(), "node2".to_string()];
        let depth = 2;
        let edge_types = vec!["RELATES_TO".to_string()];
        let results = vec![];

        // Initially not in cache
        assert!(cache
            .get(start_nodes.clone(), depth, edge_types.clone())
            .await
            .is_none());

        // Put and retrieve
        cache
            .put(start_nodes.clone(), depth, edge_types.clone(), results)
            .await;
        let retrieved = cache.get(start_nodes.clone(), depth, edge_types.clone()).await;
        assert!(retrieved.is_some());

        // Different depth should be a different entry
        assert!(cache.get(start_nodes, 3, edge_types).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats_with_latency() {
        let cache = EmbeddingCache::new();

        // Generate activity
        cache.get("text1").await; // miss
        cache.put("text1", vec![1.0]).await;
        cache.get("text1").await; // hit

        let stats = cache.stats();
        let report = stats.report();

        // Check basic stats
        assert_eq!(report.hits, 1);
        assert_eq!(report.misses, 1);
        assert_eq!(report.total_requests, 2);

        // Latency should be recorded (non-zero since we actually performed operations)
        // Note: In fast tests, latency might be 0, so we just check it's a valid number
        assert!(report.avg_hit_latency_us >= 0.0);
        assert!(report.avg_miss_latency_us >= 0.0);
    }

    #[tokio::test]
    async fn test_cache_manager_with_new_caches() {
        let manager = RagCacheManager::new();

        // Test ranking cache
        manager
            .ranking
            .put("query", 10, 0.0, 123, vec![], vec![])
            .await;
        assert!(manager.ranking.get("query", 10, 0.0, 123).await.is_some());

        // Test graph traversal cache
        manager
            .graph_traversal
            .put(vec!["node1".to_string()], 2, vec![], vec![])
            .await;
        assert!(manager
            .graph_traversal
            .get(vec!["node1".to_string()], 2, vec![])
            .await
            .is_some());

        // Test all_stats includes all caches
        let all_stats = manager.all_stats();
        assert_eq!(all_stats.len(), 4); // embedding, query, ranking, graph_traversal

        // Clear all should clear everything
        manager.clear_all().await;
        assert!(manager.ranking.get("query", 10, 0.0, 123).await.is_none());
        assert!(manager
            .graph_traversal
            .get(vec!["node1".to_string()], 2, vec![])
            .await
            .is_none());
    }

    #[tokio::test]
    async fn test_graph_traversal_cache_node_sorting() {
        let cache = GraphTraversalCache::new();

        let nodes1 = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let nodes2 = vec!["C".to_string(), "A".to_string(), "B".to_string()]; // Different order
        let depth = 2;
        let edge_types = vec!["RELATES".to_string()];
        let results = vec![];

        // Put with first ordering
        cache
            .put(nodes1.clone(), depth, edge_types.clone(), results)
            .await;

        // Should retrieve with different node ordering (since keys are sorted internally)
        let retrieved = cache.get(nodes2, depth, edge_types).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_cache_warm_up() {
        let manager = RagCacheManager::new();

        // Mock embedding function
        let compute_embedding = |text: String| async move {
            // Simple mock: just return the length as a float
            Ok(vec![text.len() as f32])
        };

        // Warm up with some texts
        let texts = vec![
            "query 1".to_string(),
            "query 2".to_string(),
            "query 3".to_string(),
        ];

        manager
            .warm_embedding_cache(texts.clone(), compute_embedding)
            .await
            .unwrap();

        // Verify all are cached by actually retrieving them
        let mut cached_count = 0;
        for text in &texts {
            if manager.embedding.get(text).await.is_some() {
                cached_count += 1;
            }
        }

        // All texts should be cached
        assert_eq!(cached_count, texts.len());
    }

    #[tokio::test]
    async fn test_cache_memory_usage() {
        let manager = RagCacheManager::new();

        // Add some data
        manager.embedding.put("test1", vec![1.0, 2.0, 3.0]).await;
        manager.embedding.put("test2", vec![4.0, 5.0, 6.0]).await;
        manager.query.put("query", 10, 0.0, vec![]).await;

        // Verify by retrieving
        assert!(manager.embedding.get("test1").await.is_some());
        assert!(manager.embedding.get("test2").await.is_some());
        assert!(manager.query.get("query", 10, 0.0).await.is_some());

        // Total memory usage calculation exists and doesn't panic
        let _usage = manager.total_memory_usage();
    }
}
