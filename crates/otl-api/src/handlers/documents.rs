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
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Database row for document queries
#[derive(sqlx::FromRow)]
struct DocumentRow {
    id: Uuid,
    title: String,
    file_type: String,
    access_level: String,
    department: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    chunk_count: i64,
}

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

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100); // Cap at 100
    let offset = ((page - 1) * page_size) as i64;

    // Get user context (for now, use default user)
    let user = state.get_default_user(None);

    // Build base query with ACL filtering
    let mut query = String::from(
        "SELECT d.id, d.title, d.file_type::text, d.access_level::text, d.department,
                d.created_at, d.updated_at, COUNT(dc.id) as chunk_count
         FROM documents d
         LEFT JOIN document_chunks dc ON d.id = dc.document_id
         WHERE d.deleted_at IS NULL"
    );

    let mut conditions = Vec::new();
    let mut param_count = 1;

    // ACL filtering based on user permissions
    if !user.is_internal {
        // Anonymous users can only see public documents
        conditions.push("d.access_level = 'public'".to_string());
    } else {
        // Internal users: apply ACL logic
        // Can see: public, internal, confidential (if dept/role match), restricted (if allowed)
        let acl_filter = format!(
            "(d.access_level = 'public' OR d.access_level = 'internal' \
             OR (d.access_level = 'confidential' AND (d.department = ${} OR d.required_roles && ${{{}}})) \
             OR (d.access_level = 'restricted' AND (d.owner_id = ${} OR ${} = ANY(d.allowed_users))))",
            param_count,
            param_count + 1,
            param_count + 2,
            param_count + 2
        );
        conditions.push(acl_filter);
        param_count += 3;
    }

    // Apply additional filters
    if let Some(ref _file_type) = params.file_type {
        conditions.push(format!("d.file_type::text = ${param_count}"));
        param_count += 1;
    }

    if let Some(ref _department) = params.department {
        conditions.push(format!("d.department = ${param_count}"));
        param_count += 1;
    }

    if let Some(ref _search) = params.search {
        conditions.push(format!("d.title ILIKE ${param_count}"));
        param_count += 1;
    }

    if !conditions.is_empty() {
        query.push_str(" AND ");
        query.push_str(&conditions.join(" AND "));
    }

    query.push_str(" GROUP BY d.id ORDER BY d.created_at DESC LIMIT $");
    query.push_str(&(param_count).to_string());
    param_count += 1;
    query.push_str(" OFFSET $");
    query.push_str(&(param_count).to_string());

    // Execute query with parameters
    let mut query_builder = sqlx::query_as::<_, DocumentRow>(&query);

    // Bind ACL parameters
    if user.is_internal {
        let dept = user.departments.first().cloned().unwrap_or_default();
        query_builder = query_builder
            .bind(dept.clone())
            .bind(&user.roles)
            .bind(&user.user_id);
    }

    // Bind filter parameters
    if let Some(ref file_type) = params.file_type {
        query_builder = query_builder.bind(file_type);
    }
    if let Some(ref department) = params.department {
        query_builder = query_builder.bind(department);
    }
    if let Some(ref search) = params.search {
        query_builder = query_builder.bind(format!("%{search}%"));
    }

    // Bind pagination
    query_builder = query_builder.bind(page_size as i64).bind(offset);

    let rows = query_builder
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch documents: {e}")))?;

    // Get total count with same filters
    let count_query = format!(
        "SELECT COUNT(DISTINCT d.id) as count FROM documents d WHERE d.deleted_at IS NULL{}",
        if conditions.is_empty() {
            String::new()
        } else {
            format!(" AND {}", conditions.join(" AND "))
        }
    );

    let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query);

    // Bind same parameters for count
    if user.is_internal {
        let dept = user.departments.first().cloned().unwrap_or_default();
        count_builder = count_builder
            .bind(dept.clone())
            .bind(&user.roles)
            .bind(&user.user_id);
    }
    if let Some(ref file_type) = params.file_type {
        count_builder = count_builder.bind(file_type);
    }
    if let Some(ref department) = params.department {
        count_builder = count_builder.bind(department);
    }
    if let Some(ref search) = params.search {
        count_builder = count_builder.bind(format!("%{search}%"));
    }

    let total = count_builder
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to count documents: {e}")))?;

    let documents: Vec<DocumentInfo> = rows
        .into_iter()
        .map(|row| DocumentInfo {
            id: row.id,
            title: row.title,
            file_type: row.file_type,
            access_level: row.access_level,
            department: row.department,
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
            chunk_count: row.chunk_count as u32,
        })
        .collect();

    let response = DocumentListResponse {
        total: total as usize,
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

    // Get user context
    let user = state.get_default_user(None);

    // Query document with chunk count
    let row = sqlx::query_as::<_, DocumentRow>(
        "SELECT d.id, d.title, d.file_type::text, d.access_level::text, d.department,
                d.created_at, d.updated_at, COUNT(dc.id) as chunk_count
         FROM documents d
         LEFT JOIN document_chunks dc ON d.id = dc.document_id
         WHERE d.id = $1 AND d.deleted_at IS NULL
         GROUP BY d.id"
    )
    .bind(id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch document: {e}")))?;

    let row = row.ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;

    // Check ACL permissions
    let acl = otl_core::DocumentAcl {
        access_level: parse_access_level(&row.access_level),
        owner_id: None, // Would need to fetch from DB if needed
        department: row.department.clone(),
        required_roles: Vec::new(), // Would need to fetch from DB if needed
        allowed_users: Vec::new(), // Would need to fetch from DB if needed
    };

    if !acl.can_access(&user) {
        return Err(AppError::Forbidden(
            "You don't have permission to access this document".to_string(),
        ));
    }

    let doc = DocumentInfo {
        id: row.id,
        title: row.title,
        file_type: row.file_type,
        access_level: row.access_level,
        department: row.department,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
        chunk_count: row.chunk_count as u32,
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

    // Validate file size (max 50MB)
    const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
    if decoded_bytes.len() > MAX_FILE_SIZE {
        return Err(AppError::BadRequest(format!(
            "File size exceeds maximum allowed size of 50MB (actual: {} bytes)",
            decoded_bytes.len()
        )));
    }

    // Validate magic bytes for file type
    match req.file_type.to_lowercase().as_str() {
        "pdf" => {
            if !decoded_bytes.starts_with(b"%PDF-") {
                return Err(AppError::BadRequest(
                    "Invalid PDF file: magic bytes do not match".to_string(),
                ));
            }
        }
        "docx" => {
            // DOCX files are ZIP archives starting with PK signature
            if !decoded_bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
                return Err(AppError::BadRequest(
                    "Invalid DOCX file: magic bytes do not match (expected ZIP signature)"
                        .to_string(),
                ));
            }
        }
        _ => {
            // For text files, no magic bytes validation needed
        }
    }

    // Extract text content based on file type
    let text_content = match req.file_type.to_lowercase().as_str() {
        "pdf" => {
            // Use PDF parser to extract text
            extract_text_from_pdf(&decoded_bytes).map_err(|e| {
                AppError::BadRequest(format!("Failed to extract text from PDF: {e}"))
            })?
        }
        "docx" => {
            // Use DOCX parser to extract text
            extract_text_from_docx(&decoded_bytes).map_err(|e| {
                AppError::BadRequest(format!("Failed to extract text from DOCX: {e}"))
            })?
        }
        _ => {
            // Assume plain text (txt, md, etc.)
            String::from_utf8(decoded_bytes)
                .map_err(|e| AppError::BadRequest(format!("Content is not valid UTF-8: {e}")))?
        }
    };

    tracing::info!(
        "Processing document upload: {} (id: {}, type: {}, size: {} bytes)",
        req.title,
        doc_id,
        req.file_type,
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

    tracing::info!("Document {} split into {} chunks", doc_id, chunk_count);

    // Get vector backend and process chunks
    let vector_backend_guard = state.vector_backend.read().await;
    if let Some(vector_backend) = vector_backend_guard.as_ref() {
        // Clone the Arc to avoid holding the lock during async operations
        let backend = vector_backend.clone();
        drop(vector_backend_guard); // Release lock before async operations

        // Process chunks in parallel using buffer_unordered for better performance
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

        // Process results and count successes
        let mut processed_count = 0;
        for (index, result) in indexing_results {
            match result {
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
fn find_chunk_boundary(
    text: &str,
    _start: usize,
    target: usize,
    respect_paragraphs: bool,
) -> usize {
    if target >= text.len() {
        return text.len();
    }

    // Ensure target is on a valid char boundary
    let target = find_char_boundary(text, target);

    if !respect_paragraphs {
        return target;
    }

    // Search window around target position (ensure boundaries are valid)
    let search_start = find_char_boundary(text, target.saturating_sub(100));
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

    // Get user context
    let user = state.get_default_user(None);

    // First, check if document exists and user has permission
    #[derive(sqlx::FromRow)]
    struct DocCheck {
        #[allow(dead_code)]
        id: Uuid,
        access_level: String,
        owner_id: Option<String>,
        department: Option<String>,
    }

    let doc: Option<DocCheck> = sqlx::query_as(
        "SELECT id, access_level::text, owner_id, department
         FROM documents
         WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch document: {e}")))?;

    let doc = doc.ok_or_else(|| AppError::NotFound(format!("Document {id} not found")))?;

    // Check ACL permissions
    let acl = otl_core::DocumentAcl {
        access_level: parse_access_level(&doc.access_level),
        owner_id: doc.owner_id.clone(),
        department: doc.department.clone(),
        required_roles: Vec::new(),
        allowed_users: Vec::new(),
    };

    if !acl.can_access(&user) {
        return Err(AppError::Forbidden(
            "You don't have permission to delete this document".to_string(),
        ));
    }

    tracing::info!("Deleting document: {id}");

    // Delete from vector store if available (use document-level deletion)
    let vector_backend_guard = state.vector_backend.read().await;
    if let Some(vector_backend) = vector_backend_guard.as_ref() {
        let backend = vector_backend.clone();
        drop(vector_backend_guard);

        match backend.delete_by_document(id).await {
            Ok(count) => {
                tracing::info!("Deleted {count} vectors from vector store for document {id}");
            }
            Err(e) => {
                tracing::warn!("Failed to delete vectors for document {id}: {e}");
            }
        }
    }

    // Soft delete the document (cascade will handle chunks via ON DELETE CASCADE)
    let result = sqlx::query(
        "UPDATE documents SET deleted_at = NOW() WHERE id = $1"
    )
    .bind(id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to delete document: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Document {id} not found")));
    }

    tracing::info!("Document {id} soft deleted successfully");

    Ok((
        StatusCode::OK,
        Json(DeleteDocumentResponse {
            message: format!("Document {id} deleted successfully"),
        }),
    ))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse access level string to enum
fn parse_access_level(level: &str) -> otl_core::AccessLevel {
    match level.to_lowercase().as_str() {
        "public" => otl_core::AccessLevel::Public,
        "internal" => otl_core::AccessLevel::Internal,
        "confidential" => otl_core::AccessLevel::Confidential,
        "restricted" => otl_core::AccessLevel::Restricted,
        _ => otl_core::AccessLevel::Internal, // Default to internal
    }
}

// ============================================================================
// Document Format Extractors
// ============================================================================

/// Extract text from PDF bytes using pdf-extract library
fn extract_text_from_pdf(bytes: &[u8]) -> Result<String, String> {
    pdf_extract::extract_text_from_mem(bytes).map_err(|e| e.to_string())
}

/// Extract text from DOCX bytes
fn extract_text_from_docx(bytes: &[u8]) -> Result<String, String> {
    // Parse the DOCX file directly from bytes
    let docx = docx_rs::read_docx(bytes).map_err(|e| format!("Failed to parse DOCX: {e}"))?;

    // Extract text from all paragraphs
    let mut text = String::new();
    for child in docx.document.children {
        if let docx_rs::DocumentChild::Paragraph(para) = child {
            for child in para.children {
                if let docx_rs::ParagraphChild::Run(run) = child {
                    for child in run.children {
                        if let docx_rs::RunChild::Text(t) = child {
                            text.push_str(&t.text);
                        }
                    }
                }
            }
            text.push('\n');
        }
    }

    if text.is_empty() {
        return Err("No text content found in DOCX".to_string());
    }

    Ok(text)
}
