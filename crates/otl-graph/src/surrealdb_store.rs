//! SurrealDB implementation for graph storage
//!
//! Provides connection management and CRUD operations for
//! entities and triples in SurrealDB.

use async_trait::async_trait;
use otl_core::{DatabaseConfig, Entity, OtlError, Result, SourceReference, Triple};
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use uuid::Uuid;

/// SurrealDB graph store implementation
pub struct SurrealDbStore {
    client: Surreal<Client>,
}

impl SurrealDbStore {
    /// Create a new SurrealDB connection
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        // Remove ws:// or wss:// prefix if present (surrealdb crate adds it automatically)
        let url = config
            .surrealdb_url
            .strip_prefix("ws://")
            .or_else(|| config.surrealdb_url.strip_prefix("wss://"))
            .unwrap_or(&config.surrealdb_url);

        let client = Surreal::new::<Ws>(url)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("SurrealDB connection failed: {e}")))?;

        // Authenticate
        client
            .signin(Root {
                username: &config.surrealdb_user,
                password: &config.surrealdb_pass,
            })
            .await
            .map_err(|e| OtlError::DatabaseError(format!("SurrealDB auth failed: {e}")))?;

        // Select namespace and database
        client
            .use_ns(&config.surrealdb_namespace)
            .use_db(&config.surrealdb_database)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("SurrealDB namespace error: {e}")))?;

        Ok(Self { client })
    }

    /// Initialize schema (run once on setup)
    pub async fn init_schema(&self) -> Result<()> {
        // Define entity table
        self.client
            .query(
                r#"
                DEFINE TABLE entity SCHEMAFULL;
                DEFINE FIELD class ON entity TYPE string;
                DEFINE FIELD properties ON entity TYPE object;
                DEFINE FIELD source ON entity TYPE object;
                DEFINE FIELD created_at ON entity TYPE datetime DEFAULT time::now();
                DEFINE FIELD updated_at ON entity TYPE datetime DEFAULT time::now();
                DEFINE INDEX idx_entity_class ON entity FIELDS class;
            "#,
            )
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Schema init failed: {e}")))?;

        Ok(())
    }
}

/// Entity record for SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntityRecord {
    id: Option<surrealdb::sql::Thing>,
    class: String,
    properties: serde_json::Value,
    source: SourceRecord,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Source reference record
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceRecord {
    document_id: String,
    page: Option<u32>,
    section: Option<String>,
    confidence: f32,
}

impl From<&SourceReference> for SourceRecord {
    fn from(src: &SourceReference) -> Self {
        Self {
            document_id: src.document_id.to_string(),
            page: src.page,
            section: src.section.clone(),
            confidence: src.confidence,
        }
    }
}

#[async_trait]
impl super::GraphStore for SurrealDbStore {
    async fn store_entity(&self, entity: &Entity) -> Result<()> {
        let record = EntityRecord {
            id: None,
            class: entity.class.clone(),
            properties: serde_json::to_value(&entity.properties).unwrap_or_default(),
            source: SourceRecord::from(&entity.source),
            created_at: Some(entity.created_at),
            updated_at: Some(entity.updated_at),
        };

        let _: Option<EntityRecord> = self
            .client
            .create(("entity", entity.id.to_string()))
            .content(record)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to store entity: {e}")))?;

        Ok(())
    }

    async fn store_triple(&self, triple: &Triple) -> Result<()> {
        let query = format!(
            "RELATE entity:{}->relates->entity:{} SET predicate = $predicate, confidence = $confidence",
            triple.subject, triple.object
        );

        let predicate = triple.predicate.clone();
        let confidence = triple.confidence;

        self.client
            .query(&query)
            .bind(("predicate", predicate))
            .bind(("confidence", confidence))
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to store triple: {e}")))?;

        Ok(())
    }

    async fn get_entity(&self, id: Uuid) -> Result<Option<Entity>> {
        let record: Option<EntityRecord> = self
            .client
            .select(("entity", id.to_string()))
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to get entity: {e}")))?;

        Ok(record.map(|r| Entity {
            id,
            class: r.class,
            properties: serde_json::from_value(r.properties).unwrap_or_default(),
            source: SourceReference::new(
                Uuid::parse_str(&r.source.document_id).unwrap_or_default(),
            ),
            created_at: r.created_at.unwrap_or_default(),
            updated_at: r.updated_at.unwrap_or_default(),
        }))
    }

    async fn find_by_class(&self, class: &str, limit: usize) -> Result<Vec<Entity>> {
        let class_owned = class.to_string();
        let records: Vec<EntityRecord> = self
            .client
            .query("SELECT * FROM entity WHERE class = $class LIMIT $limit")
            .bind(("class", class_owned))
            .bind(("limit", limit))
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Query failed: {e}")))?
            .take(0)
            .map_err(|e| OtlError::DatabaseError(format!("Result extraction failed: {e}")))?;

        Ok(records
            .into_iter()
            .map(|r| {
                let id =
                    r.id.as_ref()
                        .and_then(|t| Uuid::parse_str(&t.id.to_string()).ok())
                        .unwrap_or_default();
                Entity {
                    id,
                    class: r.class,
                    properties: serde_json::from_value(r.properties).unwrap_or_default(),
                    source: SourceReference::new(
                        Uuid::parse_str(&r.source.document_id).unwrap_or_default(),
                    ),
                    created_at: r.created_at.unwrap_or_default(),
                    updated_at: r.updated_at.unwrap_or_default(),
                }
            })
            .collect())
    }

    async fn traverse(&self, start_id: Uuid, depth: u32) -> Result<Vec<Entity>> {
        let query = format!(
            "SELECT ->relates->(? WHERE true)<-relates<-entity AS connected FROM entity:{} LIMIT {}",
            start_id, depth * 10
        );

        let _result = self
            .client
            .query(&query)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Traverse failed: {e}")))?;

        // Simplified: return empty for now, real implementation would parse graph results
        Ok(Vec::new())
    }

    async fn query(&self, query: &str) -> Result<Vec<Entity>> {
        let _result = self
            .client
            .query(query)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Query failed: {e}")))?;

        Ok(Vec::new())
    }
}
