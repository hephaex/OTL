//! OTL Vector - Vector database abstraction
//!
//! Provides abstraction over vector databases (Qdrant)
//! for storing and searching document embeddings.

use async_trait::async_trait;
use otl_core::{Result, SearchResult};
use uuid::Uuid;

/// A vector with metadata
#[derive(Debug, Clone)]
pub struct EmbeddingVector {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub document_id: Uuid,
    pub chunk_index: u32,
    pub content: String,
}

/// Trait for vector database operations
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Store an embedding
    async fn store(&self, embedding: &EmbeddingVector) -> Result<()>;

    /// Search for similar vectors
    async fn search(&self, query_vector: &[f32], limit: usize) -> Result<Vec<SearchResult>>;

    /// Delete vectors by document ID
    async fn delete_by_document(&self, document_id: Uuid) -> Result<u64>;
}

pub mod qdrant_store;
