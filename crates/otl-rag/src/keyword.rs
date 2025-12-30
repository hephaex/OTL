//! Keyword Search Backend using PostgreSQL Full-Text Search
//!
//! This module implements a keyword search backend for hybrid RAG using:
//! - PostgreSQL tsvector for full-text indexing
//! - BM25-like scoring via ts_rank_cd
//! - Support for phrase matching, boolean operators, and fuzzy search
//! - Reciprocal Rank Fusion (RRF) integration
//!
//! Author: hephaex@gmail.com
//! Related Issue: #3 Full-text search / keyword search backend

use otl_core::{DocumentAcl, OtlError, Result, SearchBackend, SearchResult, SearchResultType, SourceReference};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for keyword search
#[derive(Debug, Clone)]
pub struct KeywordSearchConfig {
    /// BM25 parameter k1 (term saturation, typically 1.2-2.0)
    pub k1: f32,

    /// BM25 parameter b (length normalization, typically 0.75)
    pub b: f32,

    /// Minimum similarity threshold for fuzzy search (0.0-1.0)
    pub fuzzy_threshold: f32,

    /// Enable query logging for analytics
    pub enable_query_logging: bool,
}

impl Default for KeywordSearchConfig {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            fuzzy_threshold: 0.3,
            enable_query_logging: true,
        }
    }
}

/// Search mode for keyword search
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// Standard BM25-like search (default)
    Standard,

    /// Exact phrase matching
    Phrase,

    /// Boolean search with AND, OR, NOT operators
    Boolean,

    /// Fuzzy search using trigram similarity
    Fuzzy,
}

impl SearchMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "simple",
            Self::Phrase => "phrase",
            Self::Boolean => "boolean",
            Self::Fuzzy => "fuzzy",
        }
    }
}

// ============================================================================
// PostgreSQL Keyword Search Backend
// ============================================================================

/// PostgreSQL-based keyword search backend
pub struct PostgresKeywordSearch {
    pool: Arc<PgPool>,
    config: KeywordSearchConfig,
}

impl PostgresKeywordSearch {
    /// Create a new keyword search backend
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            config: KeywordSearchConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(pool: Arc<PgPool>, config: KeywordSearchConfig) -> Self {
        Self { pool, config }
    }

    /// Index a document chunk for keyword search
    pub async fn index_chunk(
        &self,
        document_id: Uuid,
        chunk_id: Uuid,
        content: &str,
    ) -> Result<()> {
        let content_length = content.len() as i32;
        let word_count = content.split_whitespace().count() as i32;

        sqlx::query(
            r#"
            INSERT INTO fts_documents (document_id, chunk_id, content, content_length, word_count)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (chunk_id) DO UPDATE
            SET content = EXCLUDED.content,
                content_length = EXCLUDED.content_length,
                word_count = EXCLUDED.word_count,
                updated_at = NOW()
            "#
        )
        .bind(document_id)
        .bind(chunk_id)
        .bind(content)
        .bind(content_length)
        .bind(word_count)
        .execute(&*self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to index chunk: {e}")))?;

        Ok(())
    }

    /// Index multiple chunks in a batch
    pub async fn index_chunks_batch(
        &self,
        chunks: Vec<(Uuid, Uuid, String)>, // (document_id, chunk_id, content)
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to begin transaction: {e}")))?;

        for (document_id, chunk_id, content) in chunks {
            let content_length = content.len() as i32;
            let word_count = content.split_whitespace().count() as i32;

            sqlx::query(
                r#"
                INSERT INTO fts_documents (document_id, chunk_id, content, content_length, word_count)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (chunk_id) DO UPDATE
                SET content = EXCLUDED.content,
                    content_length = EXCLUDED.content_length,
                    word_count = EXCLUDED.word_count,
                    updated_at = NOW()
                "#
            )
            .bind(document_id)
            .bind(chunk_id)
            .bind(&content)
            .bind(content_length)
            .bind(word_count)
            .execute(&mut *tx)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to index chunk in batch: {e}")))?;
        }

        tx.commit()
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to commit transaction: {e}")))?;

        Ok(())
    }

    /// Delete indexed chunks for a document
    pub async fn delete_document(&self, document_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM fts_documents WHERE document_id = $1")
            .bind(document_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to delete document: {e}")))?;

        Ok(result.rows_affected())
    }

    /// Search with a specific mode
    pub async fn search_with_mode(
        &self,
        query: &str,
        mode: SearchMode,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let start_time = std::time::Instant::now();

        let results = match mode {
            SearchMode::Standard => self.search_bm25(query, limit).await?,
            SearchMode::Phrase => self.search_phrase(query, limit).await?,
            SearchMode::Boolean => self.search_boolean(query, limit).await?,
            SearchMode::Fuzzy => self.search_fuzzy(query, limit).await?,
        };

        let execution_time_ms = start_time.elapsed().as_millis() as i32;

        // Log query if enabled
        if self.config.enable_query_logging {
            self.log_query(query, mode, &results, execution_time_ms)
                .await
                .ok(); // Don't fail on logging errors
        }

        Ok(results)
    }

    /// Standard BM25-like search using ts_rank_cd
    async fn search_bm25(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let limit_i32 = limit as i32;

        let rows = sqlx::query("SELECT * FROM search_bm25($1, $2, $3, $4)")
            .bind(query)
            .bind(limit_i32)
            .bind(self.config.k1)
            .bind(self.config.b)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| OtlError::SearchError(format!("BM25 search failed: {e}")))?;

        let mut results = Vec::new();

        for row in rows {
            let doc_id: Option<Uuid> = row.try_get("doc_id").ok();
            let doc_id = doc_id.ok_or_else(|| {
                OtlError::SearchError("Missing document_id in search result".to_string())
            })?;

            let content: String = row.try_get("content").unwrap_or_default();
            let score: f32 = row.try_get("score").unwrap_or(0.0);

            // Fetch ACL information from documents table
            let acl = self.fetch_document_acl(doc_id).await?;

            results.push(SearchResult {
                content,
                score,
                source: SourceReference::new(doc_id).with_confidence(score),
                acl,
                result_type: SearchResultType::Keyword,
            });
        }

        Ok(results)
    }

    /// Phrase search (exact phrase matching)
    async fn search_phrase(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let limit_i32 = limit as i32;

        let rows = sqlx::query("SELECT * FROM search_phrase($1, $2)")
            .bind(query)
            .bind(limit_i32)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| OtlError::SearchError(format!("Phrase search failed: {e}")))?;

        let mut results = Vec::new();

        for row in rows {
            let doc_id: Option<Uuid> = row.try_get("doc_id").ok();
            let doc_id = doc_id.ok_or_else(|| {
                OtlError::SearchError("Missing document_id in search result".to_string())
            })?;

            let content: String = row.try_get("content").unwrap_or_default();
            let score: f32 = row.try_get("score").unwrap_or(0.0);
            let acl = self.fetch_document_acl(doc_id).await?;

            results.push(SearchResult {
                content,
                score,
                source: SourceReference::new(doc_id).with_confidence(score),
                acl,
                result_type: SearchResultType::Keyword,
            });
        }

        Ok(results)
    }

