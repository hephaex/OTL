// ! Bulk document operations handlers
//!
//! Author: hephaex@gmail.com

use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use base64::Engine;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// Single document in a bulk upload request
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkUploadDocument {
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

    /// Primary collection ID (optional)
    pub collection_id: Option<Uuid>,

    /// Tags to assign (optional)
    pub tags: Option<Vec<String>>,
}

/// Bulk upload request
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkUploadRequest {
    /// List of documents to upload
    pub documents: Vec<BulkUploadDocument>,

    /// Maximum parallel uploads (default: 4)
    pub parallel_limit: Option<usize>,
}

/// Result of a single document upload in bulk operation
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkUploadResult {
    /// Document title
    pub title: String,

    /// Success status
    pub success: bool,

    /// Document ID if successful
    pub id: Option<Uuid>,

    /// Chunk count if successful
    pub chunk_count: Option<u32>,

    /// Error message if failed
    pub error: Option<String>,
}

/// Bulk upload response
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkUploadResponse {
    /// Total documents in request
    pub total: usize,

    /// Successfully uploaded
    pub successful: usize,

    /// Failed uploads
    pub failed: usize,

    /// Detailed results for each document
    pub results: Vec<BulkUploadResult>,

    /// Overall processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Bulk upload documents
#[utoipa::path(
    post,
    path = "/api/v1/documents/bulk",
    tag = "documents",
    request_body = BulkUploadRequest,
    responses(
        (status = 200, description = "Bulk upload completed (with partial failures possible)", body = BulkUploadResponse),
        (status = 400, description = "Invalid request", body = crate::error::ApiError)
    )
)]
pub async fn bulk_upload_documents(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BulkUploadRequest>,
) -> Result<impl IntoResponse, AppError> {
    let start_time = std::time::Instant::now();
    state.increment_requests();

    // Validate request
    if req.documents.is_empty() {
        return Err(AppError::BadRequest(
            "No documents provided in bulk upload".to_string(),
        ));
    }

    if req.documents.len() > 100 {
        return Err(AppError::BadRequest(
            "Bulk upload limited to 100 documents per request".to_string(),
        ));
    }

    let parallel_limit = req.parallel_limit.unwrap_or(4).min(8);
    tracing::info!(
        "Starting bulk upload of {} documents with parallelism {}",
        req.documents.len(),
        parallel_limit
    );

    // Process documents in parallel
    let results: Vec<BulkUploadResult> = stream::iter(req.documents.into_iter())
        .map(|doc| {
            let state = state.clone();
            async move { process_single_upload(state, doc).await }
        })
        .buffer_unordered(parallel_limit)
        .collect()
        .await;

    // Count successes and failures
    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    tracing::info!(
        "Bulk upload completed: {} successful, {} failed in {}ms",
        successful,
        failed,
        processing_time_ms
    );

    let response = BulkUploadResponse {
        total: results.len(),
        successful,
        failed,
        results,
        processing_time_ms,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Process a single document upload
async fn process_single_upload(
    state: Arc<AppState>,
    doc: BulkUploadDocument,
) -> BulkUploadResult {
    // Generate document ID
    let doc_id = Uuid::new_v4();

    // Decode base64 content
    let decoded_bytes = match base64::engine::general_purpose::STANDARD.decode(&doc.content) {
        Ok(bytes) => bytes,
        Err(e) => {
            return BulkUploadResult {
                title: doc.title,
                success: false,
                id: None,
                chunk_count: None,
                error: Some(format!("Invalid base64 content: {e}")),
            };
        }
    };

    // Validate file size (max 50MB)
    const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
    if decoded_bytes.len() > MAX_FILE_SIZE {
        return BulkUploadResult {
            title: doc.title,
            success: false,
            id: None,
            chunk_count: None,
            error: Some(format!(
                "File size exceeds 50MB (actual: {} bytes)",
                decoded_bytes.len()
            )),
        };
    }

    // Extract text content
    let text_content = match extract_text(&decoded_bytes, &doc.file_type) {
        Ok(text) => text,
        Err(e) => {
            return BulkUploadResult {
                title: doc.title,
                success: false,
                id: None,
                chunk_count: None,
                error: Some(format!("Text extraction failed: {e}")),
            };
        }
    };

    // Chunk the document
    let chunk_config = otl_parser::ChunkConfig {
        chunk_size: 1000,
        overlap: 200,
        min_chunk_size: 100,
        respect_sections: true,
        respect_paragraphs: true,
    };

    let chunks = chunk_text_simple(&text_content, &chunk_config);
    let _chunk_count = chunks.len() as u32;

    // Index chunks if vector backend available
    let mut processed_count = 0u32;
    let vector_backend_guard = state.vector_backend.read().await;
    if let Some(vector_backend) = vector_backend_guard.as_ref() {
        let backend = vector_backend.clone();
        drop(vector_backend_guard);

        const PARALLEL_LIMIT: usize = 4;
        let indexing_results: Vec<_> = stream::iter(chunks.into_iter().enumerate())
            .map(|(index, chunk_text)| {
                let backend = backend.clone();
                async move {
                    let result = backend.index_text(doc_id, index as u32, &chunk_text).await;
                    (index, result)
                }
            })
            .buffer_unordered(PARALLEL_LIMIT)
            .collect()
            .await;

        for (_index, result) in indexing_results {
            if result.is_ok() {
                processed_count += 1;
            }
        }
    }

    BulkUploadResult {
        title: doc.title,
        success: true,
        id: Some(doc_id),
        chunk_count: Some(processed_count),
        error: None,
    }
}

/// Extract text from file bytes
fn extract_text(bytes: &[u8], file_type: &str) -> Result<String, String> {
    match file_type.to_lowercase().as_str() {
        "pdf" => {
            if !bytes.starts_with(b"%PDF-") {
                return Err("Invalid PDF file: magic bytes do not match".to_string());
            }
            pdf_extract::extract_text_from_mem(bytes).map_err(|e| e.to_string())
        }
        "docx" => {
            if !bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
                return Err("Invalid DOCX file: magic bytes do not match".to_string());
            }
            extract_text_from_docx(bytes)
        }
        _ => {
            // Plain text
            String::from_utf8(bytes.to_vec())
                .map_err(|e| format!("Content is not valid UTF-8: {e}"))
        }
    }
}

/// Extract text from DOCX bytes
fn extract_text_from_docx(bytes: &[u8]) -> Result<String, String> {
    let docx = docx_rs::read_docx(bytes).map_err(|e| format!("Failed to parse DOCX: {e}"))?;

    let mut text = String::new();
    for child in docx.document.children {
        let Some(para) = extract_paragraph(child) else {
            continue;
        };

        for run_child in para.children {
            let Some(run) = extract_run(run_child) else {
                continue;
            };

            let run_text: String = run
                .children
                .into_iter()
                .filter_map(extract_text_node)
                .collect();

            text.push_str(&run_text);
        }
        text.push('\n');
    }

    if text.is_empty() {
        return Err("No text content found in DOCX".to_string());
    }

    Ok(text)
}

fn extract_paragraph(child: docx_rs::DocumentChild) -> Option<Box<docx_rs::Paragraph>> {
    if let docx_rs::DocumentChild::Paragraph(para) = child {
        Some(para)
    } else {
        None
    }
}

fn extract_run(child: docx_rs::ParagraphChild) -> Option<Box<docx_rs::Run>> {
    if let docx_rs::ParagraphChild::Run(run) = child {
        Some(run)
    } else {
        None
    }
}

fn extract_text_node(child: docx_rs::RunChild) -> Option<String> {
    if let docx_rs::RunChild::Text(t) = child {
        Some(t.text)
    } else {
        None
    }
}

/// Simple text chunking
fn chunk_text_simple(text: &str, config: &otl_parser::ChunkConfig) -> Vec<String> {
    let mut chunks = Vec::new();

    if text.len() <= config.chunk_size {
        chunks.push(text.to_string());
        return chunks;
    }

    let mut start = 0;
    while start < text.len() {
        let target_end = (start + config.chunk_size).min(text.len());
        let end = find_char_boundary(text, target_end);
        let actual_end = find_chunk_boundary(text, start, end, config.respect_paragraphs);

        let safe_start = find_char_boundary(text, start);
        let chunk_text = &text[safe_start..actual_end];

        if chunk_text.len() >= config.min_chunk_size {
            chunks.push(chunk_text.to_string());
        }

        if actual_end >= text.len() {
            break;
        }

        let overlap_pos = if actual_end > config.overlap {
            actual_end - config.overlap
        } else {
            actual_end
        };
        start = find_char_boundary(text, overlap_pos);
    }

    chunks
}

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

fn find_chunk_boundary(
    text: &str,
    _start: usize,
    target: usize,
    respect_paragraphs: bool,
) -> usize {
    if target >= text.len() {
        return text.len();
    }

    let target = find_char_boundary(text, target);

    if !respect_paragraphs {
        return target;
    }

    let search_start = find_char_boundary(text, target.saturating_sub(100));
    let search_end = find_char_boundary(text, (target + 100).min(text.len()));
    let search_text = &text[search_start..search_end];

    if let Some(pos) = search_text.rfind("\n\n") {
        return (search_start + pos + 2).min(text.len());
    }

    for pattern in [". ", "。", "! ", "? ", ".\n", "。\n", "!\n", "?\n"] {
        if let Some(pos) = search_text.rfind(pattern) {
            return (search_start + pos + pattern.len()).min(text.len());
        }
    }

    if let Some(pos) = search_text.rfind('\n') {
        return (search_start + pos + 1).min(text.len());
    }

    target
}

// ============================================================================
// Bulk Delete
// ============================================================================

/// Bulk delete request
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkDeleteRequest {
    /// List of document IDs to delete
    pub document_ids: Vec<Uuid>,
}

/// Result of a single document deletion
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkDeleteResult {
    /// Document ID
    pub id: Uuid,

    /// Success status
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,
}

/// Bulk delete response
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkDeleteResponse {
    /// Total documents in request
    pub total: usize,

    /// Successfully deleted
    pub successful: usize,

    /// Failed deletions
    pub failed: usize,

    /// Detailed results for each document
    pub results: Vec<BulkDeleteResult>,
}

/// Bulk delete documents
#[utoipa::path(
    delete,
    path = "/api/v1/documents/bulk",
    tag = "documents",
    request_body = BulkDeleteRequest,
    responses(
        (status = 200, description = "Bulk delete completed", body = BulkDeleteResponse),
        (status = 400, description = "Invalid request", body = crate::error::ApiError)
    )
)]
pub async fn bulk_delete_documents(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BulkDeleteRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    if req.document_ids.is_empty() {
        return Err(AppError::BadRequest(
            "No document IDs provided in bulk delete".to_string(),
        ));
    }

    if req.document_ids.len() > 100 {
        return Err(AppError::BadRequest(
            "Bulk delete limited to 100 documents per request".to_string(),
        ));
    }

    tracing::info!("Starting bulk delete of {} documents", req.document_ids.len());

    // Process deletions in parallel
    let results: Vec<BulkDeleteResult> = stream::iter(req.document_ids.into_iter())
        .map(|doc_id| {
            let state = state.clone();
            async move { process_single_delete(state, doc_id).await }
        })
        .buffer_unordered(8)
        .collect()
        .await;

    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;

    tracing::info!(
        "Bulk delete completed: {} successful, {} failed",
        successful,
        failed
    );

    let response = BulkDeleteResponse {
        total: results.len(),
        successful,
        failed,
        results,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Process a single document deletion
async fn process_single_delete(state: Arc<AppState>, doc_id: Uuid) -> BulkDeleteResult {
    // Get user context
    let user = state.get_default_user(None);

    // Check if document exists and user has permission
    #[derive(sqlx::FromRow)]
    struct DocCheck {
        #[allow(dead_code)]
        id: Uuid,
        access_level: String,
        owner_id: Option<String>,
        department: Option<String>,
    }

    let doc: Option<DocCheck> = match sqlx::query_as(
        "SELECT id, access_level::text, owner_id, department
         FROM documents
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(doc_id)
    .fetch_optional(&state.db_pool)
    .await
    {
        Ok(doc) => doc,
        Err(e) => {
            return BulkDeleteResult {
                id: doc_id,
                success: false,
                error: Some(format!("Database error: {e}")),
            };
        }
    };

    let Some(doc) = doc else {
        return BulkDeleteResult {
            id: doc_id,
            success: false,
            error: Some("Document not found".to_string()),
        };
    };

    // Check ACL permissions
    let acl = otl_core::DocumentAcl {
        access_level: parse_access_level(&doc.access_level),
        owner_id: doc.owner_id,
        department: doc.department,
        required_roles: Vec::new(),
        allowed_users: Vec::new(),
    };

    if !acl.can_access(&user) {
        return BulkDeleteResult {
            id: doc_id,
            success: false,
            error: Some("Permission denied".to_string()),
        };
    }

    // Delete from vector store if available
    let vector_backend_guard = state.vector_backend.read().await;
    if let Some(vector_backend) = vector_backend_guard.as_ref() {
        let backend = vector_backend.clone();
        drop(vector_backend_guard);

        if let Err(e) = backend.delete_by_document(doc_id).await {
            tracing::warn!("Failed to delete vectors for document {}: {}", doc_id, e);
        }
    }

    // Soft delete the document
    match sqlx::query("UPDATE documents SET deleted_at = NOW() WHERE id = $1")
        .bind(doc_id)
        .execute(&state.db_pool)
        .await
    {
        Ok(_) => BulkDeleteResult {
            id: doc_id,
            success: true,
            error: None,
        },
        Err(e) => BulkDeleteResult {
            id: doc_id,
            success: false,
            error: Some(format!("Delete failed: {e}")),
        },
    }
}

fn parse_access_level(level: &str) -> otl_core::AccessLevel {
    match level.to_lowercase().as_str() {
        "public" => otl_core::AccessLevel::Public,
        "internal" => otl_core::AccessLevel::Internal,
        "confidential" => otl_core::AccessLevel::Confidential,
        "restricted" => otl_core::AccessLevel::Restricted,
        _ => otl_core::AccessLevel::Internal,
    }
}
