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
use base64::Engine;
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

    // Generate document ID
    let doc_id = Uuid::new_v4();

    // Decode base64 content
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.content)
        .map_err(|e| AppError::BadRequest(format!("Invalid base64 content: {e}")))?;

    // Convert to UTF-8 string
    let text_content = String::from_utf8(decoded_bytes)
        .map_err(|e| AppError::BadRequest(format!("Content is not valid UTF-8: {e}")))?;

    tracing::info!(
        "Processing document upload: {} (id: {}, size: {} bytes)",
        req.title,
        doc_id,
        text_content.len()
    );

    // Chunk the document
    let chunk_config = otl_parser::ChunkConfig {
        chunk_size: 1000,
        overlap: 200,
        min_chunk_size: 100,
        respect_sections: true,
        respect_paragraphs: true,
    };

    let chunks = chunk_text_simple(&text_content, &chunk_config);
    let chunk_count = chunks.len() as u32;

    tracing::info!(
        "Document {} split into {} chunks",
        doc_id,
        chunk_count
    );

    // Get vector backend and process chunks
    let vector_backend_guard = state.vector_backend.read().await;
    if let Some(vector_backend) = vector_backend_guard.as_ref() {
        // Clone the Arc to avoid holding the lock during async operations
        let backend = vector_backend.clone();
        drop(vector_backend_guard); // Release lock before async operations

        // Process chunks and store embeddings
        let mut processed_count = 0;
        for (index, chunk_text) in chunks.iter().enumerate() {
            match backend
                .index_text(doc_id, index as u32, chunk_text)
                .await
            {
                Ok(vector_id) => {
                    processed_count += 1;
                    tracing::debug!(
                        "Indexed chunk {} of document {} with vector_id {}",
                        index,
                        doc_id,
                        vector_id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to index chunk {} of document {}: {}",
                        index,
                        doc_id,
                        e
                    );
                }
            }
        }

        tracing::info!(
            "Successfully indexed {}/{} chunks for document {}",
            processed_count,
            chunk_count,
            doc_id
        );

        let response = UploadDocumentResponse {
            id: doc_id,
            message: format!(
                "Document uploaded and processed: {processed_count}/{chunk_count} chunks indexed"
            ),
            chunk_count: processed_count,
        };

        Ok((StatusCode::CREATED, Json(response)))
    } else {
        // Vector backend not initialized
        tracing::warn!("Vector backend not initialized, document upload not processed");

        let response = UploadDocumentResponse {
            id: doc_id,
            message: "Document received but vector store not available for indexing".to_string(),
            chunk_count: 0,
        };

        Ok((StatusCode::CREATED, Json(response)))
    }
}

/// Simple text chunking function with proper UTF-8 handling
fn chunk_text_simple(text: &str, config: &otl_parser::ChunkConfig) -> Vec<String> {
    let mut chunks = Vec::new();

    if text.len() <= config.chunk_size {
        chunks.push(text.to_string());
        return chunks;
    }

    let mut start = 0;
    while start < text.len() {
        // Calculate target end position (ensuring char boundary)
        let target_end = (start + config.chunk_size).min(text.len());
        let end = find_char_boundary(text, target_end);

        // Find a good break point (sentence or paragraph boundary)
        let actual_end = find_chunk_boundary(text, start, end, config.respect_paragraphs);

        // Ensure start is on char boundary
        let safe_start = find_char_boundary(text, start);
        let chunk_text = &text[safe_start..actual_end];

        if chunk_text.len() >= config.min_chunk_size {
            chunks.push(chunk_text.to_string());
        }

        if actual_end >= text.len() {
            break;
        }

        // Move start with overlap (ensuring char boundary)
        let overlap_pos = if actual_end > config.overlap {
            actual_end - config.overlap
        } else {
            actual_end
        };
        start = find_char_boundary(text, overlap_pos);
    }

    chunks
}

/// Find the nearest valid UTF-8 character boundary at or before the given position
fn find_char_boundary(text: &str, pos: usize) -> usize {
    if pos >= text.len() {
        return text.len();
    }
    let mut boundary = pos;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    boundary
}

/// Find a good boundary for chunking (respecting sentence/paragraph boundaries)
fn find_chunk_boundary(text: &str, _start: usize, target: usize, respect_paragraphs: bool) -> usize {
    if target >= text.len() {
        return text.len();
    }

    // Ensure target is on a valid char boundary
    let target = find_char_boundary(text, target);

    if !respect_paragraphs {
        return target;
    }

    // Search window around target position (ensure boundaries are valid)
    let search_start = find_char_boundary(text, if target > 100 { target - 100 } else { 0 });
    let search_end = find_char_boundary(text, (target + 100).min(text.len()));
    let search_text = &text[search_start..search_end];

    // Look for paragraph break (double newline)
    if let Some(pos) = search_text.rfind("\n\n") {
        return (search_start + pos + 2).min(text.len());
    }

    // Look for sentence endings
    for pattern in [". ", "。", "! ", "? ", ".\n", "。\n", "!\n", "?\n"] {
        if let Some(pos) = search_text.rfind(pattern) {
            return (search_start + pos + pattern.len()).min(text.len());
        }
    }

    // Look for single newline
    if let Some(pos) = search_text.rfind('\n') {
        return (search_start + pos + 1).min(text.len());
    }

    // Fall back to target position
    target
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
