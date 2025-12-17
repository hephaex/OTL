//! Graph Search Backend
//!
//! Implements SearchBackend for SurrealDB graph database,
//! enabling subgraph extraction for RAG queries.
//!
//! Author: hephaex@gmail.com

#![allow(clippy::uninlined_format_args)]

use async_trait::async_trait;
use otl_core::{
    AccessLevel, DatabaseConfig, DocumentAcl, OtlError, Result, SearchBackend, SearchResult,
    SearchResultType, SourceReference,
};
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use uuid::Uuid;

// ============================================================================
// Graph Search Backend
// ============================================================================

/// Graph search backend using SurrealDB
pub struct GraphSearchBackend {
    client: Surreal<Client>,
    /// Maximum depth for graph traversal
    max_depth: u32,
    /// Maximum results per query
    max_results: usize,
}

impl GraphSearchBackend {
    /// Create a new graph search backend
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let client = Surreal::new::<Ws>(&config.surrealdb_url)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("SurrealDB connection failed: {e}")))?;

        // Authenticate
        client
            .signin(Root {
                username: &config.surrealdb_user,
                password: &config.surrealdb_pass,
            })
            .await
            .map_err(|e| OtlError::DatabaseError(format!("SurrealDB auth failed: {e}")))?;

        // Select namespace and database
        client
            .use_ns(&config.surrealdb_namespace)
            .use_db(&config.surrealdb_database)
            .await
            .map_err(|e| OtlError::DatabaseError(format!("SurrealDB namespace error: {e}")))?;

        Ok(Self {
            client,
            max_depth: config.surrealdb_namespace.parse().unwrap_or(2),
            max_results: 20,
        })
    }

    /// Set maximum traversal depth
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set maximum results
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Search for entities matching keywords
    async fn search_entities(&self, keywords: &[&str], limit: usize) -> Result<Vec<GraphNode>> {
        // Build search query - search in properties.text field
        let keywords_pattern = keywords.join("|");

        let query = format!(
            r#"
            SELECT *,
                   (properties.text CONTAINS $pattern) AS relevance
            FROM entity
            WHERE properties.text CONTAINS $pattern
            ORDER BY relevance DESC
            LIMIT {}
            "#,
            limit
        );

        let records: Vec<GraphNodeRecord> = self
            .client
            .query(&query)
            .bind(("pattern", keywords_pattern))
            .await
            .map_err(|e| OtlError::SearchError(format!("Entity search failed: {e}")))?
            .take(0)
            .unwrap_or_default();

        Ok(records.into_iter().map(GraphNode::from).collect())
    }

    /// Get related entities via graph traversal
    async fn get_related_entities(&self, entity_ids: &[String], depth: u32) -> Result<Vec<GraphNode>> {
        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build IDs for query
        let ids_str = entity_ids
            .iter()
            .map(|id| format!("entity:{}", id))
            .collect::<Vec<_>>()
            .join(", ");

        // Traverse relationships to find connected entities
        let query = format!(
            r#"
            SELECT *
            FROM (
                SELECT VALUE ->relates->entity
                FROM [{ids_str}]
            )
            UNION
            SELECT *
            FROM (
                SELECT VALUE <-relates<-entity
                FROM [{ids_str}]
            )
            LIMIT {}
            "#,
            depth * 10
        );

        let records: Vec<GraphNodeRecord> = self
            .client
            .query(&query)
            .await
            .map_err(|e| OtlError::SearchError(format!("Traversal failed: {e}")))?
            .take(0)
            .unwrap_or_default();

        Ok(records.into_iter().map(GraphNode::from).collect())
    }

    /// Get relationships between entities
    async fn get_relationships(&self, entity_ids: &[String]) -> Result<Vec<GraphRelation>> {
        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }

        let ids_str = entity_ids
            .iter()
            .map(|id| format!("entity:{}", id))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            r#"
            SELECT *
            FROM relates
            WHERE in IN [{ids_str}] OR out IN [{ids_str}]
            "#
        );

        let records: Vec<RelationRecord> = self
            .client
            .query(&query)
            .await
            .map_err(|e| OtlError::SearchError(format!("Relation query failed: {e}")))?
            .take(0)
            .unwrap_or_default();

        Ok(records.into_iter().map(GraphRelation::from).collect())
    }

    /// Build context from graph nodes and relations
    fn build_context(&self, nodes: &[GraphNode], relations: &[GraphRelation]) -> Vec<SearchResult> {
        let mut results = Vec::new();

        // Convert nodes to search results
        for node in nodes {
            let content = self.format_node_content(node);
            if content.is_empty() {
                continue;
            }

            results.push(SearchResult {
                content,
                score: node.confidence,
                source: SourceReference::new(node.document_id),
                acl: DocumentAcl {
                    access_level: AccessLevel::Internal,
                    ..Default::default()
                },
                result_type: SearchResultType::Graph,
            });
        }

        // Add relation context
        for relation in relations {
            let content = self.format_relation_content(relation, nodes);
            if content.is_empty() {
                continue;
            }

            results.push(SearchResult {
                content,
                score: relation.confidence,
                source: SourceReference::new(Uuid::nil()),
                acl: DocumentAcl {
                    access_level: AccessLevel::Internal,
                    ..Default::default()
                },
                result_type: SearchResultType::Graph,
            });
        }

        results
    }

    /// Format a graph node as text content
    fn format_node_content(&self, node: &GraphNode) -> String {
        let mut parts = Vec::new();

        parts.push(format!("[{}]", node.class));

        if let Some(text) = &node.text {
            parts.push(text.clone());
        }

        // Add important properties
        for (key, value) in &node.properties {
            if key != "text" && key != "start" && key != "end" {
                parts.push(format!("{}: {}", key, value));
            }
        }

        parts.join(" - ")
    }

    /// Format a relation as text content
    fn format_relation_content(&self, relation: &GraphRelation, nodes: &[GraphNode]) -> String {
        let subject = nodes
            .iter()
            .find(|n| n.id == relation.subject_id)
            .and_then(|n| n.text.clone())
            .unwrap_or_else(|| relation.subject_id.clone());

        let object = nodes
            .iter()
            .find(|n| n.id == relation.object_id)
            .and_then(|n| n.text.clone())
            .unwrap_or_else(|| relation.object_id.clone());

        format!("{} [{}] {}", subject, relation.predicate, object)
    }
}

