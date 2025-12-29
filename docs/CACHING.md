# OTL Caching System

## Overview

The OTL RAG system implements a comprehensive multi-tier caching architecture to optimize performance and reduce latency. The caching system supports both in-memory (L1) and distributed Redis-based (L2) caching.

## Architecture

```
Request → Cache Check
  ├─ L1: In-memory (moka - hot data, ultra-fast)
  ├─ L2: Redis (distributed, shared across instances)
  └─ Miss: Compute → Cache → Return
```

## Cache Types

### 1. Embedding Cache

Caches document embeddings to avoid recomputing expensive vector transformations.

- **L1 TTL**: 1 hour (configurable)
- **Capacity**: 10,000 entries (default)
- **Average entry size**: ~1.5KB
- **Use case**: Frequently queried documents, common search terms

```rust
use otl_rag::RagCacheManager;

let cache_manager = RagCacheManager::new();

// Get cached embedding
if let Some(embedding) = cache_manager.embedding.get("document text").await {
    // Use cached embedding
} else {
    // Compute embedding
    let embedding = compute_embedding("document text").await?;
    cache_manager.embedding.put("document text", embedding.clone()).await;
}
```

### 2. Query Results Cache

Caches complete RAG query results including search results and rankings.

- **L1 TTL**: 5 minutes (default)
- **Capacity**: 1,000 entries
- **Use case**: Repeated queries, common user questions

```rust
// Check cache before executing full RAG pipeline
if let Some(results) = cache_manager.query.get("query text", 10, 0.0).await {
    return results;
}
```

### 3. Ranking Cache (NEW)

Caches RRF (Reciprocal Rank Fusion) ranked and merged results.

- **Features**:
  - Stores final ranked results with RRF scores
  - Configuration-aware (different weights = different cache keys)
  - Includes transparency scores for explainability

```rust
let config_hash = compute_config_hash(&rag_config);

// Try cache first
if let Some(ranked_results) = cache_manager.ranking.get(
    query,
    top_k,
    min_score,
    config_hash
).await {
    return ranked_results;
}
```

### 4. Graph Traversal Cache (NEW)

Caches expensive graph database traversal results.

- **TTL**: 10 minutes (default)
- **Features**:
  - Node-order independent (sorted keys)
  - Depth-aware caching
  - Edge type filtering support

```rust
let results = cache_manager.graph_traversal.get(
    vec!["node1".to_string(), "node2".to_string()],
    depth: 2,
    vec!["RELATES_TO".to_string()]
).await;
```

## Redis Integration (L2 Cache)

### Enabling Redis

Redis support is optional and enabled via feature flag:

```toml
[dependencies]
otl-rag = { version = "0.1", features = ["redis-cache"] }
```

### Configuration

```rust
use otl_rag::redis_cache::{RedisCache, RedisCacheConfig};

let redis_config = RedisCacheConfig {
    url: "redis://127.0.0.1:6379".to_string(),
    key_prefix: "otl:cache".to_string(),
    default_ttl: 3600, // 1 hour
    max_retries: 3,
    connection_timeout_ms: 5000,
};

let redis_cache = RedisCache::new(redis_config).await?;
```

### Two-Tier Cache Usage

```rust
use otl_rag::redis_cache::TwoTierCache;
use std::time::Duration;

// Create L1 + L2 cache
let cache = TwoTierCache::new(
    "embeddings",           // Cache name
    10_000,                 // L1 capacity
    Duration::from_secs(3600), // L1 TTL
    Some(redis_cache),      // L2 (optional)
);

// Get (checks L1 → L2 → miss)
if let Some(value) = cache.get("key").await {
    // L1 hit (ultra-fast) or L2 hit (fast, promoted to L1)
}

// Put (writes to both L1 and L2)
cache.put("key", value, Some(3600)).await;
```

### Graceful Fallback

Redis is optional. If Redis is unavailable or disabled:
- System falls back to L1 (in-memory) only
- No errors or service interruption
- Reduced cache sharing across instances