    /// Boolean search with AND, OR, NOT operators
    async fn search_boolean(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let limit_i32 = limit as i32;

        // Convert natural language to tsquery format
        let tsquery = convert_to_tsquery(query);

        let rows = sqlx::query("SELECT * FROM search_boolean($1, $2)")
            .bind(tsquery)
            .bind(limit_i32)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| OtlError::SearchError(format!("Boolean search failed: {e}")))?;

        let mut results = Vec::new();

        for row in rows {
            let doc_id: Option<Uuid> = row.try_get("doc_id").ok();
            let doc_id = doc_id.ok_or_else(|| {
                OtlError::SearchError("Missing document_id in search result".to_string())
            })?;

            let content: String = row.try_get("content").unwrap_or_default();
            let score: f32 = row.try_get("score").unwrap_or(0.0);
            let acl = self.fetch_document_acl(doc_id).await?;

            results.push(SearchResult {
                content,
                score,
                source: SourceReference::new(doc_id).with_confidence(score),
                acl,
                result_type: SearchResultType::Keyword,
            });
        }

        Ok(results)
    }

    /// Fuzzy search using trigram similarity
    async fn search_fuzzy(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let limit_i32 = limit as i32;

        let rows = sqlx::query("SELECT * FROM search_fuzzy($1, $2, $3)")
            .bind(query)
            .bind(self.config.fuzzy_threshold)
            .bind(limit_i32)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| OtlError::SearchError(format!("Fuzzy search failed: {e}")))?;

        let mut results = Vec::new();

        for row in rows {
            let doc_id: Option<Uuid> = row.try_get("doc_id").ok();
            let doc_id = doc_id.ok_or_else(|| {
                OtlError::SearchError("Missing document_id in search result".to_string())
            })?;

            let content: String = row.try_get("content").unwrap_or_default();
            let score: f32 = row.try_get("score").unwrap_or(0.0);
            let acl = self.fetch_document_acl(doc_id).await?;

            results.push(SearchResult {
                content,
                score,
                source: SourceReference::new(doc_id).with_confidence(score),
                acl,
                result_type: SearchResultType::Keyword,
            });
        }

        Ok(results)
    }

    /// Fetch document ACL from database
    async fn fetch_document_acl(&self, document_id: Uuid) -> Result<DocumentAcl> {
        let row = sqlx::query(
            "SELECT access_level, owner_id, department, required_roles, allowed_users FROM documents WHERE id = $1"
        )
        .bind(document_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to fetch document ACL: {e}")))?;

        match row {
            Some(row) => {
                // Parse access level from string
                let access_level_str: String = row.try_get("access_level").unwrap_or_else(|_| "internal".to_string());
                let access_level = match access_level_str.as_str() {
                    "public" => otl_core::AccessLevel::Public,
                    "internal" => otl_core::AccessLevel::Internal,
                    "confidential" => otl_core::AccessLevel::Confidential,
                    "restricted" => otl_core::AccessLevel::Restricted,
                    _ => otl_core::AccessLevel::Internal,
                };

                let owner_id: Option<String> = row.try_get("owner_id").ok().flatten();
                let department: Option<String> = row.try_get("department").ok().flatten();
                let required_roles: Vec<String> = row.try_get("required_roles").unwrap_or_default();
                let allowed_users: Vec<String> = row.try_get("allowed_users").unwrap_or_default();

                Ok(DocumentAcl {
                    access_level,
                    owner_id,
                    department,
                    required_roles,
                    allowed_users,
                })
            }
            None => {
                // Document not found, return default ACL
                Ok(DocumentAcl::default())
            }
        }
    }

    /// Log search query for analytics
    async fn log_query(
        &self,
        query: &str,
        mode: SearchMode,
        results: &[SearchResult],
        execution_time_ms: i32,
    ) -> Result<()> {
        let num_results = results.len() as i32;
        let top_score = results.first().map(|r| r.score);

        sqlx::query(
            "INSERT INTO fts_query_log (query_text, search_type, num_results, execution_time_ms, top_score) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(query)
        .bind(mode.as_str())
        .bind(num_results)
        .bind(execution_time_ms)
        .bind(top_score)
        .execute(&*self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to log query: {e}")))?;

        Ok(())
    }

    /// Get search statistics
    pub async fn get_statistics(&self) -> Result<SearchStatistics> {
        let row = sqlx::query(
            "SELECT total_documents, avg_document_length, updated_at FROM fts_statistics WHERE id = 1"
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| OtlError::DatabaseError(format!("Failed to fetch statistics: {e}")))?;

        match row {
            Some(row) => {
                let total_documents: i64 = row.try_get("total_documents").unwrap_or(0);
                let avg_document_length: f32 = row.try_get("avg_document_length").unwrap_or(0.0);
                let last_updated: chrono::DateTime<chrono::Utc> =
                    row.try_get("updated_at").unwrap_or_else(|_| chrono::Utc::now());

                Ok(SearchStatistics {
                    total_documents: total_documents as u64,
                    avg_document_length,
                    last_updated,
                })
            }
            None => Ok(SearchStatistics::default()),
        }
    }

    /// Reindex all documents (maintenance operation)
    pub async fn reindex_all(&self) -> Result<()> {
        sqlx::query("SELECT reindex_fts_documents()")
            .execute(&*self.pool)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to reindex: {e}")))?;

        Ok(())
    }

    /// Clean up old query logs
    pub async fn cleanup_old_logs(&self) -> Result<u64> {
        let result = sqlx::query("SELECT cleanup_old_query_logs()")
            .execute(&*self.pool)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("Failed to cleanup logs: {e}")))?;

        Ok(result.rows_affected())
    }
}

