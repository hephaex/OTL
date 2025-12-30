//! Ontology repository for database storage and retrieval
//!
//! Manages persistence of ontology schemas in SurrealDB with versioning support.
//!
//! Author: hephaex@gmail.com

use crate::ontology::{OntologySchema, SchemaMigration};
use async_trait::async_trait;
use otl_core::{OtlError, Result};
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

// ============================================================================
// Repository Trait
// ============================================================================

/// Trait for ontology schema persistence
#[async_trait]
pub trait OntologyRepository: Send + Sync {
    /// Store a new schema version
    async fn store_schema(&self, schema: &OntologySchema) -> Result<()>;

    /// Get the current (latest) schema
    async fn get_current_schema(&self) -> Result<Option<OntologySchema>>;

    /// Get a specific schema version
    async fn get_schema_version(&self, version: &semver::Version) -> Result<Option<OntologySchema>>;

    /// List all schema versions
    async fn list_schema_versions(&self) -> Result<Vec<SchemaVersionInfo>>;

    /// Store a migration
    async fn store_migration(&self, migration: &SchemaMigration) -> Result<()>;

    /// Get all migrations
    async fn list_migrations(&self) -> Result<Vec<SchemaMigration>>;

    /// Get migrations for a specific version transition
    async fn get_migration(
        &self,
        from_version: &semver::Version,
        to_version: &semver::Version,
    ) -> Result<Option<SchemaMigration>>;

    /// Delete an old schema version
    async fn delete_schema_version(&self, version: &semver::Version) -> Result<()>;
}

/// Schema version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersionInfo {
    pub version: semver::Version,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub author: Option<String>,
}

// ============================================================================
// SurrealDB Implementation
// ============================================================================

/// SurrealDB-based ontology repository
pub struct SurrealOntologyRepository {
    client: Surreal<Client>,
}

impl SurrealOntologyRepository {
    /// Create a new repository with existing SurrealDB client
    pub fn new(client: Surreal<Client>) -> Self {
        Self { client }
    }

    /// Initialize the ontology schema tables
    pub async fn init_schema(&self) -> Result<()> {
        // Define ontology_schema table
        self.client
            .query(
                r#"
                DEFINE TABLE ontology_schema SCHEMAFULL;
                DEFINE FIELD version ON ontology_schema TYPE string;
                DEFINE FIELD name ON ontology_schema TYPE string;
                DEFINE FIELD description ON ontology_schema TYPE option<string>;
                DEFINE FIELD schema_data ON ontology_schema TYPE object;
                DEFINE FIELD created_at ON ontology_schema TYPE datetime DEFAULT time::now();
                DEFINE FIELD updated_at ON ontology_schema TYPE datetime DEFAULT time::now();
                DEFINE FIELD author ON ontology_schema TYPE option<string>;
                DEFINE INDEX idx_schema_version ON ontology_schema FIELDS version UNIQUE;
                "#,
            )
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to create ontology_schema table: {e}"))
            })?;

        // Define schema_migration table
        self.client
            .query(
                r#"
                DEFINE TABLE schema_migration SCHEMAFULL;
                DEFINE FIELD from_version ON schema_migration TYPE string;
                DEFINE FIELD to_version ON schema_migration TYPE string;
                DEFINE FIELD migration_data ON schema_migration TYPE object;
                DEFINE FIELD description ON schema_migration TYPE string;
                DEFINE FIELD created_at ON schema_migration TYPE datetime DEFAULT time::now();
                DEFINE FIELD author ON schema_migration TYPE option<string>;
                DEFINE INDEX idx_migration_versions ON schema_migration FIELDS from_version, to_version;
                "#,
            )
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to create schema_migration table: {e}"))
            })?;

        Ok(())
    }
}

/// Schema record for SurrealDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchemaRecord {
    id: Option<surrealdb::sql::Thing>,
    version: String,
    name: String,
    description: Option<String>,
    schema_data: serde_json::Value,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    author: Option<String>,
}

/// Migration record for SurrealDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationRecord {
    id: Option<surrealdb::sql::Thing>,
    from_version: String,
    to_version: String,
    migration_data: serde_json::Value,
    description: String,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    author: Option<String>,
}

