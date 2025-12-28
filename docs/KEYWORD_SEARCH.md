# Keyword Search Backend

Full-text search backend for hybrid RAG using PostgreSQL.

## Overview

The keyword search backend provides lexical search capabilities to complement vector similarity search in the hybrid RAG pipeline. It uses PostgreSQL's powerful full-text search features including:

- **tsvector** and **tsquery** for efficient full-text indexing
- **BM25-like scoring** via `ts_rank_cd` function
- **Phrase matching** for exact quote searches
- **Boolean operators** (AND, OR, NOT) for complex queries
- **Fuzzy matching** using trigram similarity for handling typos
- **Query analytics** and performance monitoring

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Hybrid RAG Pipeline                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Vector     │  │    Graph     │  │   Keyword    │      │
│  │   Search     │  │   Search     │  │   Search     │      │
│  │  (Semantic)  │  │ (Relational) │  │  (Lexical)   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                 │                  │              │
│         └─────────────────┴──────────────────┘              │
│                          │                                  │
│                 Reciprocal Rank Fusion (RRF)                │
│                          │                                  │
│                    Final Top-K Results                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Features

### 1. Full-Text Indexing

Documents are indexed using PostgreSQL's `tsvector`:

```rust
use otl_rag::PostgresKeywordSearch;

let keyword_search = PostgresKeywordSearch::new(pool);

// Index a document chunk
keyword_search.index_chunk(
    document_id,
    chunk_id,
    "Machine learning is a subset of artificial intelligence."
).await?;

// Batch indexing for better performance
let chunks = vec![
    (doc_id, chunk1_id, content1),
    (doc_id, chunk2_id, content2),
    (doc_id, chunk3_id, content3),
];
keyword_search.index_chunks_batch(chunks).await?;
```

### 2. Search Modes

#### Standard Search (BM25)

Default search mode using BM25-like ranking:

```rust
let results = keyword_search.search("machine learning", 10).await?;
```

#### Phrase Search

Exact phrase matching with quotes:

```rust
let results = keyword_search.search("\"deep learning\"", 10).await?;
```

#### Boolean Search

Complex queries with operators:

```rust
// AND operator - both terms must be present
let results = keyword_search.search("machine AND learning", 10).await?;

// OR operator - either term can be present
let results = keyword_search.search("neural OR network", 10).await?;

// NOT operator - exclude terms
let results = keyword_search.search("AI NOT robotics", 10).await?;

// Complex expressions
let results = keyword_search.search("(machine OR deep) AND learning", 10).await?;
```

#### Fuzzy Search

Handle typos and variations:

```rust
use otl_rag::SearchMode;

// Will find "machine learning" despite typos
let results = keyword_search.search_with_mode(
    "machne lerning",
    SearchMode::Fuzzy,
    10
).await?;
```

### 3. Configuration

Customize search behavior:

```rust
use otl_rag::{PostgresKeywordSearch, KeywordSearchConfig};

let config = KeywordSearchConfig {
    k1: 1.2,                    // BM25 term saturation (1.2-2.0)
    b: 0.75,                    // Length normalization (0.75 typical)
    fuzzy_threshold: 0.3,       // Minimum similarity for fuzzy search
    enable_query_logging: true, // Enable analytics
};

let keyword_search = PostgresKeywordSearch::with_config(pool, config);
```

### 4. Hybrid RAG Integration

Use keyword search with the RAG orchestrator:

```rust
use otl_rag::{HybridRagOrchestrator, RagConfig};

let orchestrator = HybridRagOrchestrator::new(
    vector_store,
    graph_store,
    llm_client,
    RagConfig::default(),
).with_keyword_store(keyword_search);

// Queries now use all three search methods
let response = orchestrator.query(&query, &user).await?;
```

### 5. Analytics and Monitoring

Track search performance:

```rust
// Get search statistics
let stats = keyword_search.get_statistics().await?;
println!("Total documents: {}", stats.total_documents);
println!("Avg length: {:.2} words", stats.avg_document_length);

// Query logs are automatically tracked in fts_query_log table
// View popular searches:
// SELECT * FROM popular_search_terms;

// View performance metrics:
// SELECT * FROM search_performance_metrics;
```

## Database Schema

### Tables

- **fts_documents**: Indexed document chunks with tsvector
- **fts_statistics**: Global statistics for BM25 scoring
- **fts_term_stats**: Term frequency statistics
- **fts_query_log**: Query analytics and performance tracking

### Key Functions

