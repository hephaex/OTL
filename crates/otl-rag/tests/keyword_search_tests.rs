//! Integration tests for keyword search backend
//!
//! These tests require a PostgreSQL database with the full-text search schema.
//! Run with: cargo test --package otl-rag --test keyword_search_tests
//!
//! Author: hephaex@gmail.com

use otl_core::{DocumentAcl, SearchBackend};
use otl_rag::{KeywordSearchConfig, PostgresKeywordSearch, SearchMode};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// Helper to create test database pool
async fn create_test_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://otl:otl_dev_password@localhost:5432/otl".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

// Helper to setup test data
async fn setup_test_data(pool: &PgPool) -> (Uuid, Vec<Uuid>) {
    let doc_id = Uuid::new_v4();

    // Insert test document
    sqlx::query!(
        r#"
        INSERT INTO documents (id, title, file_path, file_type, file_size, access_level)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        doc_id,
        "Test Document",
        "/test/path.pdf",
        "pdf",
        1024i64,
        "internal"
    )
    .execute(pool)
    .await
    .expect("Failed to insert test document");

    // Insert test chunks
    let chunk1_id = Uuid::new_v4();
    let chunk2_id = Uuid::new_v4();
    let chunk3_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO document_chunks (id, document_id, chunk_index, content)
        VALUES ($1, $2, $3, $4)
        "#,
        chunk1_id,
        doc_id,
        0,
        "Machine learning is a subset of artificial intelligence."
    )
    .execute(pool)
    .await
    .expect("Failed to insert chunk 1");

    sqlx::query!(
        r#"
        INSERT INTO document_chunks (id, document_id, chunk_index, content)
        VALUES ($1, $2, $3, $4)
        "#,
        chunk2_id,
        doc_id,
        1,
        "Deep learning uses neural networks with multiple layers."
    )
    .execute(pool)
    .await
    .expect("Failed to insert chunk 2");

    sqlx::query!(
        r#"
        INSERT INTO document_chunks (id, document_id, chunk_index, content)
        VALUES ($1, $2, $3, $4)
        "#,
        chunk3_id,
        doc_id,
        2,
        "Natural language processing is a field of AI that deals with text."
    )
    .execute(pool)
    .await
    .expect("Failed to insert chunk 3");

    (doc_id, vec![chunk1_id, chunk2_id, chunk3_id])
}

// Helper to cleanup test data
async fn cleanup_test_data(pool: &PgPool, doc_id: Uuid) {
    sqlx::query!("DELETE FROM fts_documents WHERE document_id = $1", doc_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM document_chunks WHERE document_id = $1", doc_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM documents WHERE id = $1", doc_id)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore] // Run with --ignored flag when database is available
async fn test_index_and_search() {
    let pool = create_test_pool().await;
    let (doc_id, chunk_ids) = setup_test_data(&pool).await;

    let keyword_search = PostgresKeywordSearch::new(Arc::new(pool.clone()));

    // Index chunks
    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[0],
            "Machine learning is a subset of artificial intelligence.",
        )
        .await
        .expect("Failed to index chunk 1");

    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[1],
            "Deep learning uses neural networks with multiple layers.",
        )
        .await
        .expect("Failed to index chunk 2");

    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[2],
            "Natural language processing is a field of AI that deals with text.",
        )
        .await
        .expect("Failed to index chunk 3");

    // Test standard search
    let results = keyword_search
        .search("machine learning", 5)
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Should find results for 'machine learning'");
    assert!(
        results[0].content.contains("Machine learning"),
        "Top result should contain 'Machine learning'"
    );

    // Cleanup
    cleanup_test_data(&pool, doc_id).await;
}

#[tokio::test]
#[ignore]
async fn test_phrase_search() {
    let pool = create_test_pool().await;
    let (doc_id, chunk_ids) = setup_test_data(&pool).await;

    let keyword_search = PostgresKeywordSearch::new(Arc::new(pool.clone()));

    // Index test data
    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[0],
            "Machine learning is a subset of artificial intelligence.",
        )
        .await
        .expect("Failed to index chunk");

    // Test phrase search
    let results = keyword_search
        .search_with_mode("\"machine learning\"", SearchMode::Phrase, 5)
        .await
        .expect("Phrase search failed");

    assert!(!results.is_empty(), "Should find results for phrase search");

    // Cleanup
    cleanup_test_data(&pool, doc_id).await;
}

#[tokio::test]
#[ignore]
async fn test_boolean_search() {
    let pool = create_test_pool().await;
    let (doc_id, chunk_ids) = setup_test_data(&pool).await;

    let keyword_search = PostgresKeywordSearch::new(Arc::new(pool.clone()));

    // Index chunks
    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[0],
            "Machine learning is a subset of artificial intelligence.",
        )
        .await
        .expect("Failed to index chunk 1");

    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[1],
            "Deep learning uses neural networks with multiple layers.",
        )
        .await
        .expect("Failed to index chunk 2");

    // Test AND operator
    let results = keyword_search
        .search_with_mode("learning AND neural", SearchMode::Boolean, 5)
        .await
        .expect("Boolean search failed");

    assert!(!results.is_empty(), "Should find results with AND operator");
    assert!(
        results[0].content.contains("neural"),
        "Result should contain both terms"
    );

    // Test OR operator
    let results = keyword_search
        .search_with_mode("machine OR neural", SearchMode::Boolean, 5)
        .await
        .expect("Boolean search failed");

    assert!(!results.is_empty(), "Should find results with OR operator");

    // Cleanup
    cleanup_test_data(&pool, doc_id).await;
}

