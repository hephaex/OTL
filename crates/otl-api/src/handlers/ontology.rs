//! Ontology management handlers
//!
//! Provides REST API endpoints for ontology schema management:
//! - GET/POST schema versions
//! - Schema validation
//! - Import/Export in multiple formats
//! - Schema comparison and migration
//!
//! Author: hephaex@gmail.com

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;
use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use otl_graph::{
    export_json, export_owl, export_rdf_turtle, export_yaml, import_json, import_yaml,
    EntityTypeDef, GraphStore, OntologyRepository, OntologySchema, PropertyDef, PropertyType,
    RelationTypeDef, SchemaDiff,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

// ============================================================================
// Response Types
// ============================================================================

/// Ontology schema response
#[derive(Debug, Serialize, ToSchema)]
pub struct OntologySchemaResponse {
    /// Schema version
    pub version: String,

    /// Schema name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Entity types
    pub entities: Vec<EntityTypeResponse>,

    /// Relation types
    pub relations: Vec<RelationTypeResponse>,

    /// Creation timestamp
    pub created_at: String,

    /// Last update timestamp
    pub updated_at: String,

    /// Author
    pub author: Option<String>,
}

/// Entity type response
#[derive(Debug, Serialize, ToSchema)]
pub struct EntityTypeResponse {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    pub properties: Vec<PropertyResponse>,
    pub parent: Option<String>,
}

/// Relation type response
#[derive(Debug, Serialize, ToSchema)]
pub struct RelationTypeResponse {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    pub from_entity: String,
    pub to_entity: String,
    pub cardinality: String,
    pub properties: Vec<PropertyResponse>,
}

/// Property response
#[derive(Debug, Serialize, ToSchema)]
pub struct PropertyResponse {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub required: bool,
    pub description: Option<String>,
}

impl From<&OntologySchema> for OntologySchemaResponse {
    fn from(schema: &OntologySchema) -> Self {
        let entities = schema
            .entities
            .values()
            .map(|e| EntityTypeResponse {
                name: e.name.clone(),
                label: e.label.clone(),
                description: e.description.clone(),
                properties: e
                    .properties
                    .values()
                    .map(|p| PropertyResponse {
                        name: p.name.clone(),
                        data_type: format!("{:?}", p.data_type),
                        required: p.required,
                        description: p.description.clone(),
                    })
                    .collect(),
                parent: e.parent.clone(),
            })
            .collect();

        let relations = schema
            .relations
            .values()
            .map(|r| RelationTypeResponse {
                name: r.name.clone(),
                label: r.label.clone(),
                description: r.description.clone(),
                from_entity: r.from_entity.clone(),
                to_entity: r.to_entity.clone(),
                cardinality: format!("{:?}", r.cardinality),
                properties: r
                    .properties
                    .values()
                    .map(|p| PropertyResponse {
                        name: p.name.clone(),
                        data_type: format!("{:?}", p.data_type),
                        required: p.required,
                        description: p.description.clone(),
                    })
                    .collect(),
            })
            .collect();

        Self {
            version: schema.version.to_string(),
            name: schema.name.clone(),
            description: schema.description.clone(),
            entities,
            relations,
            created_at: schema.created_at.to_rfc3339(),
            updated_at: schema.updated_at.to_rfc3339(),
            author: schema.author.clone(),
        }
    }
}

// ============================================================================
// API Handlers
// ============================================================================

/// Get current ontology schema
#[utoipa::path(
    get,
    path = "/api/v1/ontology/schema",
    tag = "ontology",
    responses(
        (status = 200, description = "Current ontology schema", body = OntologySchemaResponse),
        (status = 404, description = "No schema found")
    )
)]
pub async fn get_current_schema(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Get ontology repository from graph DB
    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    // Access SurrealDB client
    let surreal_store = graph_db
        .as_any()
        .downcast_ref::<otl_graph::SurrealDbStore>()
        .ok_or_else(|| AppError::Internal("Invalid graph store type".to_string()))?;

    let ontology_repo = otl_graph::SurrealOntologyRepository::new(surreal_store.client().clone());

    // Get current schema
    let schema = ontology_repo
        .get_current_schema()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get schema: {}", e)))?
        .ok_or_else(|| AppError::NotFound("No ontology schema found".to_string()))?;

    let response = OntologySchemaResponse::from(&schema);

    Ok((StatusCode::OK, Json(response)))
}