#[async_trait]
impl SearchBackend for GraphSearchBackend {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Extract keywords from query
        let keywords: Vec<&str> = query
            .split_whitespace()
            .filter(|w| w.len() > 1)
            .collect();

        if keywords.is_empty() {
            return Ok(Vec::new());
        }

        // Search for matching entities
        let initial_nodes = self.search_entities(&keywords, limit).await?;

        if initial_nodes.is_empty() {
            return Ok(Vec::new());
        }

        // Get entity IDs for traversal
        let entity_ids: Vec<String> = initial_nodes.iter().map(|n| n.id.clone()).collect();

        // Get related entities via graph traversal
        let related_nodes = self.get_related_entities(&entity_ids, self.max_depth).await?;

        // Get relationships
        let all_ids: Vec<String> = initial_nodes
            .iter()
            .chain(related_nodes.iter())
            .map(|n| n.id.clone())
            .collect();
        let relations = self.get_relationships(&all_ids).await?;

        // Combine nodes
        let mut all_nodes = initial_nodes;
        all_nodes.extend(related_nodes);

        // Build search results
        let results = self.build_context(&all_nodes, &relations);

        // Limit results
        Ok(results.into_iter().take(limit).collect())
    }

    fn name(&self) -> &str {
        "graph"
    }
}

// ============================================================================
// Internal Types
// ============================================================================

/// Graph node representation
#[derive(Debug, Clone)]
struct GraphNode {
    id: String,
    class: String,
    text: Option<String>,
    properties: std::collections::HashMap<String, String>,
    document_id: Uuid,
    confidence: f32,
}

/// Graph node record from SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphNodeRecord {
    id: Option<surrealdb::sql::Thing>,
    class: String,
    properties: serde_json::Value,
    source: Option<SourceRecord>,
    #[serde(default)]
    relevance: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceRecord {
    document_id: String,
    confidence: Option<f32>,
}

impl From<GraphNodeRecord> for GraphNode {
    fn from(record: GraphNodeRecord) -> Self {
        let id = record
            .id
            .map(|t| t.id.to_string())
            .unwrap_or_default();

        let text = record
            .properties
            .get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut properties = std::collections::HashMap::new();
        if let Some(obj) = record.properties.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    properties.insert(k.clone(), s.to_string());
                } else {
                    properties.insert(k.clone(), v.to_string());
                }
            }
        }

        let document_id = record
            .source
            .as_ref()
            .and_then(|s| Uuid::parse_str(&s.document_id).ok())
            .unwrap_or_default();

        let confidence = record
            .source
            .as_ref()
            .and_then(|s| s.confidence)
            .or(record.relevance)
            .unwrap_or(0.5);

        Self {
            id,
            class: record.class,
            text,
            properties,
            document_id,
            confidence,
        }
    }
}

/// Graph relation representation
#[derive(Debug, Clone)]
struct GraphRelation {
    subject_id: String,
    predicate: String,
    object_id: String,
    confidence: f32,
}

/// Relation record from SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelationRecord {
    #[serde(rename = "in")]
    in_id: Option<surrealdb::sql::Thing>,
    #[serde(rename = "out")]
    out_id: Option<surrealdb::sql::Thing>,
    predicate: Option<String>,
    confidence: Option<f32>,
}

impl From<RelationRecord> for GraphRelation {
    fn from(record: RelationRecord) -> Self {
        Self {
            subject_id: record.in_id.map(|t| t.id.to_string()).unwrap_or_default(),
            predicate: record.predicate.unwrap_or_else(|| "relates".to_string()),
            object_id: record.out_id.map(|t| t.id.to_string()).unwrap_or_default(),
            confidence: record.confidence.unwrap_or(0.5),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn test_format_node_content() {
        // Would need full backend for testing, just verify compilation
    }
}
