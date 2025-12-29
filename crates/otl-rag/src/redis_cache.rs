//! Redis-based distributed caching layer (L2 cache)
//!
//! Provides distributed caching support using Redis for:
//! - Embeddings
//! - Query results
//! - Graph traversal results
//! - Result rankings
//!
//! This module is optional and only compiled when the `redis-cache` feature is enabled.
//!
//! Author: hephaex@gmail.com

#[cfg(feature = "redis-cache")]
use redis::aio::ConnectionManager;
#[cfg(feature = "redis-cache")]
use redis::{AsyncCommands, RedisError, RedisResult};
#[cfg(feature = "redis-cache")]
use serde::{de::DeserializeOwned, Serialize};
#[cfg(feature = "redis-cache")]
use std::sync::Arc;
#[cfg(feature = "redis-cache")]
use std::time::Duration;

// ============================================================================
// Redis Cache Configuration
// ============================================================================

#[cfg(feature = "redis-cache")]
#[derive(Debug, Clone)]
pub struct RedisCacheConfig {
    /// Redis connection URL (e.g., "redis://127.0.0.1:6379")
    pub url: String,

    /// Key prefix for all cache entries
    pub key_prefix: String,

    /// Default TTL for cache entries (in seconds)
    pub default_ttl: u64,

    /// Maximum number of connection retries
    pub max_retries: u32,

    /// Connection timeout (in milliseconds)
    pub connection_timeout_ms: u64,
}

#[cfg(feature = "redis-cache")]
impl Default for RedisCacheConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            key_prefix: "otl:cache".to_string(),
            default_ttl: 3600, // 1 hour
            max_retries: 3,
            connection_timeout_ms: 5000,
        }
    }
}

// ============================================================================
// Redis Cache Client
// ============================================================================

#[cfg(feature = "redis-cache")]
#[derive(Clone)]
pub struct RedisCache {
    connection: Arc<ConnectionManager>,
    config: RedisCacheConfig,
    stats: Arc<super::cache::CacheStats>,
}

#[cfg(feature = "redis-cache")]
impl RedisCache {
    /// Create a new Redis cache client
    pub async fn new(config: RedisCacheConfig) -> Result<Self, RedisError> {
        let client = redis::Client::open(config.url.as_str())?;
        let connection = ConnectionManager::new(client).await?;

        Ok(Self {
            connection: Arc::new(connection),
            config,
            stats: Arc::new(super::cache::CacheStats::new("redis")),
        })
    }

    /// Get a value from Redis cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    ///
    /// # Returns
    /// The cached value, or None if not in cache or on error
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let full_key = self.make_key(key);
        let start = std::time::Instant::now();

        let mut conn = self.connection.clone();
        let result: RedisResult<Vec<u8>> = conn.get(&full_key).await;

