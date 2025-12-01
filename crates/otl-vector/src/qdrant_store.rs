//! Qdrant implementation for vector storage
//!
//! Provides connection management and vector operations
//! for document chunk embeddings.

use async_trait::async_trait;
use otl_core::{DatabaseConfig, DocumentAcl, OtlError, Result, SearchResult, SearchResultType, SourceReference};
use qdrant_client::qdrant::{
    CreateCollection, Distance, PointStruct, SearchPoints, VectorParams, VectorsConfig,
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
            .map_err(|e| OtlError::DatabaseError(format!("Qdrant connection failed: {}", e)))?;

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
            .map_err(|e| OtlError::DatabaseError(format!("Failed to list collections: {}", e)))?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection);

        if !exists {
            self.client
                .create_collection(CreateCollection {
                    collection_name: self.collection.clone(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(qdrant_client::qdrant::vectors_config::Config::Params(
                            VectorParams {
                                size: self.dimension as u64,
                                distance: Distance::Cosine.into(),
                                ..Default::default()
                            },
                        )),
                    }),
                    ..Default::default()
                })
                .await
                .map_err(|e| OtlError::DatabaseError(format!("Failed to create collection: {}", e)))?;
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

        let point = PointStruct::new(
            embedding.id.to_string(),
            embedding.vector.clone(),
            serde_json::to_value(&payload)
                .unwrap_or_default()
                .as_object()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect::<std::collections::HashMap<_, _>>(),
        );

        self.client
            .upsert_points(self.collection.clone(), None, vec![point], None)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to upsert vector: {}", e)))?;

        Ok(())
    }

    async fn search(&self, query_vector: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let results = self
            .client
            .search_points(SearchPoints {
                collection_name: self.collection.clone(),
                vector: query_vector.to_vec(),
                limit: limit as u64,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await
            .map_err(|e| OtlError::SearchError(format!("Vector search failed: {}", e)))?;

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
                    .unwrap_or("internal");

                Some(SearchResult {
                    content,
                    score: point.score,
                    source: SourceReference::new(document_id),
                    acl: DocumentAcl {
                        access_level: match access_level {
                            "public" => otl_core::AccessLevel::Public,
                            "confidential" => otl_core::AccessLevel::Confidential,
                            "restricted" => otl_core::AccessLevel::Restricted,
                            _ => otl_core::AccessLevel::Internal,
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
        use qdrant_client::qdrant::{PointsSelector, Filter, Condition, FieldCondition, Match};
        use qdrant_client::qdrant::r#match::MatchValue;

        let filter = Filter {
            must: vec![Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: "document_id".to_string(),
                        r#match: Some(Match {
                            match_value: Some(MatchValue::Keyword(document_id.to_string())),
                        }),
                        ..Default::default()
                    },
                )),
            }],
            ..Default::default()
        };

        let result = self
            .client
            .delete_points(
                self.collection.clone(),
                None,
                &PointsSelector {
                    points_selector_one_of: Some(
                        qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Filter(filter),
                    ),
                },
                None,
            )
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to delete vectors: {}", e)))?;

        Ok(result.result.map(|r| r.operation_id).unwrap_or(0))
    }
}