// ============================================================================
// SearchBackend Implementation
// ============================================================================

#[async_trait::async_trait]
impl SearchBackend for PostgresKeywordSearch {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Auto-detect search mode based on query
        let mode = if query.contains('"') {
            SearchMode::Phrase
        } else if query.contains(" AND ") || query.contains(" OR ") || query.contains(" NOT ") {
            SearchMode::Boolean
        } else {
            SearchMode::Standard
        };

        self.search_with_mode(query, mode, limit).await
    }

    fn name(&self) -> &str {
        "PostgresKeywordSearch"
    }
}

// ============================================================================
// Statistics and Analytics
// ============================================================================

/// Search statistics
#[derive(Debug, Clone)]
pub struct SearchStatistics {
    pub total_documents: u64,
    pub avg_document_length: f32,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for SearchStatistics {
    fn default() -> Self {
        Self {
            total_documents: 0,
            avg_document_length: 0.0,
            last_updated: chrono::Utc::now(),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert natural language query to PostgreSQL tsquery format
///
/// Replaces AND, OR, NOT with PostgreSQL tsquery operators &, |, !
fn convert_to_tsquery(query: &str) -> String {
    query
        .replace(" AND ", " & ")
        .replace(" OR ", " | ")
        .replace(" NOT ", " ! ")
        .replace(" and ", " & ")
        .replace(" or ", " | ")
        .replace(" not ", " ! ")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsquery_conversion() {
        assert_eq!(
            convert_to_tsquery("machine AND learning"),
            "machine & learning"
        );

        assert_eq!(
            convert_to_tsquery("cat OR dog"),
            "cat | dog"
        );

        assert_eq!(
            convert_to_tsquery("rust NOT python"),
            "rust ! python"
        );
    }

    #[test]
    fn test_search_mode_as_str() {
        assert_eq!(SearchMode::Standard.as_str(), "simple");
        assert_eq!(SearchMode::Phrase.as_str(), "phrase");
        assert_eq!(SearchMode::Boolean.as_str(), "boolean");
        assert_eq!(SearchMode::Fuzzy.as_str(), "fuzzy");
    }
}
