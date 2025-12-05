//! Document management handlers
//!
//! Author: hephaex@gmail.com

use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Document information
#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentInfo {
    /// Document UUID
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: Uuid,

    /// Document title
    #[schema(example = "인사규정_2024.pdf")]
    pub title: String,

    /// File type
    #[schema(example = "pdf")]
    pub file_type: String,

    /// Access level
    #[schema(example = "internal")]
    pub access_level: String,

    /// Owner department
    #[schema(example = "인사팀")]
    pub department: Option<String>,

    /// Creation timestamp
    pub created_at: String,

    /// Last updated timestamp
    pub updated_at: String,

    /// Number of chunks
    #[schema(example = 45)]
    pub chunk_count: u32,
}

/// Document list response
#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentListResponse {
    /// List of documents
    pub documents: Vec<DocumentInfo>,

    /// Total count
    pub total: usize,

    /// Current page
    pub page: u32,

    /// Page size
    pub page_size: u32,
}

/// Query parameters for document listing
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListDocumentsQuery {
    /// Page number (1-indexed)
    #[param(default = 1)]
    pub page: Option<u32>,

    /// Items per page
    #[param(default = 20)]
    pub page_size: Option<u32>,

    /// Filter by file type
    pub file_type: Option<String>,

    /// Filter by department
    pub department: Option<String>,

    /// Search in title
    pub search: Option<String>,
}

/// List documents with filtering
#[utoipa::path(
    get,
    path = "/api/v1/documents",
    tag = "documents",
    params(ListDocumentsQuery),
    responses(
        (status = 200, description = "Document list", body = DocumentListResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ApiError)
    )
)]
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListDocumentsQuery>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    // TODO: Implement actual document listing from database
    // For now, return mock data
    let documents = vec![
        DocumentInfo {
            id: Uuid::new_v4(),
            title: "인사규정_2024.pdf".to_string(),
            file_type: "pdf".to_string(),
            access_level: "internal".to_string(),
            department: Some("인사팀".to_string()),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:00:00Z".to_string(),
            chunk_count: 45,
        },
        DocumentInfo {
            id: Uuid::new_v4(),
            title: "휴가신청_매뉴얼.docx".to_string(),
            file_type: "docx".to_string(),
            access_level: "internal".to_string(),
            department: Some("인사팀".to_string()),
            created_at: "2024-02-20T14:30:00Z".to_string(),
            updated_at: "2024-02-20T14:30:00Z".to_string(),
            chunk_count: 12,
        },
    ];

    let response = DocumentListResponse {
        total: documents.len(),
        documents,
        page,
        page_size,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Get single document by ID
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document UUID")
    ),
    responses(
        (status = 200, description = "Document details", body = DocumentInfo),
        (status = 404, description = "Document not found", body = crate::error::ApiError)
    )
)]
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // TODO: Implement actual document retrieval
    // For now, return mock data
    let doc = DocumentInfo {
        id,
        title: "인사규정_2024.pdf".to_string(),
        file_type: "pdf".to_string(),
        access_level: "internal".to_string(),
        department: Some("인사팀".to_string()),
        created_at: "2024-01-15T10:00:00Z".to_string(),
        updated_at: "2024-01-15T10:00:00Z".to_string(),
        chunk_count: 45,
    };

    Ok((StatusCode::OK, Json(doc)))
}

/// Upload document request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadDocumentRequest {
    /// Document title
    #[schema(example = "신규입사자_안내서.pdf")]
    pub title: String,

    /// Base64 encoded file content
    pub content: String,

    /// File type
    #[schema(example = "pdf")]
    pub file_type: String,

    /// Access level
    #[schema(example = "internal")]
    pub access_level: Option<String>,

    /// Owner department
    #[schema(example = "인사팀")]
    pub department: Option<String>,
}

/// Upload document response
#[derive(Debug, Serialize)]
pub struct UploadDocumentResponse {
    pub id: Uuid,
    pub message: String,
    pub chunk_count: u32,
}

/// Upload a new document
#[utoipa::path(
    post,
    path = "/api/v1/documents",
    tag = "documents",
    request_body = UploadDocumentRequest,
    responses(
        (status = 201, description = "Document uploaded successfully"),
        (status = 400, description = "Invalid request", body = crate::error::ApiError)
    )
)]
pub async fn upload_document(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UploadDocumentRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Validate request
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title cannot be empty".to_string()));
    }

    if req.content.is_empty() {
        return Err(AppError::BadRequest("Content cannot be empty".to_string()));
    }

    // TODO: Implement actual document upload and processing
    let doc_id = Uuid::new_v4();

    let response = UploadDocumentResponse {
        id: doc_id,
        message: "Document uploaded and queued for processing".to_string(),
        chunk_count: 0, // Will be updated after processing
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Delete document response
#[derive(Debug, Serialize)]
pub struct DeleteDocumentResponse {
    pub message: String,
}

/// Delete a document
#[utoipa::path(
    delete,
    path = "/api/v1/documents/{id}",
    tag = "documents",
    params(
        ("id" = Uuid, Path, description = "Document UUID")
    ),
    responses(
        (status = 200, description = "Document deleted"),
        (status = 404, description = "Document not found", body = crate::error::ApiError)
    )
)]
pub async fn delete_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // TODO: Implement actual document deletion
    tracing::info!("Deleting document: {}", id);

    Ok((
        StatusCode::OK,
        Json(DeleteDocumentResponse {
            message: format!("Document {id} deleted"),
        }),
    ))
}