#[tokio::test]
#[ignore]
async fn test_fuzzy_search() {
    let pool = create_test_pool().await;
    let (doc_id, chunk_ids) = setup_test_data(&pool).await;

    let keyword_search = PostgresKeywordSearch::new(Arc::new(pool.clone()));

    // Index chunk
    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[0],
            "Machine learning is a subset of artificial intelligence.",
        )
        .await
        .expect("Failed to index chunk");

    // Test fuzzy search (typo in "machine")
    let results = keyword_search
        .search_with_mode("machne lerning", SearchMode::Fuzzy, 5)
        .await
        .expect("Fuzzy search failed");

    // Fuzzy search should still find results despite typos
    assert!(
        !results.is_empty(),
        "Fuzzy search should handle typos"
    );

    // Cleanup
    cleanup_test_data(&pool, doc_id).await;
}

#[tokio::test]
#[ignore]
async fn test_batch_indexing() {
    let pool = create_test_pool().await;
    let (doc_id, chunk_ids) = setup_test_data(&pool).await;

    let keyword_search = PostgresKeywordSearch::new(Arc::new(pool.clone()));

    // Batch index
    let chunks = vec![
        (
            doc_id,
            chunk_ids[0],
            "Machine learning is a subset of artificial intelligence.".to_string(),
        ),
        (
            doc_id,
            chunk_ids[1],
            "Deep learning uses neural networks with multiple layers.".to_string(),
        ),
        (
            doc_id,
            chunk_ids[2],
            "Natural language processing is a field of AI that deals with text.".to_string(),
        ),
    ];

    keyword_search
        .index_chunks_batch(chunks)
        .await
        .expect("Batch indexing failed");

    // Verify indexing worked
    let results = keyword_search
        .search("deep learning", 5)
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Should find indexed documents");

    // Cleanup
    cleanup_test_data(&pool, doc_id).await;
}

#[tokio::test]
#[ignore]
async fn test_delete_document() {
    let pool = create_test_pool().await;
    let (doc_id, chunk_ids) = setup_test_data(&pool).await;

    let keyword_search = PostgresKeywordSearch::new(Arc::new(pool.clone()));

    // Index a chunk
    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[0],
            "Machine learning is a subset of artificial intelligence.",
        )
        .await
        .expect("Failed to index chunk");

    // Delete document
    let deleted = keyword_search
        .delete_document(doc_id)
        .await
        .expect("Failed to delete document");

    assert!(deleted > 0, "Should delete at least one chunk");

    // Verify deletion
    let results = keyword_search
        .search("machine learning", 5)
        .await
        .expect("Search failed");

    assert!(
        results.is_empty() || !results.iter().any(|r| r.source.document_id == doc_id),
        "Deleted document should not appear in search results"
    );

    // Cleanup
    cleanup_test_data(&pool, doc_id).await;
}

#[tokio::test]
#[ignore]
async fn test_statistics() {
    let pool = create_test_pool().await;

    let keyword_search = PostgresKeywordSearch::new(Arc::new(pool.clone()));

    // Get statistics
    let stats = keyword_search
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    // Should have valid statistics
    assert!(stats.total_documents >= 0, "Total documents should be non-negative");
    assert!(
        stats.avg_document_length >= 0.0,
        "Average document length should be non-negative"
    );
}

#[tokio::test]
#[ignore]
async fn test_search_mode_auto_detection() {
    let pool = create_test_pool().await;
    let (doc_id, chunk_ids) = setup_test_data(&pool).await;

    let keyword_search = PostgresKeywordSearch::new(Arc::new(pool.clone()));

    // Index chunk
    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[0],
            "Machine learning is a subset of artificial intelligence.",
        )
        .await
        .expect("Failed to index chunk");

    // Test auto-detection of phrase search
    let results = keyword_search
        .search("\"machine learning\"", 5)
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Should auto-detect phrase search");

    // Test auto-detection of boolean search
    let results = keyword_search
        .search("machine AND learning", 5)
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Should auto-detect boolean search");

    // Cleanup
    cleanup_test_data(&pool, doc_id).await;
}

#[tokio::test]
#[ignore]
async fn test_search_ranking() {
    let pool = create_test_pool().await;
    let (doc_id, chunk_ids) = setup_test_data(&pool).await;

    let keyword_search = PostgresKeywordSearch::new(Arc::new(pool.clone()));

    // Index chunks with different relevance
    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[0],
            "Machine learning machine learning is important.", // Higher term frequency
        )
        .await
        .expect("Failed to index chunk 1");

    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[1],
            "Deep learning uses machine learning techniques.",
        )
        .await
        .expect("Failed to index chunk 2");

    keyword_search
        .index_chunk(
            doc_id,
            chunk_ids[2],
            "Natural language processing is different.",
        )
        .await
        .expect("Failed to index chunk 3");

    // Search for "machine learning"
    let results = keyword_search
        .search("machine learning", 5)
        .await
        .expect("Search failed");

    assert!(!results.is_empty(), "Should find results");

    // First result should have higher score (more occurrences)
    if results.len() > 1 {
        assert!(
            results[0].score >= results[1].score,
            "Results should be ranked by relevance"
        );
    }

    // Cleanup
    cleanup_test_data(&pool, doc_id).await;
}