## Cache Warming Strategies

### 1. Sequential Warm-up

```rust
let common_queries = vec![
    "What is our vacation policy?".to_string(),
    "How do I submit expenses?".to_string(),
    "What are the working hours?".to_string(),
];

cache_manager.warm_embedding_cache(
    common_queries,
    compute_embedding
).await?;
```

### 2. Parallel Warm-up (NEW)

Efficiently pre-compute embeddings in parallel:

```rust
cache_manager.warm_caches_parallel(
    common_queries,
    compute_embedding
).await?;
```

Features:
- Concurrent processing (10 parallel tasks by default)
- Skip already-cached entries
- Error tolerance (continues on failures)
- Progress logging

### 3. Entity-Keyword Pre-computation (NEW)

Generate and cache common entity-keyword combinations:

```rust
let entities = vec!["vacation".to_string(), "expense".to_string()];
let keywords = vec!["policy".to_string(), "submit".to_string()];

cache_manager.precompute_combinations(
    entities,
    keywords,
    compute_embedding
).await?;
```

Generates:
- Individual entities: "vacation", "expense"
- Individual keywords: "policy", "submit"
- Combinations: "vacation policy", "policy vacation", etc.

## Cache Invalidation

### TTL-Based Invalidation (Automatic)

Caches automatically expire based on configured TTL:
- **Embedding cache**: 1 hour (stable data)
- **Query cache**: 5 minutes (may change with updates)
- **Ranking cache**: 5 minutes
- **Graph traversal**: 10 minutes

### LRU Eviction (Automatic)

When cache reaches max capacity, least recently used entries are evicted.

### Manual Invalidation

```rust
// Invalidate specific entry
cache_manager.embedding.invalidate("text").await;

// Clear all caches
cache_manager.clear_all().await;

// Clear specific cache type
cache_manager.ranking.clear().await;

// Redis pattern-based invalidation
redis_cache.invalidate_pattern("embedding:*").await?;
```

### Smart Invalidation on Updates

When documents are updated:

```rust
async fn on_document_update(document_id: &str, cache_manager: &RagCacheManager) {
    // Invalidate related embeddings
    cache_manager.embedding.invalidate(document_id).await;

    // Clear query cache (results may have changed)
    cache_manager.query.clear().await;

    // Clear ranking cache
    cache_manager.ranking.clear().await;

    // Invalidate graph traversals involving this document
    cache_manager.graph_traversal.invalidate_node(document_id).await;
}
```

## Cache Metrics

### Available Metrics

All caches track comprehensive performance metrics:

- **Hits**: Successful cache retrievals
- **Misses**: Cache lookups that failed
- **Writes**: New entries added to cache
- **Invalidations**: Manual cache entry removals
- **Hit Rate**: Percentage of successful lookups
- **Miss Rate**: Percentage of failed lookups
- **Hit Latency**: Average time for cache hits (microseconds)
- **Miss Latency**: Average time for cache misses (microseconds)

### Accessing Metrics

```rust
// Single cache stats
let stats = cache_manager.embedding.stats();
println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
println!("Avg hit latency: {:.2}µs", stats.avg_hit_latency_us());

// All caches report
let all_stats = cache_manager.all_stats();
for report in all_stats {
    println!("Cache: {}", report.name);
    println!("  Hits: {}", report.hits);
    println!("  Misses: {}", report.misses);
    println!("  Hit rate: {:.2}%", report.hit_rate * 100.0);
    println!("  Avg hit latency: {:.2}µs", report.avg_hit_latency_us);
}

// Memory usage estimate
let total_memory = cache_manager.total_memory_usage();
println!("Total cache memory: {} bytes", total_memory);
```

### Exposing Metrics to Monitoring

```rust
use serde_json;

// Serialize metrics for Prometheus, Grafana, etc.
let metrics_json = serde_json::to_string(&cache_manager.all_stats())?;
```

## Performance Best Practices

### 1. Configure Appropriate TTLs

