//! Example: Using keyword search with hybrid RAG
//!
//! This example demonstrates how to integrate the PostgreSQL keyword search
//! backend with the hybrid RAG orchestrator for improved retrieval.
//!
//! Run with: cargo run --example keyword_search_integration
//!
//! Author: hephaex@gmail.com

use otl_core::{LlmClient, RagQuery, SearchBackend, User};
use otl_rag::{
    HybridRagOrchestrator, KeywordSearchConfig, PostgresKeywordSearch, RagConfig,
};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// Mock LLM client for demonstration
struct MockLlmClient;

#[async_trait::async_trait]
impl LlmClient for MockLlmClient {
    async fn generate(&self, _prompt: &str) -> otl_core::Result<String> {
        Ok("This is a mock LLM response.".to_string())
    }

    async fn generate_stream(
        &self,
        _prompt: &str,
    ) -> otl_core::Result<futures::stream::BoxStream<'static, otl_core::Result<String>>> {
        use futures::stream;
        Ok(Box::pin(stream::iter(vec![Ok(
            "Mock streaming response.".to_string()
        )])))
    }
}

// Mock vector search backend
struct MockVectorSearch;

#[async_trait::async_trait]
impl SearchBackend for MockVectorSearch {
    async fn search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> otl_core::Result<Vec<otl_core::SearchResult>> {
        Ok(Vec::new())
    }

    fn name(&self) -> &str {
        "MockVectorSearch"
    }
}

// Mock graph search backend
struct MockGraphSearch;

#[async_trait::async_trait]
impl SearchBackend for MockGraphSearch {
    async fn search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> otl_core::Result<Vec<otl_core::SearchResult>> {
        Ok(Vec::new())
    }