/// Get specific schema version
#[utoipa::path(
    get,
    path = "/api/v1/ontology/schema/{version}",
    tag = "ontology",
    params(
        ("version" = String, Path, description = "Schema version (e.g., 1.0.0)")
    ),
    responses(
        (status = 200, description = "Schema version", body = OntologySchemaResponse),
        (status = 404, description = "Schema version not found")
    )
)]
pub async fn get_schema_version(
    State(state): State<Arc<AppState>>,
    Path(version_str): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let version = semver::Version::parse(&version_str)
        .map_err(|e| AppError::BadRequest(format!("Invalid version format: {}", e)))?;

    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    let surreal_store = graph_db
        .as_any()
        .downcast_ref::<otl_graph::SurrealDbStore>()
        .ok_or_else(|| AppError::Internal("Invalid graph store type".to_string()))?;

    let ontology_repo = otl_graph::SurrealOntologyRepository::new(surreal_store.client().clone());

    let schema = ontology_repo
        .get_schema_version(&version)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get schema: {}", e)))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Schema version {} not found", version_str))
        })?;

    let response = OntologySchemaResponse::from(&schema);

    Ok((StatusCode::OK, Json(response)))
}

/// List all schema versions
#[utoipa::path(
    get,
    path = "/api/v1/ontology/schema/versions",
    tag = "ontology",
    responses(
        (status = 200, description = "List of schema versions")
    )
)]
pub async fn list_schema_versions(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    let surreal_store = graph_db
        .as_any()
        .downcast_ref::<otl_graph::SurrealDbStore>()
        .ok_or_else(|| AppError::Internal("Invalid graph store type".to_string()))?;

    let ontology_repo = otl_graph::SurrealOntologyRepository::new(surreal_store.client().clone());

    let versions = ontology_repo
        .list_schema_versions()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to list versions: {}", e)))?;

    Ok((StatusCode::OK, Json(versions)))
}

/// Create new schema request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSchemaRequest {
    /// Schema name
    pub name: String,

    /// Semantic version
    pub version: String,

    /// Description
    pub description: Option<String>,

    /// Entity types
    pub entities: Vec<EntityTypeRequest>,

    /// Relation types
    pub relations: Vec<RelationTypeRequest>,

    /// Author
    pub author: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EntityTypeRequest {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    pub properties: Vec<PropertyRequest>,
    pub parent: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RelationTypeRequest {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    pub from_entity: String,
    pub to_entity: String,
    pub cardinality: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PropertyRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    #[serde(default)]
    pub required: bool,
    pub description: Option<String>,
}

/// Create new ontology schema (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/ontology/schema",
    tag = "ontology",
    request_body = CreateSchemaRequest,
    responses(
        (status = 201, description = "Schema created"),
        (status = 400, description = "Invalid schema"),
        (status = 403, description = "Forbidden - admin only")
    )
)]
pub async fn create_schema(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CreateSchemaRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Check admin role
    if !user.is_admin() {
        return Err(AppError::Forbidden(
            "Admin role required for schema creation".to_string(),
        ));
    }

    // Parse version
    let version = semver::Version::parse(&req.version)
        .map_err(|e| AppError::BadRequest(format!("Invalid version format: {}", e)))?;

    // Build schema
    let mut schema = OntologySchema::new(req.name, version);
    schema.description = req.description;
    schema.author = req.author.or_else(|| Some(user.name.clone()));

    // Add entities
    for entity_req in req.entities {
        let mut entity = EntityTypeDef::new(entity_req.name, entity_req.label);
        entity.description = entity_req.description;
        entity.parent = entity_req.parent;

        for prop_req in entity_req.properties {
            let prop_type = parse_property_type(&prop_req.data_type)?;
            let mut prop = PropertyDef::new(prop_req.name, prop_type);
            prop.required = prop_req.required;
            prop.description = prop_req.description;
            entity.properties.insert(prop.name.clone(), prop);
        }

        schema
            .add_entity_type(entity)
            .map_err(|e| AppError::BadRequest(format!("Invalid entity: {}", e)))?;
    }

    // Add relations
    for rel_req in req.relations {
        let mut relation = RelationTypeDef::new(
            rel_req.name,
            rel_req.label,
            rel_req.from_entity,
            rel_req.to_entity,
        );
        relation.description = rel_req.description;

        schema
            .add_relation_type(relation)
            .map_err(|e| AppError::BadRequest(format!("Invalid relation: {}", e)))?;
    }

    // Validate schema
    schema
        .validate()
        .map_err(|e| AppError::BadRequest(format!("Schema validation failed: {}", e)))?;

    // Store schema
    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    let surreal_store = graph_db
        .as_any()
        .downcast_ref::<otl_graph::SurrealDbStore>()
        .ok_or_else(|| AppError::Internal("Invalid graph store type".to_string()))?;

    let ontology_repo = otl_graph::SurrealOntologyRepository::new(surreal_store.client().clone());

    ontology_repo
        .store_schema(&schema)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to store schema: {}", e)))?;

    let response = OntologySchemaResponse::from(&schema);

    Ok((StatusCode::CREATED, Json(response)))
}

