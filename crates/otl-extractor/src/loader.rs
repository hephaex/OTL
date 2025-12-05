//! Graph Loader module
//!
//! Converts approved extractions to core Entity/Triple types
//! and stores them in the graph database.

use std::collections::HashMap;

use uuid::Uuid;

use otl_core::{Entity, SourceReference, Triple};

use crate::hitl::{PendingEntity, PendingRelation, VerificationStatus};
use crate::{ExtractedEntity, ExtractedRelation};

// ============================================================================
// Conversion utilities
// ============================================================================

/// Convert an ExtractedEntity to a core Entity
pub fn entity_to_core(extracted: &ExtractedEntity, document_id: Uuid) -> Entity {
    let source = SourceReference::new(document_id).with_confidence(extracted.confidence);

    let mut entity = Entity::new(&extracted.entity_type, source);
    entity
        .properties
        .insert("text".to_string(), serde_json::json!(extracted.text));
    entity
        .properties
        .insert("start".to_string(), serde_json::json!(extracted.start));
    entity
        .properties
        .insert("end".to_string(), serde_json::json!(extracted.end));

    entity
}

/// Convert a PendingEntity to a core Entity (only if approved)
pub fn pending_entity_to_core(pending: &PendingEntity) -> Option<Entity> {
    if pending.status != VerificationStatus::Approved
        && pending.status != VerificationStatus::AutoApproved
    {
        return None;
    }

    Some(entity_to_core(&pending.entity, pending.document_id))
}

/// Convert an ExtractedRelation to a core Triple
pub fn relation_to_triple(
    relation: &ExtractedRelation,
    document_id: Uuid,
    subject_entity_id: Uuid,
    object_entity_id: Uuid,
) -> Triple {
    let source = SourceReference::new(document_id).with_confidence(relation.confidence);

    Triple::new(
        subject_entity_id,
        &relation.predicate,
        object_entity_id,
        source,
        relation.confidence,
    )
}

// ============================================================================
// Graph Loader
// ============================================================================

/// Result of loading entities and relations to the graph
#[derive(Debug, Clone, Default)]
pub struct LoadResult {
    /// Number of entities loaded
    pub entities_loaded: usize,
    /// Number of relations loaded
    pub relations_loaded: usize,
    /// Mapping from extracted entity text to entity ID
    pub entity_map: HashMap<String, Uuid>,
    /// Errors encountered during loading
    pub errors: Vec<String>,
}

impl LoadResult {
    /// Check if any items were loaded
    pub fn is_empty(&self) -> bool {
        self.entities_loaded == 0 && self.relations_loaded == 0
    }

    /// Total items loaded
    pub fn total(&self) -> usize {
        self.entities_loaded + self.relations_loaded
    }
}

/// Graph loader for converting extractions to graph entities
pub struct GraphLoader {
    /// Document ID for source reference
    document_id: Uuid,
    /// Entity map (text -> entity ID)
    entity_map: HashMap<String, Uuid>,
    /// Prepared entities
    entities: Vec<Entity>,
    /// Prepared triples
    triples: Vec<Triple>,
}

impl GraphLoader {
    /// Create a new graph loader for a document
    pub fn new(document_id: Uuid) -> Self {
        Self {
            document_id,
            entity_map: HashMap::new(),
            entities: Vec::new(),
            triples: Vec::new(),
        }
    }

    /// Add an extracted entity
    pub fn add_entity(&mut self, extracted: &ExtractedEntity) -> Uuid {
        // Check if we already have this entity text
        if let Some(&id) = self.entity_map.get(&extracted.text) {
            return id;
        }

        let entity = entity_to_core(extracted, self.document_id);
        let id = entity.id;

        self.entity_map.insert(extracted.text.clone(), id);
        self.entities.push(entity);

        id
    }

    /// Add a pending entity (only if approved)
    pub fn add_pending_entity(&mut self, pending: &PendingEntity) -> Option<Uuid> {
        if pending.status != VerificationStatus::Approved
            && pending.status != VerificationStatus::AutoApproved
        {
            return None;
        }

        Some(self.add_entity(&pending.entity))
    }

    /// Add an extracted relation
    pub fn add_relation(&mut self, relation: &ExtractedRelation) -> Option<Uuid> {
        // Look up subject and object entity IDs
        let subject_id = self.entity_map.get(&relation.subject.text)?;
        let object_id = self.entity_map.get(&relation.object.text)?;

        let triple = relation_to_triple(relation, self.document_id, *subject_id, *object_id);
        let id = triple.id;

        self.triples.push(triple);
        Some(id)
    }

    /// Add a pending relation (only if approved)
    pub fn add_pending_relation(&mut self, pending: &PendingRelation) -> Option<Uuid> {
        if pending.status != VerificationStatus::Approved
            && pending.status != VerificationStatus::AutoApproved
        {
            return None;
        }

        self.add_relation(&pending.relation)
    }

    /// Get the prepared entities
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Get the prepared triples
    pub fn triples(&self) -> &[Triple] {
        &self.triples
    }

