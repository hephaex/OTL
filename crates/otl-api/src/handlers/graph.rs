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
use otl_graph::GraphStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    Query(params): Query<ListEntitiesQuery>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Get graph database connection
    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    // Determine query parameters
    let limit = params.limit.unwrap_or(100).min(1000); // Cap at 1000

    // Query entities based on filters
    let entities_result = if let Some(entity_type) = params.entity_type.as_ref() {
        // Filter by entity type
        graph_db.find_by_class(entity_type, limit).await
    } else if let Some(search_term) = params.search.as_ref() {
        // Search in entity text/name
        graph_db.query(&format!(
            "SELECT * FROM entity WHERE properties.text CONTAINS '{}' LIMIT {}",
            search_term.replace('\'', "\\'"),
            limit
        )).await
    } else {
        // Get all entities with limit
        graph_db.query(&format!("SELECT * FROM entity LIMIT {}", limit)).await
    };

    let entities = entities_result
        .map_err(|e| AppError::Internal(format!("Failed to query entities: {}", e)))?;

    // Convert to EntityInfo and count relations
    let mut entity_infos = Vec::new();
    for entity in entities {
        // Get relation count for this entity
        let relation_count = count_entity_relations(&**graph_db, entity.id).await.unwrap_or(0);

        // Extract name from properties
        let name = extract_entity_name(&entity.properties);

        entity_infos.push(EntityInfo {
            id: entity.id,
            entity_type: entity.class.clone(),
            name,
            properties: serde_json::to_value(&entity.properties).unwrap_or_default(),
            relation_count,
        });
    }

    let response = EntityListResponse {
        total: entity_infos.len(),
        entities: entity_infos,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Extract entity name from properties
fn extract_entity_name(properties: &HashMap<String, serde_json::Value>) -> String {
    properties
        .get("text")
        .and_then(|v| v.as_str())
        .or_else(|| properties.get("name").and_then(|v| v.as_str()))
        .or_else(|| properties.get("label").and_then(|v| v.as_str()))
        .unwrap_or("Unnamed")
        .to_string()
}

/// Count relations for an entity
async fn count_entity_relations(
    graph_db: &dyn GraphStore,
    entity_id: Uuid,
) -> Result<u32, otl_core::OtlError> {
    // Since we don't have a count method, we'll use traverse as approximation
    let related = graph_db.traverse(entity_id, 1).await?;
    Ok(related.len() as u32)
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

    // Get graph database connection
    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    // Get the entity
    let entity_opt = graph_db
        .get_entity(id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get entity: {}", e)))?;

    let entity = entity_opt.ok_or_else(|| AppError::NotFound(format!("Entity {} not found", id)))?;

    // Get relation count
    let relation_count = count_entity_relations(&**graph_db, entity.id)
        .await
        .unwrap_or(0);

    // Extract name
    let name = extract_entity_name(&entity.properties);

    // Build EntityInfo
    let entity_info = EntityInfo {
        id: entity.id,
        entity_type: entity.class.clone(),
        name: name.clone(),
        properties: serde_json::to_value(&entity.properties).unwrap_or_default(),
        relation_count,
    };

    // Get relations (both incoming and outgoing)
    let (incoming_relations, outgoing_relations) =
        get_entity_relations(&**graph_db, id, &name).await?;

    let response = EntityDetailResponse {
        entity: entity_info,
        incoming_relations,
        outgoing_relations,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Get incoming and outgoing relations for an entity
async fn get_entity_relations(
    graph_db: &dyn GraphStore,
    entity_id: Uuid,
    entity_name: &str,
) -> Result<(Vec<RelationInfo>, Vec<RelationInfo>), AppError> {
    // We'll need to use raw SurrealDB queries since GraphStore doesn't have relation methods
    // For now, use traverse to get connected entities and infer relationships
    let related_entities = graph_db
        .traverse(entity_id, 1)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to traverse: {}", e)))?;

    // Build relation info (simplified - in real implementation would query relates table)
    let mut outgoing = Vec::new();
    let incoming = Vec::new(); // Currently not populated - would need actual query

    for related in related_entities {
        let target_name = extract_entity_name(&related.properties);

        // Create a relation (we don't have full relation data from traverse)
        let relation = RelationInfo {
            id: Uuid::new_v4(), // Would be actual relation ID
            relation_type: "relates".to_string(), // Would be actual predicate
            source_id: entity_id,
            source_name: entity_name.to_string(),
            target_id: related.id,
            target_name,
            confidence: 0.8, // Default confidence
        };

        outgoing.push(relation);
    }

    Ok((incoming, outgoing))
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

    // Get graph database connection
    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    // Search for matching entities using keyword search
    let initial_entities = graph_db
        .query(&format!(
            "SELECT * FROM entity WHERE properties.text CONTAINS '{}' LIMIT {}",
            req.query.replace('\'', "\\'"),
            req.limit
        ))
        .await
        .map_err(|e| AppError::Internal(format!("Search failed: {}", e)))?;

    if initial_entities.is_empty() {
        return Ok((
            StatusCode::OK,
            Json(GraphSearchResponse {
                metadata: SearchMetadata {
                    query: req.query,
                    depth: req.depth,
                    total_entities: 0,
                    total_relations: 0,
                    processing_time_ms: start.elapsed().as_millis() as u64,
                },
                entities: vec![],
                relations: vec![],
            }),
        ));
    }

    // Expand to subgraph by traversing relations
    let mut all_entities = initial_entities.clone();
    let mut entity_map: HashMap<Uuid, String> = HashMap::new();

    for entity in &initial_entities {
        let name = extract_entity_name(&entity.properties);
        entity_map.insert(entity.id, name);
    }

    // Traverse to get related entities based on depth
    if req.depth > 0 {
        for entity in &initial_entities {
            let related = graph_db
                .traverse(entity.id, req.depth)
                .await
                .map_err(|e| AppError::Internal(format!("Traversal failed: {}", e)))?;

            for rel_entity in related {
                if !entity_map.contains_key(&rel_entity.id) {
                    let name = extract_entity_name(&rel_entity.properties);
                    entity_map.insert(rel_entity.id, name);
                    all_entities.push(rel_entity);
                }
            }
        }
    }

    // Build EntityInfo list
    let mut entity_infos = Vec::new();
    for entity in &all_entities {
        let name = entity_map
            .get(&entity.id)
            .cloned()
            .unwrap_or_else(|| "Unnamed".to_string());

        let relation_count = count_entity_relations(&**graph_db, entity.id)
            .await
            .unwrap_or(0);

        entity_infos.push(EntityInfo {
            id: entity.id,
            entity_type: entity.class.clone(),
            name,
            properties: serde_json::to_value(&entity.properties).unwrap_or_default(),
            relation_count,
        });
    }

    // Build relations between entities in the subgraph
    let mut relations = Vec::new();
    for entity in &all_entities {
        let entity_name = entity_map.get(&entity.id).cloned().unwrap_or_default();

        let (_, outgoing) = get_entity_relations(&**graph_db, entity.id, &entity_name).await?;

        // Only include relations where both ends are in the subgraph
        for rel in outgoing {
            if entity_map.contains_key(&rel.target_id) {
                relations.push(rel);
            }
        }
    }

    let response = GraphSearchResponse {
        metadata: SearchMetadata {
            query: req.query,
            depth: req.depth,
            total_entities: entity_infos.len(),
            total_relations: relations.len(),
            processing_time_ms: start.elapsed().as_millis() as u64,
        },
        entities: entity_infos,
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

    // Query ontology from database or use default schema
    // For now, return the HR ontology schema from Sprint 0
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
                name: "Position".to_string(),
                label: "직위".to_string(),
                parent: None,
            },
            OntologyClass {
                name: "LeaveType".to_string(),
                label: "휴가유형".to_string(),
                parent: None,
            },
            OntologyClass {
                name: "Policy".to_string(),
                label: "정책".to_string(),
                parent: None,
            },
            OntologyClass {
                name: "ApprovalProcess".to_string(),
                label: "승인절차".to_string(),
                parent: None,
            },
            OntologyClass {
                name: "BenefitType".to_string(),
                label: "복리후생".to_string(),
                parent: None,
            },
            OntologyClass {
                name: "Regulation".to_string(),
                label: "규정".to_string(),
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
                name: "manages".to_string(),
                label: "관리".to_string(),
                domain: "Employee".to_string(),
                range: "Department".to_string(),
            },
            OntologyProperty {
                name: "requires".to_string(),
                label: "필요".to_string(),
                domain: "LeaveType".to_string(),
                range: "ApprovalProcess".to_string(),
            },
            OntologyProperty {
                name: "references".to_string(),
                label: "참조".to_string(),
                domain: "Policy".to_string(),
                range: "Regulation".to_string(),
            },
            OntologyProperty {
                name: "appliesTo".to_string(),
                label: "적용대상".to_string(),
                domain: "Policy".to_string(),
                range: "Employee".to_string(),
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
    Json(req): Json<UpdateOntologyRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // TODO: Add proper authentication and admin role check
    // For now, this is a placeholder that validates the request structure

    // Validate classes if provided
    if let Some(classes) = &req.classes {
        if classes.is_empty() {
            return Err(AppError::BadRequest(
                "Classes array cannot be empty".to_string(),
            ));
        }

        // Validate each class
        for class in classes {
            if class.name.is_empty() || class.label.is_empty() {
                return Err(AppError::BadRequest(format!(
                    "Invalid class definition: name and label are required"
                )));
            }
        }
    }

    // Validate properties if provided
    if let Some(properties) = &req.properties {
        if properties.is_empty() {
            return Err(AppError::BadRequest(
                "Properties array cannot be empty".to_string(),
            ));
        }

        // Validate each property
        for prop in properties {
            if prop.name.is_empty()
                || prop.label.is_empty()
                || prop.domain.is_empty()
                || prop.range.is_empty()
            {
                return Err(AppError::BadRequest(format!(
                    "Invalid property definition: all fields are required"
                )));
            }
        }
    }

    // In a real implementation, this would:
    // 1. Check if user has admin role
    // 2. Store the ontology in PostgreSQL metadata table
    // 3. Update SurrealDB schema definitions
    // 4. Invalidate any cached ontology data

    tracing::info!(
        "Ontology update request received (classes: {}, properties: {})",
        req.classes.as_ref().map(|c| c.len()).unwrap_or(0),
        req.properties.as_ref().map(|p| p.len()).unwrap_or(0)
    );

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Ontology update validated successfully",
            "note": "Full implementation pending: requires admin authentication and database storage"
        })),
    ))
}