    fn name(&self) -> &str {
        "MockGraphSearch"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Keyword Search Integration Example ===\n");

    // 1. Connect to PostgreSQL
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://otl:otl_dev_password@localhost:5432/otl".to_string());

    println!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;
    println!("✓ Connected to PostgreSQL\n");

    // 2. Create keyword search backend with custom configuration
    let keyword_config = KeywordSearchConfig {
        k1: 1.2,           // BM25 parameter
        b: 0.75,           // Length normalization
        fuzzy_threshold: 0.3,
        enable_query_logging: true,
    };

    let keyword_search = Arc::new(PostgresKeywordSearch::with_config(
        Arc::new(pool.clone()),
        keyword_config,
    ));

    println!("✓ Keyword search backend initialized\n");

    // 3. Index sample documents
    println!("Indexing sample documents...");

    let doc_id = Uuid::new_v4();
    let chunk1_id = Uuid::new_v4();
    let chunk2_id = Uuid::new_v4();

    // First, create the document in the database
    sqlx::query!(
        r#"
        INSERT INTO documents (id, title, file_path, file_type, file_size, access_level)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        doc_id,
        "Machine Learning Guide",
        "/docs/ml_guide.pdf",
        "pdf",
        2048i64,
        "internal"
    )
    .execute(&pool)
    .await?;

    // Create chunks
    sqlx::query!(
        r#"
        INSERT INTO document_chunks (id, document_id, chunk_index, content)
        VALUES ($1, $2, $3, $4)
        "#,
        chunk1_id,
        doc_id,
        0,
        "Machine learning is a field of artificial intelligence that enables computers to learn from data without being explicitly programmed."
    )
    .execute(&pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO document_chunks (id, document_id, chunk_index, content)
        VALUES ($1, $2, $3, $4)
        "#,
        chunk2_id,
        doc_id,
        1,
        "Deep learning is a subset of machine learning that uses neural networks with multiple layers to learn hierarchical representations."
    )
    .execute(&pool)
    .await?;

    // Index the chunks
    keyword_search
        .index_chunk(
            doc_id,
            chunk1_id,
            "Machine learning is a field of artificial intelligence that enables computers to learn from data without being explicitly programmed.",
        )
        .await?;

    keyword_search
        .index_chunk(
            doc_id,
            chunk2_id,
            "Deep learning is a subset of machine learning that uses neural networks with multiple layers to learn hierarchical representations.",
        )
        .await?;

    println!("✓ Indexed 2 document chunks\n");

    // 4. Test different search modes
    println!("=== Testing Search Modes ===\n");

    // Standard search
    println!("1. Standard BM25 Search:");
    let results = keyword_search.search("machine learning", 5).await?;
    println!("   Found {} results", results.len());
    for (i, result) in results.iter().enumerate() {
        println!(
            "   [{}] Score: {:.4} - {}...",
            i + 1,
            result.score,
            result.content.chars().take(60).collect::<String>()
        );
    }
    println!();

    // Phrase search
    println!("2. Phrase Search:");
    let results = keyword_search.search("\"deep learning\"", 5).await?;
    println!("   Found {} results", results.len());
    for (i, result) in results.iter().enumerate() {
        println!(
            "   [{}] Score: {:.4} - {}...",
            i + 1,
            result.score,
            result.content.chars().take(60).collect::<String>()
        );
    }
    println!();

    // Boolean search
    println!("3. Boolean Search (AND operator):");
    let results = keyword_search.search("machine AND neural", 5).await?;
    println!("   Found {} results", results.len());
    for (i, result) in results.iter().enumerate() {
        println!(
            "   [{}] Score: {:.4} - {}...",
            i + 1,
            result.score,
            result.content.chars().take(60).collect::<String>()
        );
    }
    println!();

    // 5. Create hybrid RAG orchestrator with keyword search
    println!("=== Creating Hybrid RAG Orchestrator ===\n");

    let vector_store = Arc::new(MockVectorSearch) as Arc<dyn SearchBackend>;
    let graph_store = Arc::new(MockGraphSearch) as Arc<dyn SearchBackend>;
    let llm_client = Arc::new(MockLlmClient) as Arc<dyn LlmClient>;

    let rag_config = RagConfig {
        vector_top_k: 20,
        graph_depth: 2,
        keyword_top_k: 10,
        final_top_k: 5,
        min_score: 0.0,
        rrf_k: 60.0,
        vector_weight: 1.0,
        graph_weight: 1.5,
        keyword_weight: 0.8,
        max_context_length: 8000,
        include_ontology: true,
    };

    let orchestrator = HybridRagOrchestrator::new(
        vector_store,
        graph_store,
        llm_client,
        rag_config,
    )
    .with_keyword_store(keyword_search.clone() as Arc<dyn SearchBackend>);

    println!("✓ Hybrid RAG orchestrator created with keyword search\n");

    // 6. Execute a RAG query
    println!("=== Executing RAG Query ===\n");

    let query = RagQuery {
        question: "What is deep learning?".to_string(),
        filters: None,
    };

    let user = User::internal("demo_user", vec!["EMPLOYEE".to_string()]);

    println!("Query: {}", query.question);
    let response = orchestrator.query(&query, &user).await?;

    println!("\nResponse:");
    println!("  Answer: {}", response.answer);
    println!("  Confidence: {:.2}", response.confidence);
    println!("  Processing time: {}ms", response.processing_time_ms);
    println!("  Citations: {}", response.citations.len());
    println!();

    // 7. Get search statistics
    println!("=== Search Statistics ===\n");
    let stats = keyword_search.get_statistics().await?;
    println!("  Total documents: {}", stats.total_documents);
    println!("  Average document length: {:.2} words", stats.avg_document_length);
    println!("  Last updated: {}", stats.last_updated);
    println!();

    // Cleanup
    println!("Cleaning up test data...");
    keyword_search.delete_document(doc_id).await?;
    sqlx::query!("DELETE FROM document_chunks WHERE document_id = $1", doc_id)
        .execute(&pool)
        .await?;
    sqlx::query!("DELETE FROM documents WHERE id = $1", doc_id)
        .execute(&pool)
        .await?;
    println!("✓ Cleanup complete\n");

    println!("=== Example Complete ===");

    Ok(())
}
