//! HITL verification handlers
//!
//! Author: hephaex@gmail.com

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use chrono::{DateTime, Utc};
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

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let offset = ((page - 1) * page_size) as i64;

    // Build query with filters
    let mut query = String::from(
        r#"
        SELECT
            eq.id,
            eq.document_id,
            d.title as document_title,
            eq.extracted_entities,
            eq.extracted_relations,
            eq.source_text,
            eq.confidence_score,
            eq.status::text,
            eq.created_at
        FROM extraction_queue eq
        JOIN documents d ON eq.document_id = d.id
        WHERE eq.status = 'pending'
        "#,
    );

    // Add filters
    if params.document_id.is_some() {
        query.push_str(" AND eq.document_id = $1");
    }
    if params.max_confidence.is_some() {
        let param_idx = if params.document_id.is_some() { 2 } else { 1 };
        query.push_str(&format!(" AND eq.confidence_score <= ${param_idx}"));
    }

    query.push_str(" ORDER BY eq.priority, eq.created_at LIMIT $");
    let limit_idx = if params.document_id.is_some() {
        if params.max_confidence.is_some() {
            3
        } else {
            2
        }
    } else if params.max_confidence.is_some() {
        2
    } else {
        1
    };
    query.push_str(&format!("{limit_idx} OFFSET ${}", limit_idx + 1));

    // Execute query based on filters
    #[derive(sqlx::FromRow)]
    struct ExtractionRow {
        id: Uuid,
        document_id: Uuid,
        document_title: String,
        extracted_entities: serde_json::Value,
        extracted_relations: serde_json::Value,
        source_text: Option<String>,
        confidence_score: f32,
        status: String,
        created_at: DateTime<Utc>,
    }

    let rows: Vec<ExtractionRow> = match (params.document_id, params.max_confidence) {
        (Some(doc_id), Some(max_conf)) => sqlx::query_as(&query)
            .bind(doc_id)
            .bind(max_conf)
            .bind(page_size as i64)
            .bind(offset)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Database query failed: {}", e);
                AppError::Internal(format!("Failed to fetch pending extractions: {e}"))
            })?,
        (Some(doc_id), None) => sqlx::query_as(&query)
            .bind(doc_id)
            .bind(page_size as i64)
            .bind(offset)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Database query failed: {}", e);
                AppError::Internal(format!("Failed to fetch pending extractions: {e}"))
            })?,
        (None, Some(max_conf)) => sqlx::query_as(&query)
            .bind(max_conf)
            .bind(page_size as i64)
            .bind(offset)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Database query failed: {}", e);
                AppError::Internal(format!("Failed to fetch pending extractions: {e}"))
            })?,
        (None, None) => sqlx::query_as(&query)
            .bind(page_size as i64)
            .bind(offset)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Database query failed: {}", e);
                AppError::Internal(format!("Failed to fetch pending extractions: {e}"))
            })?,
    };

    // Get total count for pagination
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM extraction_queue
        WHERE status = 'pending'
        "#,
    )
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    // Convert rows to response format
    let extractions = rows
        .into_iter()
        .flat_map(|row| {
            let context = row.source_text.unwrap_or_default();
            let created_at = row.created_at.to_rfc3339();

            let mut results = Vec::new();

            // Process entities
            if let Some(entities) = row.extracted_entities.as_array() {
                for entity in entities {
                    // Skip non-object entities
                    if entity.as_object().is_none() {
                        continue;
                    }

                    // Try to parse entity content
                    let Ok(content) = serde_json::from_value::<ExtractedContent>(entity.clone())
                    else {
                        continue;
                    };

                    results.push(PendingExtraction {
                        id: row.id,
                        document_id: row.document_id,
                        document_title: row.document_title.clone(),
                        extraction_type: "entity".to_string(),
                        content,
                        confidence: row.confidence_score,
                        context: context.clone(),
                        status: row.status.clone(),
                        created_at: created_at.clone(),
                    });
                }
            }

            // Process relations
            if let Some(relations) = row.extracted_relations.as_array() {
                for relation in relations {
                    // Skip non-object relations
                    if relation.as_object().is_none() {
                        continue;
                    }

                    // Try to parse relation content
                    let Ok(content) = serde_json::from_value::<ExtractedContent>(relation.clone())
                    else {
                        continue;
                    };

                    results.push(PendingExtraction {
                        id: row.id,
                        document_id: row.document_id,
                        document_title: row.document_title.clone(),
                        extraction_type: "relation".to_string(),
                        content,
                        confidence: row.confidence_score,
                        context: context.clone(),
                        status: row.status.clone(),
                        created_at: created_at.clone(),
                    });
                }
            }

            results
        })
        .collect();

    let response = PendingListResponse {
        total: total as usize,
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
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(action): Json<VerifyAction>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Start database transaction
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to start transaction: {e}")))?;

    // Verify extraction exists and is pending
    let extraction: Option<(String, serde_json::Value, serde_json::Value)> = sqlx::query_as(
        r#"
        SELECT status::text, extracted_entities, extracted_relations
        FROM extraction_queue
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch extraction: {e}")))?;

    let (current_status, mut entities, mut relations) =
        extraction.ok_or_else(|| AppError::NotFound(format!("Extraction {id} not found")))?;

    if current_status != "pending" {
        return Err(AppError::BadRequest(format!(
            "Cannot approve extraction in status: {current_status}"
        )));
    }

    // If correction provided, update the extraction content
    if let Some(correction) = action.correction {
        match correction {
            ExtractedContent::Entity { .. } => {
                // Replace entities with corrected version
                entities = serde_json::to_value(vec![correction]).unwrap_or(serde_json::json!([]));
            }
            ExtractedContent::Relation { .. } => {
                // Replace relations with corrected version
                relations = serde_json::to_value(vec![correction]).unwrap_or(serde_json::json!([]));
            }
        }
    }

    // Update extraction status to approved
    let now = Utc::now();
    let notes_for_log = action.notes.clone();
    let notes_value = action.notes.clone(); // Clone before bind
    let result = sqlx::query(
        r#"
        UPDATE extraction_queue
        SET status = 'approved',
            reviewer_id = $1,
            review_notes = $2,
            reviewed_at = $3,
            extracted_entities = $4,
            extracted_relations = $5
        WHERE id = $6
        "#,
    )
    .bind(user.user_id.to_string())
    .bind(notes_value)
    .bind(now)
    .bind(entities)
    .bind(relations)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update extraction: {e}")))?;

    if result.rows_affected() == 0 {
        tx.rollback().await.ok();
        return Err(AppError::NotFound(format!("Extraction {id} not found")));
    }

    // Commit transaction
    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to commit transaction: {e}")))?;

    tracing::info!("Approved extraction {} with notes: {:?}", id, notes_for_log);

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
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(action): Json<RejectAction>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Start database transaction
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to start transaction: {e}")))?;

    // Verify extraction exists and is pending
    let current_status: Option<String> = sqlx::query_scalar(
        r#"
        SELECT status::text
        FROM extraction_queue
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch extraction: {e}")))?;

    let status =
        current_status.ok_or_else(|| AppError::NotFound(format!("Extraction {id} not found")))?;

    if status != "pending" {
        return Err(AppError::BadRequest(format!(
            "Cannot reject extraction in status: {status}"
        )));
    }

    // Update extraction status to rejected
    let now = Utc::now();
    let review_notes = format!(
        "REJECTED: {}\n{}",
        action.reason,
        action.notes.unwrap_or_default()
    );

    let result = sqlx::query(
        r#"
        UPDATE extraction_queue
        SET status = 'rejected',
            reviewer_id = $1,
            review_notes = $2,
            reviewed_at = $3
        WHERE id = $4
        "#,
    )
    .bind(user.user_id.to_string())
    .bind(review_notes)
    .bind(now)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update extraction: {e}")))?;

    if result.rows_affected() == 0 {
        tx.rollback().await.ok();
        return Err(AppError::NotFound(format!("Extraction {id} not found")));
    }

    // Commit transaction
    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to commit transaction: {e}")))?;

    tracing::info!("Rejected extraction {} with reason: {}", id, action.reason);

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

    // Query overall statistics
    #[derive(sqlx::FromRow)]
    struct StatusCount {
        status: String,
        count: i64,
    }

    let status_counts: Vec<StatusCount> = sqlx::query_as(
        r#"
        SELECT status::text, COUNT(*) as count
        FROM extraction_queue
        GROUP BY status
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch status counts: {}", e);
        AppError::Internal(format!("Failed to fetch statistics: {e}"))
    })?;

    let mut total_pending = 0;
    let mut total_approved = 0;
    let mut total_rejected = 0;

    for stat in status_counts {
        match stat.status.as_str() {
            "pending" | "in_review" => total_pending += stat.count as u32,
            "approved" => total_approved += stat.count as u32,
            "rejected" => total_rejected += stat.count as u32,
            _ => {}
        }
    }

    // Query entity statistics
    #[derive(sqlx::FromRow)]
    struct EntityCount {
        status: String,
        entity_count: i64,
    }

    let entity_stats: Vec<EntityCount> = sqlx::query_as(
        r#"
        SELECT
            status::text,
            SUM(jsonb_array_length(extracted_entities)) as entity_count
        FROM extraction_queue
        WHERE jsonb_array_length(extracted_entities) > 0
        GROUP BY status
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    let mut entity_pending = 0;
    let mut entity_approved = 0;
    let mut entity_rejected = 0;

    for stat in entity_stats {
        match stat.status.as_str() {
            "pending" | "in_review" => entity_pending += stat.entity_count as u32,
            "approved" => entity_approved += stat.entity_count as u32,
            "rejected" => entity_rejected += stat.entity_count as u32,
            _ => {}
        }
    }

    // Query relation statistics
    #[derive(sqlx::FromRow)]
    struct RelationCount {
        status: String,
        relation_count: i64,
    }

    let relation_stats: Vec<RelationCount> = sqlx::query_as(
        r#"
        SELECT
            status::text,
            SUM(jsonb_array_length(extracted_relations)) as relation_count
        FROM extraction_queue
        WHERE jsonb_array_length(extracted_relations) > 0
        GROUP BY status
        "#,
    )
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    let mut relation_pending = 0;
    let mut relation_approved = 0;
    let mut relation_rejected = 0;

    for stat in relation_stats {
        match stat.status.as_str() {
            "pending" | "in_review" => relation_pending += stat.relation_count as u32,
            "approved" => relation_approved += stat.relation_count as u32,
            "rejected" => relation_rejected += stat.relation_count as u32,
            _ => {}
        }
    }

    // Calculate approval rates
    let entity_total = entity_approved + entity_rejected;
    let entity_approval_rate = if entity_total > 0 {
        (entity_approved as f32 / entity_total as f32) * 100.0
    } else {
        0.0
    };

    let relation_total = relation_approved + relation_rejected;
    let relation_approval_rate = if relation_total > 0 {
        (relation_approved as f32 / relation_total as f32) * 100.0
    } else {
        0.0
    };

    // Auto-approved count (confidence >= 0.9)
    let entity_auto_approved: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM extraction_queue
        WHERE status = 'approved'
            AND confidence_score >= 0.9
            AND jsonb_array_length(extracted_entities) > 0
        "#,
    )
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    let relation_auto_approved: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM extraction_queue
        WHERE status = 'approved'
            AND confidence_score >= 0.9
            AND jsonb_array_length(extracted_relations) > 0
        "#,
    )
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    let stats = VerifyStats {
        total_pending,
        total_approved,
        total_rejected,
        entities: EntityStats {
            pending: entity_pending,
            approved: entity_approved,
            auto_approved: entity_auto_approved as u32,
            rejected: entity_rejected,
            approval_rate: entity_approval_rate,
        },
        relations: RelationStats {
            pending: relation_pending,
            approved: relation_approved,
            auto_approved: relation_auto_approved as u32,
            rejected: relation_rejected,
            approval_rate: relation_approval_rate,
        },
    };

    Ok((StatusCode::OK, Json(stats)))
}