- **Stable data** (embeddings): Longer TTL (1 hour+)
- **Dynamic data** (query results): Shorter TTL (5 minutes)
- **Balance**: Memory usage vs. freshness

### 2. Use Redis for Distributed Deployments

- Share cache across multiple instances
- Reduce redundant computations
- Better resource utilization

### 3. Implement Cache Warming

- Pre-compute common queries on startup
- Warm cache during off-peak hours
- Use parallel warming for large datasets

### 4. Monitor Cache Performance

```rust
// Log cache statistics periodically
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let stats = cache_manager.all_stats();
        for report in stats {
            tracing::info!(
                "Cache {}: hit_rate={:.2}%, hits={}, misses={}",
                report.name,
                report.hit_rate * 100.0,
                report.hits,
                report.misses
            );
        }
    }
});
```

### 5. Tune Cache Sizes

Based on your workload:

```rust
use otl_rag::CacheConfig;

let config = CacheConfig {
    embedding_max_capacity: 50_000,  // Increase for large datasets
    query_max_capacity: 5_000,        // Increase for high query volume
    embedding_ttl_seconds: 7200,      // 2 hours for stable embeddings
    query_ttl_seconds: 300,           // 5 minutes for queries
    enable_stats: true,
};

let cache_manager = RagCacheManager::with_config(&config);
```

## Example: Full Integration

```rust
use otl_rag::{RagCacheManager, CacheConfig};
use otl_rag::redis_cache::{RedisCache, RedisCacheConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Configure caching
    let cache_config = CacheConfig {
        embedding_max_capacity: 20_000,
        query_max_capacity: 2_000,
        embedding_ttl_seconds: 3600,
        query_ttl_seconds: 300,
        enable_stats: true,
    };

    let cache_manager = RagCacheManager::with_config(&cache_config);

    // Optional: Enable Redis L2 cache
    #[cfg(feature = "redis-cache")]
    {
        let redis_config = RedisCacheConfig::default();
        let redis_cache = RedisCache::new(redis_config).await?;
        // Use two-tier caches if needed
    }

    // Warm up common queries
    let common_queries = load_common_queries();
    cache_manager.warm_caches_parallel(
        common_queries,
        |text| compute_embedding(text)
    ).await?;

    // Start metrics reporting
    start_metrics_reporter(cache_manager.clone());

    // Use in RAG pipeline
    let embedding = match cache_manager.embedding.get("query").await {
        Some(cached) => cached,
        None => {
            let computed = compute_embedding("query").await?;
            cache_manager.embedding.put("query", computed.clone()).await;
            computed
        }
    };

    Ok(())
}
```

## Migration Guide

### From Old Cache to New Cache

Old code:
```rust
let cache = EmbeddingCache::new();
```

New code (unchanged API, extended features):
```rust
let cache = EmbeddingCache::new(); // Still works!
// But now you get latency metrics automatically
```

### Adding New Cache Types

Old code:
```rust
let manager = RagCacheManager::new();
// Only embedding and query caches
```

New code:
```rust
let manager = RagCacheManager::new();
// Now includes:
// - manager.embedding
// - manager.query
// - manager.ranking (NEW)
// - manager.graph_traversal (NEW)
```

## Troubleshooting

### High Miss Rate

**Symptoms**: Cache hit rate < 20%

**Solutions**:
- Increase cache capacity
- Extend TTL for stable data
- Implement cache warming
- Check query patterns (high variability?)

### Memory Usage Too High

**Symptoms**: High memory consumption

**Solutions**:
- Reduce cache capacity
- Shorten TTL
- Enable Redis L2 (move data out of memory)
- Monitor with `total_memory_usage()`

### Redis Connection Issues

**Symptoms**: Redis errors in logs

**Solutions**:
- Verify Redis is running
- Check connection URL
- Increase `connection_timeout_ms`
- System gracefully falls back to L1 only

### Stale Cache Data

**Symptoms**: Outdated results returned

**Solutions**:
- Reduce TTL for affected cache type
- Implement invalidation on document updates
- Use manual invalidation for critical updates
