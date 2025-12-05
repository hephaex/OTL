//! Knowledge graph handlers
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

/// Entity information
#[derive(Debug, Serialize, ToSchema)]
pub struct EntityInfo {
    /// Entity UUID
    pub id: Uuid,

    /// Entity type (e.g., "Employee", "Department", "LeaveType")
    #[schema(example = "LeaveType")]
    pub entity_type: String,

    /// Entity name/text
    #[schema(example = "연차휴가")]
    pub name: String,

    /// Additional properties
    pub properties: serde_json::Value,

    /// Related entities count
    pub relation_count: u32,
}

/// Relation information
#[derive(Debug, Serialize, ToSchema)]
pub struct RelationInfo {
    /// Relation UUID
    pub id: Uuid,

    /// Relation type
    #[schema(example = "requires")]
    pub relation_type: String,

    /// Source entity ID
    pub source_id: Uuid,

    /// Source entity name
    #[schema(example = "연차휴가")]
    pub source_name: String,

    /// Target entity ID
    pub target_id: Uuid,

    /// Target entity name
    #[schema(example = "팀장승인")]
    pub target_name: String,

    /// Confidence score
    pub confidence: f32,
}

/// Entity list response
#[derive(Debug, Serialize)]
pub struct EntityListResponse {
    pub entities: Vec<EntityInfo>,
    pub total: usize,
}

/// Entity detail response with relations
#[derive(Debug, Serialize)]
pub struct EntityDetailResponse {
    pub entity: EntityInfo,
    pub incoming_relations: Vec<RelationInfo>,
    pub outgoing_relations: Vec<RelationInfo>,
}

/// Query parameters for entity listing
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListEntitiesQuery {
    /// Filter by entity type
    pub entity_type: Option<String>,

    /// Search in name
    pub search: Option<String>,

    /// Limit results
    #[param(default = 100)]
    pub limit: Option<usize>,
}

