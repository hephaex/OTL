//! Qdrant implementation for vector storage
//!
//! Provides connection management and vector operations
//! for document chunk embeddings.
//!
//! Author: hephaex@gmail.com

use async_trait::async_trait;
use otl_core::{
    AccessLevel, DatabaseConfig, DocumentAcl, OtlError, Result, SearchBackend, SearchResult,
    SearchResultType, SourceReference,
};
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointStruct,
    SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::embedding::EmbeddingClient;
use crate::VectorStore;

/// Qdrant vector store implementation
pub struct QdrantStore {
    client: Qdrant,
    collection: String,
    dimension: usize,
}

/// Vector search backend that wraps QdrantStore with an embedding client
pub struct VectorSearchBackend {
    store: QdrantStore,
    embedding_client: Arc<dyn EmbeddingClient>,
}

impl QdrantStore {
    /// Create a new Qdrant connection
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let client = Qdrant::from_url(&config.qdrant_url)
            .build()
            .map_err(|e| OtlError::DatabaseError(format!("Qdrant connection failed: {e}")))?;

        Ok(Self {
            client,
            collection: config.qdrant_collection.clone(),
            dimension: config.vector_dimension,
        })
    }

    /// Initialize collection (run once on setup)
    pub async fn init_collection(&self) -> Result<()> {
        // Check if collection exists
        let collections = self
            .client
            .list_collections()
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to list collections: {e}")))?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection);

        if !exists {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection).vectors_config(
                        VectorParamsBuilder::new(self.dimension as u64, Distance::Cosine),
                    ),
                )
                .await
                .map_err(|e| {
                    OtlError::DatabaseError(format!("Failed to create collection: {e}"))
                })?;
        }

        Ok(())
    }
}

/// Payload stored with each vector
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorPayload {
    document_id: String,
    chunk_index: u32,
    content: String,
    page: Option<u32>,
    section: Option<String>,
    access_level: String,
    department: Option<String>,
    required_roles: Vec<String>,
}

#[async_trait]
impl super::VectorStore for QdrantStore {
    async fn store(&self, embedding: &super::EmbeddingVector) -> Result<()> {
        let payload = VectorPayload {
            document_id: embedding.document_id.to_string(),
            chunk_index: embedding.chunk_index,
            content: embedding.content.clone(),
            page: None,
            section: None,
            access_level: "internal".to_string(),
            department: None,
            required_roles: vec![],
        };

        let payload_map: std::collections::HashMap<String, qdrant_client::qdrant::Value> =
            serde_json::to_value(&payload)
                .unwrap_or_default()
                .as_object()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect();

        let point = PointStruct::new(
            embedding.id.to_string(),
            embedding.vector.clone(),
            payload_map,
        );

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, vec![point]))
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to upsert vector: {e}")))?;

        Ok(())
    }

    async fn search(&self, query_vector: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let results = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection, query_vector.to_vec(), limit as u64)
                    .with_payload(true),
            )
            .await
            .map_err(|e| OtlError::SearchError(format!("Vector search failed: {e}")))?;

        let search_results: Vec<SearchResult> = results
            .result
            .into_iter()
            .map(|point| {
                let payload = point.payload;
                let content = payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                let document_id = payload
                    .get("document_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_default();

                let access_level = payload
                    .get("access_level")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "internal".to_string());

                SearchResult {
                    content,
                    score: point.score,
                    source: SourceReference::new(document_id),
                    acl: DocumentAcl {
                        access_level: match access_level.as_str() {
                            "public" => AccessLevel::Public,
                            "confidential" => AccessLevel::Confidential,
                            "restricted" => AccessLevel::Restricted,
                            _ => AccessLevel::Internal,
                        },
                        ..Default::default()
                    },
                    result_type: SearchResultType::Vector,
                }
            })
            .collect();

        Ok(search_results)
    }

    async fn delete_by_document(&self, document_id: Uuid) -> Result<u64> {
        let filter = Filter::must([Condition::matches("document_id", document_id.to_string())]);

        let _result = self
            .client
            .delete_points(DeletePointsBuilder::new(&self.collection).points(filter))
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to delete vectors: {e}")))?;

        // Return 1 as placeholder - actual count not available from delete response
        Ok(1)
    }
}

// ============================================================================
// VectorSearchBackend Implementation
// ============================================================================

impl VectorSearchBackend {
    /// Create a new vector search backend
    pub fn new(store: QdrantStore, embedding_client: Arc<dyn EmbeddingClient>) -> Self {
        Self {
            store,
            embedding_client,
        }
    }

    /// Create from database config and embedding client
    pub async fn from_config(
        config: &DatabaseConfig,
        embedding_client: Arc<dyn EmbeddingClient>,
    ) -> Result<Self> {
        let store = QdrantStore::new(config).await?;
        Ok(Self::new(store, embedding_client))
    }

    /// Initialize the collection
    pub async fn init(&self) -> Result<()> {
        self.store.init_collection().await
    }

    /// Store an embedding
    pub async fn store(&self, embedding: &super::EmbeddingVector) -> Result<()> {
        self.store.store(embedding).await
    }

    /// Generate embedding and store a text chunk
    pub async fn index_text(
        &self,
        document_id: Uuid,
        chunk_index: u32,
        content: &str,
    ) -> Result<Uuid> {
        let vector = self.embedding_client.embed(content).await?;
        let id = Uuid::new_v4();

        let embedding = super::EmbeddingVector {
            id,
            vector,
            document_id,
            chunk_index,
            content: content.to_string(),
        };

        self.store(&embedding).await?;
        Ok(id)
    }

    /// Search by vector directly
    pub async fn search_by_vector(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        self.store.search(query_vector, limit).await
    }

    /// Delete vectors by document ID
    pub async fn delete_by_document(&self, document_id: Uuid) -> Result<u64> {
        self.store.delete_by_document(document_id).await
    }
}

#[async_trait]
impl SearchBackend for VectorSearchBackend {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Generate embedding for the query
        let query_vector = self
            .embedding_client
            .embed(query)
            .await
            .map_err(|e| OtlError::SearchError(format!("Failed to embed query: {e}")))?;

        // Search with the vector
        self.store.search(&query_vector, limit).await
    }

    fn name(&self) -> &str {
        "vector"
    }
}

// ============================================================================
// Additional Tests
// ============================================================================

#[cfg(test)]
mod vector_backend_tests {

    #[test]
    fn test_vector_backend_name() {
        // VectorSearchBackend requires async initialization, so we just test the trait behavior
        // would require mocking for full tests
    }
}
