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
use std::time::Duration;

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
}

impl CacheStats {
    /// Create new cache statistics tracker
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            writes: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
        }
    }

    /// Record a cache hit
    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache write
    fn record_write(&self) {
        self.writes.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache invalidation
    fn record_invalidation(&self) {
        self.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    /// Reset all statistics
    fn reset(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.writes.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
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
        }
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        self.embedding.clear().await;
        self.query.clear().await;
    }

    /// Get combined statistics for all caches
    pub fn all_stats(&self) -> Vec<CacheStatsReport> {
        vec![self.embedding.stats().report(), self.query.stats().report()]
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
}