/// Compare two schema versions
#[utoipa::path(
    get,
    path = "/api/v1/ontology/schema/diff",
    tag = "ontology",
    params(
        ("from" = String, Query, description = "Source version"),
        ("to" = String, Query, description = "Target version")
    ),
    responses(
        (status = 200, description = "Schema diff"),
        (status = 404, description = "Schema version not found")
    )
)]
pub async fn compare_schemas(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CompareSchemasQuery>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let from_version = semver::Version::parse(&params.from)
        .map_err(|e| AppError::BadRequest(format!("Invalid 'from' version: {}", e)))?;
    let to_version = semver::Version::parse(&params.to)
        .map_err(|e| AppError::BadRequest(format!("Invalid 'to' version: {}", e)))?;

    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    let surreal_store = graph_db
        .as_any()
        .downcast_ref::<otl_graph::SurrealDbStore>()
        .ok_or_else(|| AppError::Internal("Invalid graph store type".to_string()))?;

    let ontology_repo = otl_graph::SurrealOntologyRepository::new(surreal_store.client().clone());

    let old_schema = ontology_repo
        .get_schema_version(&from_version)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get old schema: {}", e)))?
        .ok_or_else(|| AppError::NotFound(format!("Schema version {} not found", params.from)))?;

    let new_schema = ontology_repo
        .get_schema_version(&to_version)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get new schema: {}", e)))?
        .ok_or_else(|| AppError::NotFound(format!("Schema version {} not found", params.to)))?;

    let diff = SchemaDiff::compare(&old_schema, &new_schema);

    Ok((StatusCode::OK, Json(diff)))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct CompareSchemasQuery {
    pub from: String,
    pub to: String,
}

/// Export schema in specified format
#[utoipa::path(
    get,
    path = "/api/v1/ontology/schema/export",
    tag = "ontology",
    params(
        ("format" = String, Query, description = "Export format: json, yaml, owl, rdf"),
        ("version" = Option<String>, Query, description = "Schema version (default: current)")
    ),
    responses(
        (status = 200, description = "Exported schema"),
        (status = 400, description = "Invalid format"),
        (status = 404, description = "Schema not found")
    )
)]
pub async fn export_schema(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ExportSchemaQuery>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    let surreal_store = graph_db
        .as_any()
        .downcast_ref::<otl_graph::SurrealDbStore>()
        .ok_or_else(|| AppError::Internal("Invalid graph store type".to_string()))?;

    let ontology_repo = otl_graph::SurrealOntologyRepository::new(surreal_store.client().clone());

    // Get schema
    let schema = if let Some(version_str) = &params.version {
        let version = semver::Version::parse(version_str)
            .map_err(|e| AppError::BadRequest(format!("Invalid version: {}", e)))?;
        ontology_repo
            .get_schema_version(&version)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get schema: {}", e)))?
            .ok_or_else(|| {
                AppError::NotFound(format!("Schema version {} not found", version_str))
            })?
    } else {
        ontology_repo
            .get_current_schema()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get schema: {}", e)))?
            .ok_or_else(|| AppError::NotFound("No ontology schema found".to_string()))?
    };

    // Export in requested format
    let (content_type, data) = match params.format.to_lowercase().as_str() {
        "json" => (
            "application/json",
            export_json(&schema)
                .map_err(|e| AppError::Internal(format!("Export failed: {}", e)))?,
        ),
        "yaml" => (
            "application/x-yaml",
            export_yaml(&schema)
                .map_err(|e| AppError::Internal(format!("Export failed: {}", e)))?,
        ),
        "owl" => (
            "application/rdf+xml",
            export_owl(&schema)
                .map_err(|e| AppError::Internal(format!("Export failed: {}", e)))?,
        ),
        "rdf" | "turtle" => (
            "text/turtle",
            export_rdf_turtle(&schema)
                .map_err(|e| AppError::Internal(format!("Export failed: {}", e)))?,
        ),
        _ => {
            return Err(AppError::BadRequest(
                "Invalid format. Supported: json, yaml, owl, rdf".to_string(),
            ))
        }
    };

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        data,
    ))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ExportSchemaQuery {
    pub format: String,
    pub version: Option<String>,
}