#[async_trait]
impl OntologyRepository for SurrealOntologyRepository {
    async fn store_schema(&self, schema: &OntologySchema) -> Result<()> {
        let record = SchemaRecord {
            id: None,
            version: schema.version.to_string(),
            name: schema.name.clone(),
            description: schema.description.clone(),
            schema_data: serde_json::to_value(schema)
                .map_err(|e| OtlError::Other(anyhow::anyhow!("Serialization error: {e}")))?,
            created_at: Some(schema.created_at),
            updated_at: Some(schema.updated_at),
            author: schema.author.clone(),
        };

        let _: Option<SchemaRecord> = self
            .client
            .create(("ontology_schema", schema.id.to_string()))
            .content(record)
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to store ontology schema: {e}"))
            })?;

        Ok(())
    }

    async fn get_current_schema(&self) -> Result<Option<OntologySchema>> {
        // Query for the latest schema by version
        let mut result = self
            .client
            .query("SELECT * FROM ontology_schema ORDER BY created_at DESC LIMIT 1")
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to query current schema: {e}"))
            })?;

        let records: Vec<SchemaRecord> = result.take(0).map_err(|e| {
            OtlError::DatabaseError(format!("Failed to extract schema records: {e}"))
        })?;

        if let Some(record) = records.first() {
            let schema: OntologySchema = serde_json::from_value(record.schema_data.clone())
                .map_err(|e| OtlError::Other(anyhow::anyhow!("Deserialization error: {e}")))?;
            Ok(Some(schema))
        } else {
            Ok(None)
        }
    }

    async fn get_schema_version(&self, version: &semver::Version) -> Result<Option<OntologySchema>> {
        let version_str = version.to_string();
        let mut result = self
            .client
            .query("SELECT * FROM ontology_schema WHERE version = $version LIMIT 1")
            .bind(("version", version_str))
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to query schema version: {e}"))
            })?;

        let records: Vec<SchemaRecord> = result.take(0).map_err(|e| {
            OtlError::DatabaseError(format!("Failed to extract schema records: {e}"))
        })?;

        if let Some(record) = records.first() {
            let schema: OntologySchema = serde_json::from_value(record.schema_data.clone())
                .map_err(|e| OtlError::Other(anyhow::anyhow!("Deserialization error: {e}")))?;
            Ok(Some(schema))
        } else {
            Ok(None)
        }
    }

    async fn list_schema_versions(&self) -> Result<Vec<SchemaVersionInfo>> {
        let mut result = self
            .client
            .query("SELECT version, name, created_at, author FROM ontology_schema ORDER BY created_at DESC")
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to list schema versions: {e}"))
            })?;

        let records: Vec<SchemaRecord> = result.take(0).map_err(|e| {
            OtlError::DatabaseError(format!("Failed to extract schema records: {e}"))
        })?;

        let mut versions = Vec::new();
        for record in records {
            let version = semver::Version::parse(&record.version).map_err(|e| {
                OtlError::Other(anyhow::anyhow!("Invalid version string: {e}"))
            })?;

            versions.push(SchemaVersionInfo {
                version,
                name: record.name,
                created_at: record.created_at.unwrap_or_default(),
                author: record.author,
            });
        }

        Ok(versions)
    }

    async fn store_migration(&self, migration: &SchemaMigration) -> Result<()> {
        let record = MigrationRecord {
            id: None,
            from_version: migration.from_version.to_string(),
            to_version: migration.to_version.to_string(),
            migration_data: serde_json::to_value(migration)
                .map_err(|e| OtlError::Other(anyhow::anyhow!("Serialization error: {e}")))?,
            description: migration.description.clone(),
            created_at: Some(migration.created_at),
            author: migration.author.clone(),
        };

        let _: Option<MigrationRecord> = self
            .client
            .create(("schema_migration", migration.id.to_string()))
            .content(record)
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to store migration: {e}"))
            })?;

        Ok(())
    }

    async fn list_migrations(&self) -> Result<Vec<SchemaMigration>> {
        let mut result = self
            .client
            .query("SELECT * FROM schema_migration ORDER BY created_at DESC")
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to list migrations: {e}"))
            })?;

        let records: Vec<MigrationRecord> = result.take(0).map_err(|e| {
            OtlError::DatabaseError(format!("Failed to extract migration records: {e}"))
        })?;

        let mut migrations = Vec::new();
        for record in records {
            let migration: SchemaMigration = serde_json::from_value(record.migration_data)
                .map_err(|e| OtlError::Other(anyhow::anyhow!("Deserialization error: {e}")))?;
            migrations.push(migration);
        }

        Ok(migrations)
    }

    async fn get_migration(
        &self,
        from_version: &semver::Version,
        to_version: &semver::Version,
    ) -> Result<Option<SchemaMigration>> {
        let from_str = from_version.to_string();
        let to_str = to_version.to_string();

        let mut result = self
            .client
            .query("SELECT * FROM schema_migration WHERE from_version = $from AND to_version = $to LIMIT 1")
            .bind(("from", from_str))
            .bind(("to", to_str))
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to query migration: {e}"))
            })?;

        let records: Vec<MigrationRecord> = result.take(0).map_err(|e| {
            OtlError::DatabaseError(format!("Failed to extract migration records: {e}"))
        })?;

        if let Some(record) = records.first() {
            let migration: SchemaMigration = serde_json::from_value(record.migration_data.clone())
                .map_err(|e| OtlError::Other(anyhow::anyhow!("Deserialization error: {e}")))?;
            Ok(Some(migration))
        } else {
            Ok(None)
        }
    }

    async fn delete_schema_version(&self, version: &semver::Version) -> Result<()> {
        let version_str = version.to_string();

        self.client
            .query("DELETE FROM ontology_schema WHERE version = $version")
            .bind(("version", version_str))
            .await
            .map_err(|e| {
                OtlError::DatabaseError(format!("Failed to delete schema version: {e}"))
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require a running SurrealDB instance
    // They are placeholders for integration tests

    #[test]
    fn test_schema_record_serialization() {
        use crate::ontology::{EntityTypeDef, OntologySchema};

        let mut schema = OntologySchema::new("Test", semver::Version::new(1, 0, 0));
        schema.add_entity_type(EntityTypeDef::new("Person", "个人")).unwrap();

        let schema_json = serde_json::to_value(&schema).unwrap();
        let record = SchemaRecord {
            id: None,
            version: schema.version.to_string(),
            name: schema.name.clone(),
            description: schema.description.clone(),
            schema_data: schema_json,
            created_at: Some(schema.created_at),
            updated_at: Some(schema.updated_at),
            author: None,
        };

        // Test that we can serialize and deserialize
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: SchemaRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, "1.0.0");
    }
}
