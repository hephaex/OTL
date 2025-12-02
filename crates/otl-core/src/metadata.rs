//! PostgreSQL metadata store
//!
//! Provides document metadata and ACL management using SQLx and PostgreSQL.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{AccessLevel, DocumentAcl, DocumentChunk, DocumentMetadata, OtlError, Result};

/// PostgreSQL metadata store
pub struct MetadataStore {
    pool: PgPool,
}

impl MetadataStore {
    /// Create a new metadata store connection
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("PostgreSQL connection failed: {e}")))?;

        Ok(Self { pool })
    }

    /// Create from an existing pool
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Document row from database
#[derive(Debug, FromRow)]
struct DocumentRow {
    id: Uuid,
    title: String,
    file_path: String,
    file_type: String,
    file_size: i64,
    access_level: String,
    owner_id: Option<String>,
    department: Option<String>,
    required_roles: Vec<String>,
    allowed_users: Vec<String>,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<DocumentRow> for DocumentMetadata {
    fn from(row: DocumentRow) -> Self {
        let acl = DocumentAcl {
            access_level: match row.access_level.as_str() {
                "public" => AccessLevel::Public,
                "confidential" => AccessLevel::Confidential,
                "restricted" => AccessLevel::Restricted,
                _ => AccessLevel::Internal,
            },
            owner_id: row.owner_id,
            department: row.department,
            required_roles: row.required_roles,
            allowed_users: row.allowed_users,
        };

        let mut metadata = DocumentMetadata::new(&row.title, &row.file_path, &row.file_type);
        metadata.id = row.id;
        metadata.file_size = row.file_size as u64;
        metadata.acl = acl;
        metadata.created_at = row.created_at;
        metadata.updated_at = row.updated_at;

        if let Some(obj) = row.metadata.as_object() {
            for (k, v) in obj {
                metadata.extra.insert(k.clone(), v.clone());
            }
        }

        metadata
    }
}

/// Document chunk row from database
#[derive(Debug, FromRow)]
struct ChunkRow {
    id: Uuid,
    document_id: Uuid,
    chunk_index: i32,
    content: String,
    page_number: Option<i32>,
    section_name: Option<String>,
    vector_id: Option<String>,
}

impl From<ChunkRow> for DocumentChunk {
    fn from(row: ChunkRow) -> Self {
        DocumentChunk {
            id: row.id,
            document_id: row.document_id,
            chunk_index: row.chunk_index as u32,
            content: row.content,
            page_number: row.page_number.map(|n| n as u32),
            section_name: row.section_name,
            vector_id: row.vector_id,
        }
    }
}

/// Trait for metadata operations
#[async_trait]
pub trait MetadataRepository: Send + Sync {
    /// Store document metadata
    async fn create_document(&self, doc: &DocumentMetadata) -> Result<Uuid>;

    /// Get document by ID
    async fn get_document(&self, id: Uuid) -> Result<Option<DocumentMetadata>>;

    /// List documents with optional filters
    async fn list_documents(&self, limit: i64, offset: i64) -> Result<Vec<DocumentMetadata>>;

    /// Update document metadata
    async fn update_document(&self, doc: &DocumentMetadata) -> Result<()>;

    /// Delete document (soft delete)
    async fn delete_document(&self, id: Uuid) -> Result<()>;

    /// Store document chunk
    async fn create_chunk(&self, chunk: &DocumentChunk) -> Result<Uuid>;

    /// Get chunks for a document
    async fn get_chunks(&self, document_id: Uuid) -> Result<Vec<DocumentChunk>>;

    /// Update chunk with vector ID
    async fn update_chunk_vector_id(&self, chunk_id: Uuid, vector_id: &str) -> Result<()>;
}

#[async_trait]
impl MetadataRepository for MetadataStore {
    async fn create_document(&self, doc: &DocumentMetadata) -> Result<Uuid> {
        let access_level = doc.acl.access_level.to_string();
        let metadata_json =
            serde_json::to_value(&doc.extra).unwrap_or(serde_json::Value::Object(Default::default()));

        let row: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO documents (
                id, title, file_path, file_type, file_size,
                access_level, owner_id, department, required_roles, allowed_users,
                metadata
            ) VALUES (
                $1, $2, $3, $4::file_type, $5,
                $6::access_level, $7, $8, $9, $10,
                $11
            )
            RETURNING id
            "#,
        )
        .bind(doc.id)
        .bind(&doc.title)
        .bind(&doc.file_path)
        .bind(&doc.file_type)
        .bind(doc.file_size as i64)
        .bind(&access_level)
        .bind(&doc.acl.owner_id)
        .bind(&doc.acl.department)
        .bind(&doc.acl.required_roles)
        .bind(&doc.acl.allowed_users)
        .bind(&metadata_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to create document: {e}")))?;

        Ok(row.0)
    }