/// Import schema from file
#[utoipa::path(
    post,
    path = "/api/v1/ontology/schema/import",
    tag = "ontology",
    params(
        ("format" = String, Query, description = "Import format: json, yaml")
    ),
    responses(
        (status = 201, description = "Schema imported"),
        (status = 400, description = "Invalid schema or format"),
        (status = 403, description = "Forbidden - admin only")
    )
)]
pub async fn import_schema(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ImportSchemaQuery>,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Check admin role
    if !user.is_admin() {
        return Err(AppError::Forbidden(
            "Admin role required for schema import".to_string(),
        ));
    }

    // Parse schema based on format
    let schema_str = String::from_utf8(body.to_vec())
        .map_err(|e| AppError::BadRequest(format!("Invalid UTF-8: {}", e)))?;

    let schema = match params.format.to_lowercase().as_str() {
        "json" => import_json(&schema_str)
            .map_err(|e| AppError::BadRequest(format!("Import failed: {}", e)))?,
        "yaml" => import_yaml(&schema_str)
            .map_err(|e| AppError::BadRequest(format!("Import failed: {}", e)))?,
        _ => {
            return Err(AppError::BadRequest(
                "Invalid format. Supported: json, yaml".to_string(),
            ))
        }
    };

    // Store schema
    let graph_db = state.graph_db.read().await;
    let graph_db = graph_db
        .as_ref()
        .ok_or_else(|| AppError::Internal("Graph database not initialized".to_string()))?;

    let surreal_store = graph_db
        .as_any()
        .downcast_ref::<otl_graph::SurrealDbStore>()
        .ok_or_else(|| AppError::Internal("Invalid graph store type".to_string()))?;

    let ontology_repo = otl_graph::SurrealOntologyRepository::new(surreal_store.client().clone());

    ontology_repo
        .store_schema(&schema)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to store schema: {}", e)))?;

    let response = OntologySchemaResponse::from(&schema);

    Ok((StatusCode::CREATED, Json(response)))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ImportSchemaQuery {
    pub format: String,
}

/// Validate schema without storing
#[utoipa::path(
    post,
    path = "/api/v1/ontology/schema/validate",
    tag = "ontology",
    request_body = CreateSchemaRequest,
    responses(
        (status = 200, description = "Schema is valid"),
        (status = 400, description = "Schema validation failed")
    )
)]
pub async fn validate_schema(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSchemaRequest>,
) -> Result<impl IntoResponse, AppError> {
    state.increment_requests();

    // Parse version
    let version = semver::Version::parse(&req.version)
        .map_err(|e| AppError::BadRequest(format!("Invalid version format: {}", e)))?;

    // Build schema
    let mut schema = OntologySchema::new(req.name, version);
    schema.description = req.description;
    schema.author = req.author;

    // Add entities
    for entity_req in req.entities {
        let mut entity = EntityTypeDef::new(entity_req.name, entity_req.label);
        entity.description = entity_req.description;
        entity.parent = entity_req.parent;

        for prop_req in entity_req.properties {
            let prop_type = parse_property_type(&prop_req.data_type)?;
            let mut prop = PropertyDef::new(prop_req.name, prop_type);
            prop.required = prop_req.required;
            prop.description = prop_req.description;
            entity.properties.insert(prop.name.clone(), prop);
        }

        schema
            .add_entity_type(entity)
            .map_err(|e| AppError::BadRequest(format!("Invalid entity: {}", e)))?;
    }

    // Add relations
    for rel_req in req.relations {
        let relation = RelationTypeDef::new(
            rel_req.name,
            rel_req.label,
            rel_req.from_entity,
            rel_req.to_entity,
        );

        schema
            .add_relation_type(relation)
            .map_err(|e| AppError::BadRequest(format!("Invalid relation: {}", e)))?;
    }

    // Validate schema
    schema
        .validate()
        .map_err(|e| AppError::BadRequest(format!("Schema validation failed: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "valid": true,
            "message": "Schema is valid"
        })),
    ))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse property type from string
fn parse_property_type(type_str: &str) -> Result<PropertyType, AppError> {
    match type_str.to_lowercase().as_str() {
        "string" => Ok(PropertyType::String),
        "integer" => Ok(PropertyType::Integer),
        "float" => Ok(PropertyType::Float),
        "boolean" => Ok(PropertyType::Boolean),
        "datetime" => Ok(PropertyType::DateTime),
        "date" => Ok(PropertyType::Date),
        "time" => Ok(PropertyType::Time),
        "object" => Ok(PropertyType::Object),
        "any" => Ok(PropertyType::Any),
        _ => {
            // Check for reference type: "reference:EntityType"
            if let Some(ref_type) = type_str.strip_prefix("reference:") {
                Ok(PropertyType::Reference(ref_type.to_string()))
            } else if let Some(array_type) = type_str.strip_prefix("array:") {
                let inner_type = parse_property_type(array_type)?;
                Ok(PropertyType::Array(Box::new(inner_type)))
            } else {
                Err(AppError::BadRequest(format!(
                    "Invalid property type: {}",
                    type_str
                )))
            }
        }
    }
}
