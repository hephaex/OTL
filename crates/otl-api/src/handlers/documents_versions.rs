//! Document versioning handlers
//!
//! Author: hephaex@gmail.com

use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// Document version information
#[derive(Debug, Serialize, ToSchema)]
pub struct VersionInfo {
    /// Version ID
    pub id: Uuid,

    /// Version number
    pub version_number: i32,

    /// Document title at this version
    pub title: String,

    /// File size in bytes
    pub file_size: i64,

    /// File hash (SHA-256)
    pub file_hash: Option<String>,

    /// Change summary
    pub change_summary: Option<String>,

    /// Who created this version
    pub changed_by: Option<Uuid>,

    /// When this version was created
    pub created_at: DateTime<Utc>,
}

/// Version history response
#[derive(Debug, Serialize, ToSchema)]
pub struct VersionHistoryResponse {
    /// Document ID
    pub document_id: Uuid,

    /// Current version number
    pub current_version: i32,

    /// List of versions (newest first)
    pub versions: Vec<VersionInfo>,

    /// Total version count
    pub total_versions: usize,
}

/// Get version history for a document
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/versions",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document UUID")
    ),
    responses(
        (status = 200, description = "Version history", body = VersionHistoryResponse),
        (status = 404, description = "Document not found", body = crate::error::ApiError)
    )
)]
pub async fn get_version_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Check if document exists
    let current_version: Option<(i32,)> = sqlx::query_as(
        "SELECT current_version FROM documents WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to check document: {e}")))?;

    let current_version = current_version
        .ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?
        .0;

    // Fetch all versions
    #[derive(sqlx::FromRow)]
    struct VersionRow {
        id: Uuid,
        version_number: i32,
        title: String,
        file_size: i64,
        file_hash: Option<String>,
        change_summary: Option<String>,
        changed_by: Option<Uuid>,
        created_at: DateTime<Utc>,
    }

    let rows: Vec<VersionRow> = sqlx::query_as(
        "SELECT id, version_number, title, file_size, file_hash, change_summary, changed_by, created_at
         FROM document_versions
         WHERE document_id = $1
         ORDER BY version_number DESC",
    )
    .bind(id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch versions: {e}")))?;

    let versions: Vec<VersionInfo> = rows
        .into_iter()
        .map(|row| VersionInfo {
            id: row.id,
            version_number: row.version_number,
            title: row.title,
            file_size: row.file_size,
            file_hash: row.file_hash,
            change_summary: row.change_summary,
            changed_by: row.changed_by,
            created_at: row.created_at,
        })
        .collect();

    let response = VersionHistoryResponse {
        document_id: id,
        current_version,
        total_versions: versions.len(),
        versions,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Diff change type
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DiffType {
    Add,
    Delete,
    Modify,
    Equal,
}

/// Single diff change
#[derive(Debug, Serialize, ToSchema)]
pub struct DiffChange {
    /// Type of change
    pub change_type: DiffType,

    /// Old content (for delete/modify)
    pub old_content: Option<String>,

    /// New content (for add/modify)
    pub new_content: Option<String>,

    /// Line number in old version
    pub old_line: Option<usize>,

    /// Line number in new version
    pub new_line: Option<usize>,
}

/// Version comparison diff
#[derive(Debug, Serialize, ToSchema)]
pub struct VersionDiffResponse {
    /// Old version number
    pub old_version: i32,

    /// New version number
    pub new_version: i32,

    /// Number of additions
    pub additions: usize,

    /// Number of deletions
    pub deletions: usize,

    /// Number of modifications
    pub modifications: usize,

    /// List of changes (limited to first 100)
    pub changes: Vec<DiffChange>,

    /// Whether the diff was truncated
    pub truncated: bool,
}

/// Compare two document versions
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/versions/{old_version}/diff/{new_version}",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document UUID"),
        ("old_version" = i32, Path, description = "Old version number"),
        ("new_version" = i32, Path, description = "New version number")
    ),
    responses(
        (status = 200, description = "Version diff", body = VersionDiffResponse),
        (status = 404, description = "Version not found", body = crate::error::ApiError)
    )
)]
pub async fn compare_versions(
    State(state): State<Arc<AppState>>,
    Path((id, old_version, new_version)): Path<(Uuid, i32, i32)>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Fetch both versions
    #[derive(sqlx::FromRow)]
    struct VersionContent {
        version_number: i32,
        content_snapshot: Option<String>,
    }

    let versions: Vec<VersionContent> = sqlx::query_as(
        "SELECT version_number, content_snapshot
         FROM document_versions
         WHERE document_id = $1 AND version_number = ANY($2)",
    )
    .bind(id)
    .bind([old_version, new_version])
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch versions: {e}")))?;

    if versions.len() != 2 {
        return Err(AppError::NotFound("One or both versions not found".to_string()));
    }

    // Extract content
    let old_content = versions
        .iter()
        .find(|v| v.version_number == old_version)
        .and_then(|v| v.content_snapshot.as_ref())
        .ok_or_else(|| AppError::NotFound("Old version content not found".to_string()))?;

    let new_content = versions
        .iter()
        .find(|v| v.version_number == new_version)
        .and_then(|v| v.content_snapshot.as_ref())
        .ok_or_else(|| AppError::NotFound("New version content not found".to_string()))?;

    // Compute diff
    let diff = compute_line_diff(old_content, new_content);

    Ok((StatusCode::OK, Json(diff)))
}