    async fn get_document(&self, id: Uuid) -> Result<Option<DocumentMetadata>> {
        let row: Option<DocumentRow> = sqlx::query_as(
            r#"
            SELECT
                id, title, file_path, file_type::text, file_size,
                access_level::text, owner_id, department, required_roles, allowed_users,
                metadata, created_at, updated_at
            FROM documents
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to get document: {e}")))?;

        Ok(row.map(DocumentMetadata::from))
    }

    async fn list_documents(&self, limit: i64, offset: i64) -> Result<Vec<DocumentMetadata>> {
        let rows: Vec<DocumentRow> = sqlx::query_as(
            r#"
            SELECT
                id, title, file_path, file_type::text, file_size,
                access_level::text, owner_id, department, required_roles, allowed_users,
                metadata, created_at, updated_at
            FROM documents
            WHERE deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to list documents: {e}")))?;

        Ok(rows.into_iter().map(DocumentMetadata::from).collect())
    }

    async fn update_document(&self, doc: &DocumentMetadata) -> Result<()> {
        let access_level = doc.acl.access_level.to_string();
        let metadata_json =
            serde_json::to_value(&doc.extra).unwrap_or(serde_json::Value::Object(Default::default()));

        sqlx::query(
            r#"
            UPDATE documents SET
                title = $2,
                file_path = $3,
                file_type = $4::file_type,
                file_size = $5,
                access_level = $6::access_level,
                owner_id = $7,
                department = $8,
                required_roles = $9,
                allowed_users = $10,
                metadata = $11,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(doc.id)
        .bind(&doc.title)
        .bind(&doc.file_path)
        .bind(&doc.file_type)
        .bind(doc.file_size as i64)
        .bind(&access_level)
        .bind(&doc.acl.owner_id)
        .bind(&doc.acl.department)
        .bind(&doc.acl.required_roles)
        .bind(&doc.acl.allowed_users)
        .bind(&metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to update document: {e}")))?;

        Ok(())
    }

    async fn delete_document(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE documents SET deleted_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to delete document: {e}")))?;

        Ok(())
    }

    async fn create_chunk(&self, chunk: &DocumentChunk) -> Result<Uuid> {
        let row: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO document_chunks (
                id, document_id, chunk_index, content,
                page_number, section_name, vector_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(chunk.id)
        .bind(chunk.document_id)
        .bind(chunk.chunk_index as i32)
        .bind(&chunk.content)
        .bind(chunk.page_number.map(|n| n as i32))
        .bind(&chunk.section_name)
        .bind(&chunk.vector_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to create chunk: {e}")))?;

        Ok(row.0)
    }

    async fn get_chunks(&self, document_id: Uuid) -> Result<Vec<DocumentChunk>> {
        let rows: Vec<ChunkRow> = sqlx::query_as(
            r#"
            SELECT id, document_id, chunk_index, content, page_number, section_name, vector_id
            FROM document_chunks
            WHERE document_id = $1
            ORDER BY chunk_index
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to get chunks: {e}")))?;

        Ok(rows.into_iter().map(DocumentChunk::from).collect())
    }

    async fn update_chunk_vector_id(&self, chunk_id: Uuid, vector_id: &str) -> Result<()> {
        sqlx::query("UPDATE document_chunks SET vector_id = $2 WHERE id = $1")
            .bind(chunk_id)
            .bind(vector_id)
            .execute(&self.pool)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to update chunk: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_row_conversion() {
        // This would require a database to fully test
        // For now, just verify the types compile
        let _metadata = DocumentMetadata::new("Test", "/path/test.pdf", "pdf");
    }
}
