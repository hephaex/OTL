//! Document lineage/processing history handlers
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
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// Processing step information
#[derive(Debug, Serialize, ToSchema)]
pub struct ProcessingStepInfo {
    /// Step ID
    pub id: Uuid,

    /// Step name (e.g., 'upload', 'parse', 'chunk', 'embed')
    pub step_name: String,

    /// Step type (e.g., 'ingestion', 'processing', 'extraction')
    pub step_type: String,

    /// Status (pending, running, completed, failed)
    pub status: String,

    /// When started
    pub started_at: DateTime<Utc>,

    /// When completed
    pub completed_at: Option<DateTime<Utc>>,

    /// Duration in milliseconds
    pub duration_ms: Option<i32>,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Processing metrics
    pub metrics: serde_json::Value,

    /// Who triggered this step
    pub triggered_by: Option<Uuid>,
}

/// Document lineage response
#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentLineageResponse {
    /// Document ID
    pub document_id: Uuid,

    /// Processing history (chronological order)
    pub history: Vec<ProcessingStepInfo>,

    /// Total steps
    pub total_steps: usize,

    /// Latest step status
    pub latest_status: Option<String>,
}

/// Get processing lineage for a document
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/lineage",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document UUID")
    ),
    responses(
        (status = 200, description = "Document lineage", body = DocumentLineageResponse),
        (status = 404, description = "Document not found", body = crate::error::ApiError)
    )
)]
pub async fn get_document_lineage(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
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

    // Fetch lineage records
    #[derive(sqlx::FromRow)]
    struct LineageRow {
        id: Uuid,
        step_name: String,
        step_type: String,
        status: String,
        started_at: DateTime<Utc>,
        completed_at: Option<DateTime<Utc>>,
        duration_ms: Option<i32>,
        error_message: Option<String>,
        metrics: serde_json::Value,
        triggered_by: Option<Uuid>,
    }

    let rows: Vec<LineageRow> = sqlx::query_as(
        "SELECT id, step_name, step_type, status, started_at, completed_at,
                duration_ms, error_message, metrics, triggered_by
         FROM document_lineage
         WHERE document_id = $1
         ORDER BY started_at ASC",
    )
    .bind(id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch lineage: {e}")))?;

    let latest_status = rows.last().map(|r| r.status.clone());

    let history: Vec<ProcessingStepInfo> = rows
        .into_iter()
        .map(|row| ProcessingStepInfo {
            id: row.id,
            step_name: row.step_name,
            step_type: row.step_type,
            status: row.status,
            started_at: row.started_at,
            completed_at: row.completed_at,
            duration_ms: row.duration_ms,
            error_message: row.error_message,
            metrics: row.metrics,
            triggered_by: row.triggered_by,
        })
        .collect();

    let response = DocumentLineageResponse {
        document_id: id,
        total_steps: history.len(),
        latest_status,
        history,
    };

    Ok((StatusCode::OK, Json(response)))
}