- `search_bm25(query, limit, k1, b)`: Standard BM25 search
- `search_phrase(phrase, limit)`: Exact phrase matching
- `search_boolean(expression, limit)`: Boolean query search
- `search_fuzzy(text, threshold, limit)`: Fuzzy search
- `bm25_score(...)`: Calculate BM25 score for a term

### Maintenance

```sql
-- Reindex all documents (after bulk updates)
SELECT reindex_fts_documents();

-- Clean up old query logs (keeps last 30 days)
SELECT cleanup_old_query_logs();

-- Manual statistics update
UPDATE fts_statistics
SET total_documents = (SELECT COUNT(*) FROM fts_documents),
    avg_document_length = (SELECT AVG(word_count) FROM fts_documents)
WHERE id = 1;
```

## Performance Optimization

### Indexing

1. **GIN indexes** are used for fast full-text search (faster than GiST for most queries)
2. **Batch indexing** reduces transaction overhead
3. **Auto-generated tsvector** columns are stored for faster querying

### Query Optimization

1. **Prepared statements** via sqlx for query safety and caching
2. **Limited result sets** to avoid unnecessary processing
3. **Statistics tracking** for query optimization insights

### Best Practices

1. **Reindex periodically** after bulk updates
2. **Monitor query logs** to optimize common queries
3. **Adjust BM25 parameters** (k1, b) based on your content
4. **Use appropriate search modes** for different query types

## Search Quality

### When to Use Each Mode

| Search Mode | Use Case | Example |
|-------------|----------|---------|
| **Standard** | General keyword search | "machine learning algorithms" |
| **Phrase** | Exact quotes or terms | "neural network architecture" |
| **Boolean** | Complex requirements | "deep learning AND (CNN OR RNN)" |
| **Fuzzy** | Handling typos/variations | User input with potential errors |

### Scoring

BM25 scoring considers:

1. **Term frequency (TF)**: How often a term appears in the document
2. **Inverse document frequency (IDF)**: How rare the term is across all documents
3. **Document length**: Normalized to avoid bias toward longer documents

### Hybrid Ranking

Results from keyword search are combined with vector and graph search using **Reciprocal Rank Fusion (RRF)**:

```
RRF_score(result) = Σ (weight / (k + rank))
```

Where:
- `weight`: Configured per search type (keyword_weight = 0.8 by default)
- `k`: RRF constant (typically 60)
- `rank`: Position in the ranked list from each search method

## Migration

Apply the database migration:

```bash
# Using sqlx-cli
sqlx migrate run

# Or manually
psql -U otl -d otl -f migrations/002_fulltext_search.sql
```

## Testing

Run integration tests (requires PostgreSQL):

```bash
# Run all tests including ignored ones
cargo test --package otl-rag --test keyword_search_tests -- --ignored

# Run specific test
cargo test --package otl-rag --test keyword_search_tests test_boolean_search -- --ignored
```

Run the example:

```bash
cargo run --package otl-rag --example keyword_search_integration
```

## Troubleshooting

### Issue: No results returned

**Solutions:**
1. Verify documents are indexed: `SELECT COUNT(*) FROM fts_documents;`
2. Check query syntax for boolean searches
3. Ensure statistics are up-to-date: `SELECT * FROM fts_statistics;`

### Issue: Slow query performance

**Solutions:**
1. Check if GIN indexes exist: `\d fts_documents` in psql
2. Run VACUUM ANALYZE: `SELECT reindex_fts_documents();`
3. Review query complexity and limit result size

### Issue: Poor ranking quality

**Solutions:**
1. Adjust BM25 parameters (k1, b) in config
2. Check document length distribution
3. Verify term statistics are accurate

## Future Enhancements

Potential improvements for future versions:

1. **Multi-language support**: Add language-specific tsvector columns
2. **Synonym expansion**: Integrate thesaurus for synonym matching
3. **Semantic boosting**: Combine keyword scores with embeddings
4. **Custom ranking functions**: Allow user-defined scoring algorithms
5. **Faceted search**: Add support for filtering by metadata
6. **Highlighting**: Return matching text snippets with highlights

## References

- [PostgreSQL Full-Text Search](https://www.postgresql.org/docs/current/textsearch.html)
- [BM25 Algorithm](https://en.wikipedia.org/wiki/Okapi_BM25)
- [Reciprocal Rank Fusion](https://plg.uwaterloo.ca/~gvcormac/cormacksigir09-rrf.pdf)

## Related Documentation

- [RAG Pipeline Overview](./TECHNICAL_GUIDE.md)
- [Database Schema](./DEPLOYMENT.md#database)
- [API Documentation](./PROJECT.md#api)
