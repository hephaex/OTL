//! Document tagging handlers
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
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// Tag information
#[derive(Debug, Serialize, ToSchema)]
pub struct TagInfo {
    /// Tag ID
    pub id: Uuid,

    /// Tag name
    pub name: String,

    /// Category (e.g., 'language', 'department', 'topic')
    pub category: Option<String>,

    /// Description
    pub description: Option<String>,

    /// Color (hex format)
    pub color: Option<String>,
}

/// Auto-tag document request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AutoTagRequest {
    /// Minimum confidence threshold (0.0-1.0)
    #[serde(default = "default_confidence")]
    pub min_confidence: f32,
}

fn default_confidence() -> f32 {
    0.7
}

/// Auto-tag response
#[derive(Debug, Serialize, ToSchema)]
pub struct AutoTagResponse {
    /// Document ID
    pub document_id: Uuid,

    /// Tags assigned
    pub tags_assigned: Vec<TagAssignmentInfo>,

    /// Detected language
    pub detected_language: Option<LanguageInfo>,
}

/// Tag assignment info
#[derive(Debug, Serialize, ToSchema)]
pub struct TagAssignmentInfo {
    /// Tag ID
    pub tag_id: Uuid,

    /// Tag name
    pub tag_name: String,

    /// Confidence score
    pub confidence: f32,

    /// Whether it was auto-assigned
    pub auto_assigned: bool,
}

/// Language detection info
#[derive(Debug, Serialize, ToSchema)]
pub struct LanguageInfo {
    /// Language code (ISO 639-1)
    pub code: String,

    /// Language name
    pub name: String,

    /// Confidence score
    pub confidence: f32,
}

/// Auto-tag a document
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/tags/auto",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document UUID")
    ),
    request_body = AutoTagRequest,
    responses(
        (status = 200, description = "Document auto-tagged", body = AutoTagResponse),
        (status = 404, description = "Document not found", body = crate::error::ApiError)
    )
)]
pub async fn auto_tag_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<AutoTagRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Check if document exists
    let _exists: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM documents WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to check document: {e}")))?;

    if _exists.is_none() {
        return Err(AppError::NotFound(format!("Document {id} not found")));
    }

    // TODO: Implement actual auto-tagging logic
    // This would involve:
    // 1. Fetch document content
    // 2. Detect language using whatlang or lingua
    // 3. Extract keywords/topics
    // 4. Match against existing tags
    // 5. Create new tag assignments with confidence scores
    // 6. Update document's detected_language field

    let min_conf = req.min_confidence;
    tracing::info!(
        "Auto-tagging document {} with min confidence {}",
        id,
        min_conf
    );

    // Placeholder response
    let response = AutoTagResponse {
        document_id: id,
        tags_assigned: vec![],
        detected_language: None,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Get document tags
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/tags",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document UUID")
    ),
    responses(
        (status = 200, description = "Document tags", body = Vec<TagAssignmentInfo>),
        (status = 404, description = "Document not found", body = crate::error::ApiError)
    )
)]
pub async fn get_document_tags(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    #[derive(sqlx::FromRow)]
    struct TagRow {
        tag_id: Uuid,
        tag_name: String,
        auto_assigned: bool,
        confidence: Option<f32>,
    }

    let rows: Vec<TagRow> = sqlx::query_as(
        "SELECT dta.tag_id, dt.name as tag_name, dta.auto_assigned, dta.confidence
         FROM document_tag_assignments dta
         JOIN document_tags dt ON dta.tag_id = dt.id
         WHERE dta.document_id = $1",
    )
    .bind(id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch tags: {e}")))?;

    let tags: Vec<TagAssignmentInfo> = rows
        .into_iter()
        .map(|row| TagAssignmentInfo {
            tag_id: row.tag_id,
            tag_name: row.tag_name,
            confidence: row.confidence.unwrap_or(1.0),
            auto_assigned: row.auto_assigned,
        })
        .collect();

    Ok((StatusCode::OK, Json(tags)))
}

/// Manual tag assignment request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignTagsRequest {
    /// Tag IDs to assign
    pub tag_ids: Vec<Uuid>,
}

/// Assign tags to a document
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/tags",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document UUID")
    ),
    request_body = AssignTagsRequest,
    responses(
        (status = 200, description = "Tags assigned"),
        (status = 404, description = "Document not found", body = crate::error::ApiError)
    )
)]
pub async fn assign_tags(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
    Json(_req): Json<AssignTagsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement manual tag assignment
    Err(AppError::Internal(
        "Tag assignment not yet implemented".to_string(),
    ))
}
