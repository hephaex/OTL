//! Qdrant implementation for vector storage
//!
//! Provides connection management and vector operations
//! for document chunk embeddings.
//!
//! Author: hephaex@gmail.com

use async_trait::async_trait;
use otl_core::{
    AccessLevel, DatabaseConfig, DocumentAcl, OtlError, Result, SearchResult, SearchResultType,
    SourceReference,
};
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointStruct,
    SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Qdrant vector store implementation
pub struct QdrantStore {
    client: Qdrant,
    collection: String,
    dimension: usize,
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
        let collections =
            self.client.list_collections().await.map_err(|e| {
                OtlError::DatabaseError(format!("Failed to list collections: {e}"))
            })?;

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
            .filter_map(|point| {
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

                Some(SearchResult {
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
                })
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