/// Compute line-by-line diff between two texts
fn compute_line_diff(old: &str, new: &str) -> VersionDiffResponse {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut changes = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;
    let mut modifications = 0;

    // Simple line-by-line comparison (not using LCS for performance)
    let max_len = old_lines.len().max(new_lines.len());
    let limit = 100; // Limit to first 100 changes

    for i in 0..max_len {
        if changes.len() >= limit {
            break;
        }

        let old_line = old_lines.get(i);
        let new_line = new_lines.get(i);

        match (old_line, new_line) {
            (Some(&old), Some(&new)) if old == new => {
                // Equal lines - skip or include a few for context
                if changes.len() < 10 || changes.len() % 20 == 0 {
                    changes.push(DiffChange {
                        change_type: DiffType::Equal,
                        old_content: Some(old.to_string()),
                        new_content: Some(new.to_string()),
                        old_line: Some(i + 1),
                        new_line: Some(i + 1),
                    });
                }
            }
            (Some(&old), Some(&new)) => {
                // Modified line
                modifications += 1;
                changes.push(DiffChange {
                    change_type: DiffType::Modify,
                    old_content: Some(old.to_string()),
                    new_content: Some(new.to_string()),
                    old_line: Some(i + 1),
                    new_line: Some(i + 1),
                });
            }
            (Some(&old), None) => {
                // Deleted line
                deletions += 1;
                changes.push(DiffChange {
                    change_type: DiffType::Delete,
                    old_content: Some(old.to_string()),
                    new_content: None,
                    old_line: Some(i + 1),
                    new_line: None,
                });
            }
            (None, Some(&new)) => {
                // Added line
                additions += 1;
                changes.push(DiffChange {
                    change_type: DiffType::Add,
                    old_content: None,
                    new_content: Some(new.to_string()),
                    old_line: None,
                    new_line: Some(i + 1),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    let truncated = max_len > limit;

    VersionDiffResponse {
        old_version: 0, // Will be set by caller
        new_version: 0, // Will be set by caller
        additions,
        deletions,
        modifications,
        changes,
        truncated,
    }
}

/// Create new version request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateVersionRequest {
    /// Change summary
    pub change_summary: String,

    /// Updated content (base64 encoded)
    pub content: String,

    /// File hash (optional, will be computed if not provided)
    pub file_hash: Option<String>,
}

/// Create a new version of a document
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/versions",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document UUID")
    ),
    request_body = CreateVersionRequest,
    responses(
        (status = 201, description = "Version created", body = VersionInfo),
        (status = 404, description = "Document not found", body = crate::error::ApiError)
    )
)]
pub async fn create_version(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
    Json(_req): Json<CreateVersionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement version creation
    // This would involve:
    // 1. Decode content
    // 2. Extract text
    // 3. Compute hash
    // 4. Call create_document_version() function
    // 5. Update document's current_version and latest_version_id
    // 6. Re-index chunks if content changed

    Err(AppError::Internal(
        "Version creation not yet implemented".to_string(),
    ))
}
