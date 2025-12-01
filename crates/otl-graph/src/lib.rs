//! OTL Graph - Graph database abstraction
//!
//! Provides abstraction over graph databases (SurrealDB)
//! for storing and querying ontology-based knowledge graphs.

use async_trait::async_trait;
use otl_core::{Entity, OtlError, Result, Triple};
use uuid::Uuid;

/// Trait for graph database operations
#[async_trait]
pub trait GraphStore: Send + Sync {
    /// Store an entity
    async fn store_entity(&self, entity: &Entity) -> Result<()>;

    /// Store a triple (relationship)
    async fn store_triple(&self, triple: &Triple) -> Result<()>;

    /// Get entity by ID
    async fn get_entity(&self, id: Uuid) -> Result<Option<Entity>>;

    /// Find entities by class
    async fn find_by_class(&self, class: &str, limit: usize) -> Result<Vec<Entity>>;

    /// Traverse graph from an entity
    async fn traverse(&self, start_id: Uuid, depth: u32) -> Result<Vec<Entity>>;

    /// Execute a graph query
    async fn query(&self, query: &str) -> Result<Vec<Entity>>;
}

pub mod surrealdb_store;
