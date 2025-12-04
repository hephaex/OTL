//! HITL verification handlers
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

/// Pending extraction for review
#[derive(Debug, Serialize, ToSchema)]
pub struct PendingExtraction {
    /// Extraction UUID
    pub id: Uuid,

    /// Source document ID
    pub document_id: Uuid,

    /// Source document title
    #[schema(example = "인사규정_2024.pdf")]
    pub document_title: String,

    /// Extraction type (entity or relation)
    #[schema(example = "entity")]
    pub extraction_type: String,

    /// Extracted content
    pub content: ExtractedContent,

    /// Confidence score
    #[schema(example = 0.65)]
    pub confidence: f32,

    /// Original text context
    #[schema(example = "연차휴가는 팀장의 사전 승인을 받아야 한다.")]
    pub context: String,

    /// Status
    #[schema(example = "pending")]
    pub status: String,

    /// Created timestamp
    pub created_at: String,
}

/// Extracted content details
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ExtractedContent {
    Entity {
        text: String,
        entity_type: String,
        start: usize,
        end: usize,
    },
    Relation {
        subject: String,
        predicate: String,
        object: String,
    },
}

/// Pending extractions list response
#[derive(Debug, Serialize)]
pub struct PendingListResponse {
    pub extractions: Vec<PendingExtraction>,
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
}

/// Query parameters for pending list
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListPendingQuery {
    /// Filter by extraction type
    pub extraction_type: Option<String>,

    /// Filter by document ID
    pub document_id: Option<Uuid>,

    /// Maximum confidence to include
    pub max_confidence: Option<f32>,

    /// Page number
    #[param(default = 1)]
    pub page: Option<u32>,

    /// Page size
    #[param(default = 20)]
    pub page_size: Option<u32>,
}

/// List pending extractions
#[utoipa::path(
    get,
    path = "/api/v1/verify/pending",
    tag = "verify",
    params(ListPendingQuery),
    responses(
        (status = 200, description = "Pending extractions list"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_pending(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListPendingQuery>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    // TODO: Implement actual pending list from database
    let extractions = vec![
        PendingExtraction {
            id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            document_title: "인사규정_2024.pdf".to_string(),
            extraction_type: "entity".to_string(),
            content: ExtractedContent::Entity {
                text: "연차휴가".to_string(),
                entity_type: "LeaveType".to_string(),
                start: 0,
                end: 4,
            },
            confidence: 0.65,
            context: "연차휴가는 팀장의 사전 승인을 받아야 한다.".to_string(),
            status: "pending".to_string(),
            created_at: "2024-12-03T10:00:00Z".to_string(),
        },
        PendingExtraction {
            id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            document_title: "인사규정_2024.pdf".to_string(),
            extraction_type: "relation".to_string(),
            content: ExtractedContent::Relation {
                subject: "연차휴가".to_string(),
                predicate: "requires".to_string(),
                object: "팀장승인".to_string(),
            },
            confidence: 0.72,
            context: "연차휴가는 팀장의 사전 승인을 받아야 한다.".to_string(),
            status: "pending".to_string(),
            created_at: "2024-12-03T10:00:00Z".to_string(),
        },
    ];

    let response = PendingListResponse {
        total: extractions.len(),
        extractions,
        page,
        page_size,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Verification action
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyAction {
    /// Optional correction for the extraction
    pub correction: Option<ExtractedContent>,

    /// Reviewer notes
    pub notes: Option<String>,
}

/// Verification response
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub id: Uuid,
    pub status: String,
    pub message: String,
}

/// Approve an extraction
#[utoipa::path(
    post,
    path = "/api/v1/verify/{id}/approve",
    tag = "verify",
    params(
        ("id" = Uuid, Path, description = "Extraction UUID")
    ),
    request_body = VerifyAction,
    responses(
        (status = 200, description = "Extraction approved"),
        (status = 404, description = "Extraction not found")
    )
)]
pub async fn approve_extraction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(action): Json<VerifyAction>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // TODO: Implement actual approval logic
    // 1. Update extraction status to 'approved'
    // 2. If correction provided, update the extraction content
    // 3. Queue for loading into knowledge graph

    tracing::info!("Approving extraction {} with notes: {:?}", id, action.notes);

    let response = VerifyResponse {
        id,
        status: "approved".to_string(),
        message: "Extraction approved and queued for graph loading".to_string(),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Reject action
#[derive(Debug, Deserialize)]
pub struct RejectAction {
    /// Reason for rejection
    pub reason: String,

    /// Reviewer notes
    pub notes: Option<String>,
}

/// Reject an extraction
#[utoipa::path(
    post,
    path = "/api/v1/verify/{id}/reject",
    tag = "verify",
    params(
        ("id" = Uuid, Path, description = "Extraction UUID")
    ),
    responses(
        (status = 200, description = "Extraction rejected"),
        (status = 404, description = "Extraction not found")
    )
)]
pub async fn reject_extraction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(action): Json<RejectAction>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // TODO: Implement actual rejection logic
    tracing::info!("Rejecting extraction {} with reason: {}", id, action.reason);

    let response = VerifyResponse {
        id,
        status: "rejected".to_string(),
        message: format!("Extraction rejected: {}", action.reason),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Verification statistics
#[derive(Debug, Serialize)]
pub struct VerifyStats {
    pub total_pending: u32,
    pub total_approved: u32,
    pub total_rejected: u32,
    pub entities: EntityStats,
    pub relations: RelationStats,
}

#[derive(Debug, Serialize)]
pub struct EntityStats {
    pub pending: u32,
    pub approved: u32,
    pub auto_approved: u32,
    pub rejected: u32,
    pub approval_rate: f32,
}

#[derive(Debug, Serialize)]
pub struct RelationStats {
    pub pending: u32,
    pub approved: u32,
    pub auto_approved: u32,
    pub rejected: u32,
    pub approval_rate: f32,
}

/// Get verification statistics
pub async fn get_stats(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // TODO: Implement actual stats from database
    let stats = VerifyStats {
        total_pending: 25,
        total_approved: 150,
        total_rejected: 12,
        entities: EntityStats {
            pending: 15,
            approved: 100,
            auto_approved: 80,
            rejected: 8,
            approval_rate: 92.5,
        },
        relations: RelationStats {
            pending: 10,
            approved: 50,
            auto_approved: 30,
            rejected: 4,
            approval_rate: 89.3,
        },
    };

    Ok((StatusCode::OK, Json(stats)))
}