    /// Get the entity map
    pub fn entity_map(&self) -> &HashMap<String, Uuid> {
        &self.entity_map
    }

    /// Take ownership of entities
    pub fn take_entities(&mut self) -> Vec<Entity> {
        std::mem::take(&mut self.entities)
    }

    /// Take ownership of triples
    pub fn take_triples(&mut self) -> Vec<Triple> {
        std::mem::take(&mut self.triples)
    }

    /// Get load result summary
    pub fn result(&self) -> LoadResult {
        LoadResult {
            entities_loaded: self.entities.len(),
            relations_loaded: self.triples.len(),
            entity_map: self.entity_map.clone(),
            errors: Vec::new(),
        }
    }
}

// ============================================================================
// Batch loader for verification queue
// ============================================================================

/// Load all approved items from a verification queue
pub fn load_approved_from_queue(
    entities: &[PendingEntity],
    relations: &[PendingRelation],
) -> GraphLoader {
    // Get document ID from first entity (assuming all from same document)
    let document_id = entities
        .first()
        .map(|e| e.document_id)
        .unwrap_or_else(Uuid::new_v4);

    let mut loader = GraphLoader::new(document_id);

    // Add approved entities first
    for pending in entities {
        loader.add_pending_entity(pending);
    }

    // Add approved relations
    for pending in relations {
        loader.add_pending_relation(pending);
    }

    loader
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hitl::VerificationQueue;

    fn create_entity(text: &str, entity_type: &str) -> ExtractedEntity {
        ExtractedEntity {
            text: text.to_string(),
            entity_type: entity_type.to_string(),
            start: 0,
            end: text.len(),
            confidence: 0.9,
        }
    }

    fn create_relation(
        subject: ExtractedEntity,
        predicate: &str,
        object: ExtractedEntity,
    ) -> ExtractedRelation {
        ExtractedRelation {
            subject,
            predicate: predicate.to_string(),
            object,
            confidence: 0.85,
        }
    }

    #[test]
    fn test_entity_to_core() {
        let extracted = create_entity("연차휴가", "AnnualLeave");
        let doc_id = Uuid::new_v4();

        let entity = entity_to_core(&extracted, doc_id);

        assert_eq!(entity.class, "AnnualLeave");
        assert_eq!(entity.source.document_id, doc_id);
        assert!(entity.properties.contains_key("text"));
    }

    #[test]
    fn test_graph_loader_add_entity() {
        let doc_id = Uuid::new_v4();
        let mut loader = GraphLoader::new(doc_id);

        let entity = create_entity("병가", "SickLeave");
        let id1 = loader.add_entity(&entity);
        let id2 = loader.add_entity(&entity); // Same entity

        assert_eq!(id1, id2); // Should return same ID
        assert_eq!(loader.entities().len(), 1); // Only one stored
    }

    #[test]
    fn test_graph_loader_add_relation() {
        let doc_id = Uuid::new_v4();
        let mut loader = GraphLoader::new(doc_id);

        let subject = create_entity("병가", "SickLeave");
        let object = create_entity("진단서", "Document");

        // Add entities first
        loader.add_entity(&subject);
        loader.add_entity(&object);

        // Add relation
        let relation = create_relation(subject, "requiresDocument", object);
        let triple_id = loader.add_relation(&relation);

        assert!(triple_id.is_some());
        assert_eq!(loader.triples().len(), 1);
    }

    #[test]
    fn test_load_approved_from_queue() {
        let mut queue = VerificationQueue::new().with_threshold(0.95);
        let doc_id = Uuid::new_v4();

        // Add entities with different confidence levels
        queue.add_entity(doc_id, create_entity("연차휴가", "AnnualLeave")); // pending (0.9)
        queue.add_entity(doc_id, create_entity("병가", "SickLeave")); // pending

        // Auto-approve one with high confidence
        let high_conf = ExtractedEntity {
            text: "진단서".to_string(),
            entity_type: "Document".to_string(),
            start: 0,
            end: 6,
            confidence: 0.98, // Above threshold
        };
        queue.add_entity(doc_id, high_conf);

        // Manually collect entities
        let _all_entities: Vec<_> = queue
            .pending_entities()
            .iter()
            .map(|e| (*e).clone())
            .collect();

        let approved: Vec<_> = queue
            .approved_entities()
            .iter()
            .map(|e| (*e).clone())
            .collect();

        // Load only approved
        let loader = load_approved_from_queue(&approved, &[]);

        assert_eq!(loader.entities().len(), 1); // Only auto-approved one
    }

    #[test]
    fn test_load_result() {
        let doc_id = Uuid::new_v4();
        let mut loader = GraphLoader::new(doc_id);

        loader.add_entity(&create_entity("연차휴가", "AnnualLeave"));
        loader.add_entity(&create_entity("15일", "Days"));

        let result = loader.result();
        assert_eq!(result.entities_loaded, 2);
        assert_eq!(result.relations_loaded, 0);
        assert_eq!(result.total(), 2);
    }
}
