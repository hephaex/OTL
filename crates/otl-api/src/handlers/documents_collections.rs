//! Document collections/folders handlers
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
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Collection information
#[derive(Debug, Serialize, ToSchema)]
pub struct CollectionInfo {
    /// Collection ID
    pub id: Uuid,

    /// Collection name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Parent collection ID
    pub parent_id: Option<Uuid>,

    /// Full path
    pub path: Option<String>,

    /// Access level
    pub access_level: String,

    /// Owner ID
    pub owner_id: Option<Uuid>,

    /// Department
    pub department: Option<String>,

    /// Number of documents in this collection
    pub document_count: i64,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// List collections query parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListCollectionsQuery {
    /// Parent collection ID (null for root)
    pub parent_id: Option<Uuid>,

    /// Include deleted collections
    #[param(default = false)]
    pub include_deleted: Option<bool>,
}

/// List collections response
#[derive(Debug, Serialize, ToSchema)]
pub struct ListCollectionsResponse {
    /// List of collections
    pub collections: Vec<CollectionInfo>,

    /// Total count
    pub total: usize,
}

/// List collections
#[utoipa::path(
    get,
    path = "/api/v1/collections",
    tag = "collections",
    params(ListCollectionsQuery),
    responses(
        (status = 200, description = "Collections list", body = ListCollectionsResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ApiError)
    )
)]
pub async fn list_collections(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListCollectionsQuery>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let include_deleted = params.include_deleted.unwrap_or(false);

    let query = if include_deleted {
        "SELECT c.id, c.name, c.description, c.parent_id, c.path, c.access_level::text,
                c.owner_id, c.department, c.created_at, c.updated_at,
                COUNT(dcm.document_id) as document_count
         FROM document_collections c
         LEFT JOIN document_collection_members dcm ON c.id = dcm.collection_id
         WHERE ($1::UUID IS NULL OR c.parent_id = $1) OR ($1 IS NULL AND c.parent_id IS NULL)
         GROUP BY c.id
         ORDER BY c.name"
    } else {
        "SELECT c.id, c.name, c.description, c.parent_id, c.path, c.access_level::text,
                c.owner_id, c.department, c.created_at, c.updated_at,
                COUNT(dcm.document_id) as document_count
         FROM document_collections c
         LEFT JOIN document_collection_members dcm ON c.id = dcm.collection_id
         WHERE c.deleted_at IS NULL AND (($1::UUID IS NULL OR c.parent_id = $1) OR ($1 IS NULL AND c.parent_id IS NULL))
         GROUP BY c.id
         ORDER BY c.name"
    };

    #[derive(sqlx::FromRow)]
    struct CollectionRow {
        id: Uuid,
        name: String,
        description: Option<String>,
        parent_id: Option<Uuid>,
        path: Option<String>,
        access_level: String,
        owner_id: Option<Uuid>,
        department: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        document_count: i64,
    }

    let rows: Vec<CollectionRow> = sqlx::query_as(query)
        .bind(params.parent_id)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch collections: {e}")))?;

    let collections: Vec<CollectionInfo> = rows
        .into_iter()
        .map(|row| CollectionInfo {
            id: row.id,
            name: row.name,
            description: row.description,
            parent_id: row.parent_id,
            path: row.path,
            access_level: row.access_level,
            owner_id: row.owner_id,
            department: row.department,
            document_count: row.document_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    let response = ListCollectionsResponse {
        total: collections.len(),
        collections,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Create collection request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCollectionRequest {
    /// Collection name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Parent collection ID
    pub parent_id: Option<Uuid>,

    /// Access level
    pub access_level: Option<String>,

    /// Department
    pub department: Option<String>,
}

/// Create a new collection
#[utoipa::path(
    post,
    path = "/api/v1/collections",
    tag = "collections",
    request_body = CreateCollectionRequest,
    responses(
        (status = 201, description = "Collection created", body = CollectionInfo),
        (status = 400, description = "Invalid request", body = crate::error::ApiError)
    )
)]
pub async fn create_collection(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<CreateCollectionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement collection creation
    Err(AppError::Internal(
        "Collection creation not yet implemented".to_string(),
    ))
}

/// Add document to collection request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddToCollectionRequest {
    /// Document IDs to add
    pub document_ids: Vec<Uuid>,
}

/// Add documents to a collection
#[utoipa::path(
    post,
    path = "/api/v1/collections/{id}/documents",
    tag = "collections",
    params(
        ("id" = Uuid, Path, description = "Collection UUID")
    ),
    request_body = AddToCollectionRequest,
    responses(
        (status = 200, description = "Documents added to collection"),
        (status = 404, description = "Collection not found", body = crate::error::ApiError)
    )
)]
pub async fn add_to_collection(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<Uuid>,
    Json(_req): Json<AddToCollectionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Implement adding documents to collection
    Err(AppError::Internal(
        "Add to collection not yet implemented".to_string(),
    ))
}
