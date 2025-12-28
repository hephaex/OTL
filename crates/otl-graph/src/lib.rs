//! OTL Graph - Graph database abstraction
//!
//! Provides abstraction over graph databases (SurrealDB)
//! for storing and querying ontology-based knowledge graphs.
//!
//! Author: hephaex@gmail.com

use async_trait::async_trait;
use otl_core::{Entity, Result, Triple};
use uuid::Uuid;

pub mod ontology;
pub mod ontology_io;
pub mod ontology_repository;
pub mod search;
pub mod surrealdb_store;

pub use ontology::{
    Constraint, EntityTypeDef, EntityTypeChanges, OntologySchema, PropertyDef, PropertyType,
    RelationCardinality, RelationTypeDef, RelationTypeChanges, SchemaChange, SchemaDiff,
    SchemaMigration,
};
pub use ontology_io::{
    export_json, export_owl, export_rdf_turtle, export_yaml, import_json, import_yaml,
};
pub use ontology_repository::{OntologyRepository, SchemaVersionInfo, SurrealOntologyRepository};
pub use search::GraphSearchBackend;
pub use surrealdb_store::SurrealDbStore;

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

    /// Downcast to concrete type (for accessing specific implementation features)
    fn as_any(&self) -> &dyn std::any::Any;
}