        match result {
            Ok(bytes) => {
                if let Ok(value) = bincode::deserialize::<T>(&bytes) {
                    self.stats.record_hit();
                    tracing::debug!(
                        "Redis cache hit: {} (latency: {:?})",
                        key,
                        start.elapsed()
                    );
                    Some(value)
                } else {
                    tracing::warn!("Failed to deserialize Redis cache value for key: {}", key);
                    self.stats.record_miss();
                    None
                }
            }
            Err(e) => {
                if !matches!(e.kind(), redis::ErrorKind::TypeError) {
                    tracing::debug!("Redis cache miss: {} (error: {})", key, e);
                }
                self.stats.record_miss();
                None
            }
        }
    }

    /// Store a value in Redis cache
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `value` - The value to cache
    /// * `ttl` - Optional TTL in seconds (uses default if None)
    pub async fn put<T: Serialize>(&self, key: &str, value: &T, ttl: Option<u64>) -> bool {
        let full_key = self.make_key(key);
        let ttl_seconds = ttl.unwrap_or(self.config.default_ttl);

        let bytes = match bincode::serialize(value) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to serialize value for Redis cache: {}", e);
                return false;
            }
        };

        let mut conn = self.connection.clone();
        let result: RedisResult<()> = conn
            .set_ex(&full_key, bytes, ttl_seconds as usize)
            .await;

        match result {
            Ok(_) => {
                self.stats.record_write();
                tracing::debug!("Redis cache write: {} (ttl: {}s)", key, ttl_seconds);
                true
            }
            Err(e) => {
                tracing::error!("Failed to write to Redis cache: {}", e);
                false
            }
        }
    }

    /// Check if a key exists in cache
    pub async fn contains(&self, key: &str) -> bool {
        let full_key = self.make_key(key);
        let mut conn = self.connection.clone();

        match conn.exists::<_, bool>(&full_key).await {
            Ok(exists) => exists,
            Err(_) => false,
        }
    }

    /// Invalidate a specific cache entry
    pub async fn invalidate(&self, key: &str) -> bool {
        let full_key = self.make_key(key);
        let mut conn = self.connection.clone();

        match conn.del::<_, i32>(&full_key).await {
            Ok(_) => {
                self.stats.record_invalidation();
                tracing::debug!("Redis cache invalidation: {}", key);
                true
            }
            Err(e) => {
                tracing::error!("Failed to invalidate Redis cache entry: {}", e);
                false
            }
        }
    }

    /// Invalidate all cache entries matching a pattern
    ///
    /// # Arguments
    /// * `pattern` - Key pattern (e.g., "embedding:*")
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<u64, RedisError> {
        let full_pattern = self.make_key(pattern);
        let mut conn = self.connection.clone();

        // Get all keys matching the pattern
        let keys: Vec<String> = conn.keys(&full_pattern).await?;

        if keys.is_empty() {
            return Ok(0);
        }

        // Delete all matching keys
        let deleted: i32 = conn.del(&keys).await?;

        tracing::debug!(
            "Redis cache pattern invalidation: {} ({} keys deleted)",
            pattern,
            deleted
        );

        Ok(deleted as u64)
    }

    /// Clear all cache entries with the configured prefix
    pub async fn clear(&self) -> Result<u64, RedisError> {
        self.invalidate_pattern("*").await
    }

    /// Get cache statistics
    pub fn stats(&self) -> Arc<super::cache::CacheStats> {
        Arc::clone(&self.stats)
    }

    /// Ping the Redis server to check connectivity
    pub async fn ping(&self) -> bool {
        let mut conn = self.connection.clone();
        match redis::cmd("PING").query_async::<_, String>(&mut *conn).await {
            Ok(response) => response == "PONG",
            Err(_) => false,
        }
    }

    /// Get the time-to-live (TTL) for a key
    pub async fn ttl(&self, key: &str) -> Option<i64> {
        let full_key = self.make_key(key);
        let mut conn = self.connection.clone();

        match conn.ttl::<_, i64>(&full_key).await {
            Ok(ttl) => {
                if ttl >= 0 {
                    Some(ttl)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Make a full cache key with prefix
    fn make_key(&self, key: &str) -> String {
        format!("{}:{}", self.config.key_prefix, key)
    }
}

// ============================================================================
// Two-Tier Cache Layer (L1: Memory + L2: Redis)
// ============================================================================

#[cfg(feature = "redis-cache")]
#[derive(Clone)]
pub struct TwoTierCache<T>
where
    T: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// L1 cache (in-memory, fast)
    l1_cache: moka::future::Cache<String, T>,

    /// L2 cache (Redis, distributed)
    l2_cache: Option<RedisCache>,

    /// Cache name for metrics
    name: String,

    /// Combined statistics
    stats: Arc<super::cache::CacheStats>,
}

#[cfg(feature = "redis-cache")]
impl<T> TwoTierCache<T>
where
    T: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    /// Create a new two-tier cache
    ///
    /// # Arguments
    /// * `name` - Cache name for metrics
    /// * `l1_capacity` - Maximum entries in L1 cache
    /// * `l1_ttl` - TTL for L1 cache entries
    /// * `redis_cache` - Optional Redis cache for L2
    pub fn new(
        name: impl Into<String>,
        l1_capacity: u64,
        l1_ttl: Duration,
        redis_cache: Option<RedisCache>,
    ) -> Self {
        let l1_cache = moka::future::Cache::builder()
            .max_capacity(l1_capacity)
            .time_to_live(l1_ttl)
            .build();

        Self {
            l1_cache,
            l2_cache: redis_cache,
            name: name.into(),
            stats: Arc::new(super::cache::CacheStats::new(name)),
        }
    }

    /// Get a value from the cache (checks L1 then L2)
    pub async fn get(&self, key: &str) -> Option<T> {
        let start = std::time::Instant::now();

        // Check L1 cache first
        if let Some(value) = self.l1_cache.get(key).await {
            self.stats.record_hit();
            tracing::trace!(
                "L1 cache hit: {} (latency: {:?})",
                key,
                start.elapsed()
            );
            return Some(value);
        }

        // Check L2 cache (Redis)
        if let Some(ref redis) = self.l2_cache {
            if let Some(value) = redis.get::<T>(key).await {
                // Populate L1 cache
                self.l1_cache.insert(key.to_string(), value.clone()).await;
                self.stats.record_hit();
                tracing::debug!(
                    "L2 cache hit, promoted to L1: {} (latency: {:?})",
                    key,
                    start.elapsed()
                );
                return Some(value);
            }
        }

        self.stats.record_miss();
        tracing::trace!("Cache miss: {} (latency: {:?})", key, start.elapsed());
        None
    }

    /// Store a value in the cache (writes to both L1 and L2)
    pub async fn put(&self, key: &str, value: T, ttl: Option<u64>) {
        // Write to L1 cache
        self.l1_cache.insert(key.to_string(), value.clone()).await;

        // Write to L2 cache if available
        if let Some(ref redis) = self.l2_cache {
            redis.put(key, &value, ttl).await;
        }

        self.stats.record_write();
    }

    /// Check if a key exists in cache (checks L1 then L2)
    pub async fn contains(&self, key: &str) -> bool {
        if self.l1_cache.contains_key(key) {
            return true;
        }

        if let Some(ref redis) = self.l2_cache {
            return redis.contains(key).await;
        }

        false
    }

    /// Invalidate a cache entry (removes from both L1 and L2)
    pub async fn invalidate(&self, key: &str) {
        self.l1_cache.invalidate(key).await;

        if let Some(ref redis) = self.l2_cache {
            redis.invalidate(key).await;
        }

        self.stats.record_invalidation();
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        self.l1_cache.invalidate_all();
        self.l1_cache.run_pending_tasks().await;

        if let Some(ref redis) = self.l2_cache {
            let _ = redis.clear().await;
        }

        self.stats.reset();
    }

    /// Get cache statistics
    pub fn stats(&self) -> Arc<super::cache::CacheStats> {
        Arc::clone(&self.stats)
    }

    /// Get L1 cache entry count
    pub fn l1_entry_count(&self) -> u64 {
        self.l1_cache.entry_count()
    }

    /// Check if L2 cache is available
    pub fn has_l2(&self) -> bool {
        self.l2_cache.is_some()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "redis-cache"))]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        id: u32,
        value: String,
    }

    #[tokio::test]
    #[ignore] // Requires Redis server to be running
    async fn test_redis_cache_basic() {
        let config = RedisCacheConfig {
            url: "redis://127.0.0.1:6379".to_string(),
            key_prefix: "test".to_string(),
            ..Default::default()
        };

        let cache = RedisCache::new(config).await.unwrap();

        // Test ping
        assert!(cache.ping().await);

        // Test get/put
        let key = "test_key";
        let data = TestData {
            id: 1,
            value: "test value".to_string(),
        };

        assert!(cache.put(key, &data, Some(60)).await);
        let retrieved: Option<TestData> = cache.get(key).await;
        assert_eq!(retrieved, Some(data));

        // Test invalidation
        assert!(cache.invalidate(key).await);
        let after_invalidate: Option<TestData> = cache.get(key).await;
        assert_eq!(after_invalidate, None);
    }

    #[tokio::test]
    async fn test_two_tier_cache_l1_only() {
        let cache = TwoTierCache::<String>::new(
            "test",
            100,
            Duration::from_secs(60),
            None, // No Redis
        );

        let key = "test_key";
        let value = "test_value".to_string();

        // Initially not in cache
        assert_eq!(cache.get(key).await, None);

        // Put and retrieve
        cache.put(key, value.clone(), None).await;
        assert_eq!(cache.get(key).await, Some(value));

        // Invalidate
        cache.invalidate(key).await;
        assert_eq!(cache.get(key).await, None);
    }
}