/// List entities
#[utoipa::path(
    get,
    path = "/api/v1/graph/entities",
    tag = "graph",
    params(ListEntitiesQuery),
    responses(
        (status = 200, description = "Entity list"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_entities(
    State(state): State<Arc<AppState>>,
    Query(_params): Query<ListEntitiesQuery>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // TODO: Implement actual entity listing from SurrealDB
    let entities = vec![
        EntityInfo {
            id: Uuid::new_v4(),
            entity_type: "LeaveType".to_string(),
            name: "연차휴가".to_string(),
            properties: serde_json::json!({"maxDays": 15}),
            relation_count: 3,
        },
        EntityInfo {
            id: Uuid::new_v4(),
            entity_type: "LeaveType".to_string(),
            name: "병가".to_string(),
            properties: serde_json::json!({"requiresDocument": true}),
            relation_count: 2,
        },
        EntityInfo {
            id: Uuid::new_v4(),
            entity_type: "ApprovalProcess".to_string(),
            name: "팀장승인".to_string(),
            properties: serde_json::json!({"level": 1}),
            relation_count: 5,
        },
    ];

    let response = EntityListResponse {
        total: entities.len(),
        entities,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Get entity with relations
#[utoipa::path(
    get,
    path = "/api/v1/graph/entities/{id}",
    tag = "graph",
    params(
        ("id" = Uuid, Path, description = "Entity UUID")
    ),
    responses(
        (status = 200, description = "Entity details with relations"),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // TODO: Implement actual entity retrieval with relations
    let entity = EntityInfo {
        id,
        entity_type: "LeaveType".to_string(),
        name: "연차휴가".to_string(),
        properties: serde_json::json!({"maxDays": 15, "description": "연간 15일 기본 부여"}),
        relation_count: 3,
    };

    let response = EntityDetailResponse {
        entity,
        incoming_relations: vec![],
        outgoing_relations: vec![RelationInfo {
            id: Uuid::new_v4(),
            relation_type: "requires".to_string(),
            source_id: id,
            source_name: "연차휴가".to_string(),
            target_id: Uuid::new_v4(),
            target_name: "팀장승인".to_string(),
            confidence: 0.95,
        }],
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Graph search request
#[derive(Debug, Deserialize, ToSchema)]
pub struct GraphSearchRequest {
    /// Search query
    #[schema(example = "휴가 승인 절차")]
    pub query: String,

    /// Maximum depth for graph traversal
    #[serde(default = "default_depth")]
    #[schema(default = 2)]
    pub depth: u32,

    /// Maximum results
    #[serde(default = "default_limit")]
    #[schema(default = 20)]
    pub limit: usize,
}

fn default_depth() -> u32 {
    2
}

fn default_limit() -> usize {
    20
}

/// Graph search response
#[derive(Debug, Serialize, ToSchema)]
pub struct GraphSearchResponse {
    /// Matching entities
    pub entities: Vec<EntityInfo>,

    /// Relations between found entities
    pub relations: Vec<RelationInfo>,

    /// Search metadata
    pub metadata: SearchMetadata,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchMetadata {
    pub query: String,
    pub depth: u32,
    pub total_entities: usize,
    pub total_relations: usize,
    pub processing_time_ms: u64,
}

/// Search the knowledge graph
#[utoipa::path(
    post,
    path = "/api/v1/graph/search",
    tag = "graph",
    request_body = GraphSearchRequest,
    responses(
        (status = 200, description = "Search results", body = GraphSearchResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn search_graph(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GraphSearchRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let start = std::time::Instant::now();

    if req.query.trim().is_empty() {
        return Err(AppError::BadRequest("Query cannot be empty".to_string()));
    }

    // TODO: Implement actual graph search
    let entities = vec![
        EntityInfo {
            id: Uuid::new_v4(),
            entity_type: "LeaveType".to_string(),
            name: "연차휴가".to_string(),
            properties: serde_json::json!({}),
            relation_count: 3,
        },
        EntityInfo {
            id: Uuid::new_v4(),
            entity_type: "ApprovalProcess".to_string(),
            name: "팀장승인".to_string(),
            properties: serde_json::json!({}),
            relation_count: 5,
        },
    ];

    let relations = vec![RelationInfo {
        id: Uuid::new_v4(),
        relation_type: "requires".to_string(),
        source_id: entities[0].id,
        source_name: "연차휴가".to_string(),
        target_id: entities[1].id,
        target_name: "팀장승인".to_string(),
        confidence: 0.95,
    }];

    let response = GraphSearchResponse {
        metadata: SearchMetadata {
            query: req.query,
            depth: req.depth,
            total_entities: entities.len(),
            total_relations: relations.len(),
            processing_time_ms: start.elapsed().as_millis() as u64,
        },
        entities,
        relations,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Ontology schema response
#[derive(Debug, Serialize)]
pub struct OntologyResponse {
    pub classes: Vec<OntologyClass>,
    pub properties: Vec<OntologyProperty>,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OntologyClass {
    pub name: String,
    pub label: String,
    pub parent: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OntologyProperty {
    pub name: String,
    pub label: String,
    pub domain: String,
    pub range: String,
}

/// Get ontology schema
pub async fn get_ontology(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let response = OntologyResponse {
        classes: vec![
            OntologyClass {
                name: "Employee".to_string(),
                label: "직원".to_string(),
                parent: None,
            },
            OntologyClass {
                name: "Department".to_string(),
                label: "부서".to_string(),
                parent: None,
            },
            OntologyClass {
                name: "LeaveType".to_string(),
                label: "휴가유형".to_string(),
                parent: None,
            },
        ],
        properties: vec![
            OntologyProperty {
                name: "belongsTo".to_string(),
                label: "소속".to_string(),
                domain: "Employee".to_string(),
                range: "Department".to_string(),
            },
            OntologyProperty {
                name: "requires".to_string(),
                label: "필요".to_string(),
                domain: "LeaveType".to_string(),
                range: "ApprovalProcess".to_string(),
            },
        ],
        version: "1.0.0".to_string(),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Update ontology request
#[derive(Debug, Deserialize)]
pub struct UpdateOntologyRequest {
    pub classes: Option<Vec<OntologyClass>>,
    pub properties: Option<Vec<OntologyProperty>>,
}

/// Update ontology (admin only)
pub async fn update_ontology(
    State(state): State<Arc<AppState>>,
    Json(_req): Json<UpdateOntologyRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // TODO: Implement ontology update with admin check
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"message": "Ontology updated"})),
    ))
}
