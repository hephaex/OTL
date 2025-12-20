//! Human-in-the-Loop (HITL) verification module
//!
//! Provides types and functions for managing the verification workflow
//! of extracted entities and relations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ExtractedEntity, ExtractedRelation};

// ============================================================================
// Verification Status
// ============================================================================

/// Status of an extracted item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// Pending review
    Pending,
    /// Approved by human
    Approved,
    /// Rejected by human
    Rejected,
    /// Auto-approved (high confidence)
    AutoApproved,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
            Self::AutoApproved => write!(f, "auto_approved"),
        }
    }
}

// ============================================================================
// Verification Item Types
// ============================================================================

/// An entity awaiting verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingEntity {
    pub id: Uuid,
    pub document_id: Uuid,
    pub chunk_id: Option<Uuid>,
    pub entity: ExtractedEntity,
    pub status: VerificationStatus,
    pub reviewer: Option<String>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

impl PendingEntity {
    /// Create a new pending entity
    pub fn new(document_id: Uuid, entity: ExtractedEntity) -> Self {
        Self {
            id: Uuid::new_v4(),
            document_id,
            chunk_id: None,
            entity,
            status: VerificationStatus::Pending,
            reviewer: None,
            review_note: None,
            created_at: Utc::now(),
            reviewed_at: None,
        }
    }

    /// Create with chunk association
    pub fn with_chunk(mut self, chunk_id: Uuid) -> Self {
        self.chunk_id = Some(chunk_id);
        self
    }

    /// Auto-approve if confidence is above threshold
    pub fn auto_approve_if_confident(mut self, threshold: f32) -> Self {
        if self.entity.confidence >= threshold {
            self.status = VerificationStatus::AutoApproved;
            self.reviewed_at = Some(Utc::now());
        }
        self
    }
}

/// A relation awaiting verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRelation {
    pub id: Uuid,
    pub document_id: Uuid,
    pub relation: ExtractedRelation,
    pub status: VerificationStatus,
    pub reviewer: Option<String>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

impl PendingRelation {
    /// Create a new pending relation
    pub fn new(document_id: Uuid, relation: ExtractedRelation) -> Self {
        Self {
            id: Uuid::new_v4(),
            document_id,
            relation,
            status: VerificationStatus::Pending,
            reviewer: None,
            review_note: None,
            created_at: Utc::now(),
            reviewed_at: None,
        }
    }

    /// Auto-approve if confidence is above threshold
    pub fn auto_approve_if_confident(mut self, threshold: f32) -> Self {
        if self.relation.confidence >= threshold {
            self.status = VerificationStatus::AutoApproved;
            self.reviewed_at = Some(Utc::now());
        }
        self
    }
}

// ============================================================================
// Verification Queue
// ============================================================================

/// In-memory verification queue
#[derive(Debug, Default)]
pub struct VerificationQueue {
    entities: Vec<PendingEntity>,
    relations: Vec<PendingRelation>,
    auto_approve_threshold: f32,
}

impl VerificationQueue {
    /// Create a new verification queue
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            relations: Vec::new(),
            auto_approve_threshold: 0.95, // Default: auto-approve at 95% confidence
        }
    }

    /// Set auto-approve threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.auto_approve_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Add an entity for verification
    pub fn add_entity(&mut self, document_id: Uuid, entity: ExtractedEntity) -> Uuid {
        let pending = PendingEntity::new(document_id, entity)
            .auto_approve_if_confident(self.auto_approve_threshold);
        let id = pending.id;
        self.entities.push(pending);
        id
    }

    /// Add a relation for verification
    pub fn add_relation(&mut self, document_id: Uuid, relation: ExtractedRelation) -> Uuid {
        let pending = PendingRelation::new(document_id, relation)
            .auto_approve_if_confident(self.auto_approve_threshold);
        let id = pending.id;
        self.relations.push(pending);
        id
    }

    /// Get all pending entities
    pub fn pending_entities(&self) -> Vec<&PendingEntity> {
        self.entities
            .iter()
            .filter(|e| e.status == VerificationStatus::Pending)
            .collect()
    }

    /// Get all pending relations
    pub fn pending_relations(&self) -> Vec<&PendingRelation> {
        self.relations
            .iter()
            .filter(|r| r.status == VerificationStatus::Pending)
            .collect()
    }

    /// Get all approved entities
    pub fn approved_entities(&self) -> Vec<&PendingEntity> {
        self.entities
            .iter()
            .filter(|e| {
                e.status == VerificationStatus::Approved
                    || e.status == VerificationStatus::AutoApproved
            })
            .collect()
    }

    /// Get all approved relations
    pub fn approved_relations(&self) -> Vec<&PendingRelation> {
        self.relations
            .iter()
            .filter(|r| {
                r.status == VerificationStatus::Approved
                    || r.status == VerificationStatus::AutoApproved
            })
            .collect()
    }

    /// Approve an entity
    pub fn approve_entity(&mut self, id: Uuid, reviewer: &str, note: Option<&str>) -> bool {
        if let Some(entity) = self.entities.iter_mut().find(|e| e.id == id) {
            entity.status = VerificationStatus::Approved;
            entity.reviewer = Some(reviewer.to_string());
            entity.review_note = note.map(String::from);
            entity.reviewed_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    /// Reject an entity
    pub fn reject_entity(&mut self, id: Uuid, reviewer: &str, reason: &str) -> bool {
        if let Some(entity) = self.entities.iter_mut().find(|e| e.id == id) {
            entity.status = VerificationStatus::Rejected;
            entity.reviewer = Some(reviewer.to_string());
            entity.review_note = Some(reason.to_string());
            entity.reviewed_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    /// Approve a relation
    pub fn approve_relation(&mut self, id: Uuid, reviewer: &str, note: Option<&str>) -> bool {
        if let Some(relation) = self.relations.iter_mut().find(|r| r.id == id) {
            relation.status = VerificationStatus::Approved;
            relation.reviewer = Some(reviewer.to_string());
            relation.review_note = note.map(String::from);
            relation.reviewed_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    /// Reject a relation
    pub fn reject_relation(&mut self, id: Uuid, reviewer: &str, reason: &str) -> bool {
        if let Some(relation) = self.relations.iter_mut().find(|r| r.id == id) {
            relation.status = VerificationStatus::Rejected;
            relation.reviewer = Some(reviewer.to_string());
            relation.review_note = Some(reason.to_string());
            relation.reviewed_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    /// Get entity by ID
    pub fn get_entity(&self, id: Uuid) -> Option<&PendingEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Get relation by ID
    pub fn get_relation(&self, id: Uuid) -> Option<&PendingRelation> {
        self.relations.iter().find(|r| r.id == id)
    }

    /// Get statistics
    pub fn stats(&self) -> VerificationStats {
        VerificationStats {
            pending_entities: self.pending_entities().len(),
            pending_relations: self.pending_relations().len(),
            approved_entities: self
                .entities
                .iter()
                .filter(|e| e.status == VerificationStatus::Approved)
                .count(),
            auto_approved_entities: self
                .entities
                .iter()
                .filter(|e| e.status == VerificationStatus::AutoApproved)
                .count(),
            rejected_entities: self
                .entities
                .iter()
                .filter(|e| e.status == VerificationStatus::Rejected)
                .count(),
            approved_relations: self
                .relations
                .iter()
                .filter(|r| r.status == VerificationStatus::Approved)
                .count(),
            auto_approved_relations: self
                .relations
                .iter()
                .filter(|r| r.status == VerificationStatus::AutoApproved)
                .count(),
            rejected_relations: self
                .relations
                .iter()
                .filter(|r| r.status == VerificationStatus::Rejected)
                .count(),
        }
    }
}

/// Verification statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStats {
    pub pending_entities: usize,
    pub pending_relations: usize,
    pub approved_entities: usize,
    pub auto_approved_entities: usize,
    pub rejected_entities: usize,
    pub approved_relations: usize,
    pub auto_approved_relations: usize,
    pub rejected_relations: usize,
}

impl VerificationStats {
    /// Total entities processed
    pub fn total_entities(&self) -> usize {
        self.pending_entities
            + self.approved_entities
            + self.auto_approved_entities
            + self.rejected_entities
    }

    /// Total relations processed
    pub fn total_relations(&self) -> usize {
        self.pending_relations
            + self.approved_relations
            + self.auto_approved_relations
            + self.rejected_relations
    }

    /// Entity approval rate
    pub fn entity_approval_rate(&self) -> f32 {
        let total = self.approved_entities + self.auto_approved_entities + self.rejected_entities;
        if total == 0 {
            0.0
        } else {
            (self.approved_entities + self.auto_approved_entities) as f32 / total as f32
        }
    }

    /// Relation approval rate
    pub fn relation_approval_rate(&self) -> f32 {
        let total =
            self.approved_relations + self.auto_approved_relations + self.rejected_relations;
        if total == 0 {
            0.0
        } else {
            (self.approved_relations + self.auto_approved_relations) as f32 / total as f32
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_entity(text: &str, entity_type: &str, confidence: f32) -> ExtractedEntity {
        ExtractedEntity {
            text: text.to_string(),
            entity_type: entity_type.to_string(),
            start: 0,
            end: text.len(),
            confidence,
        }
    }

    fn create_relation(
        subject: &str,
        predicate: &str,
        object: &str,
        confidence: f32,
    ) -> ExtractedRelation {
        ExtractedRelation {
            subject: create_entity(subject, "Subject", 0.9),
            predicate: predicate.to_string(),
            object: create_entity(object, "Object", 0.9),
            confidence,
        }
    }

    #[test]
    fn test_verification_queue_add_entity() {
        let mut queue = VerificationQueue::new();
        let doc_id = Uuid::new_v4();
        let entity = create_entity("연차휴가", "AnnualLeave", 0.85);

        let id = queue.add_entity(doc_id, entity);
        assert!(!id.is_nil());
        assert_eq!(queue.pending_entities().len(), 1);
    }

    #[test]
    fn test_auto_approve_high_confidence() {
        let mut queue = VerificationQueue::new().with_threshold(0.9);
        let doc_id = Uuid::new_v4();

        // Low confidence - should be pending
        let low = create_entity("연차휴가", "AnnualLeave", 0.85);
        queue.add_entity(doc_id, low);

        // High confidence - should be auto-approved
        let high = create_entity("병가", "SickLeave", 0.95);
        queue.add_entity(doc_id, high);

        assert_eq!(queue.pending_entities().len(), 1);
        assert_eq!(queue.approved_entities().len(), 1);
    }

    #[test]
    fn test_approve_entity() {
        let mut queue = VerificationQueue::new();
        let doc_id = Uuid::new_v4();
        let entity = create_entity("연차휴가", "AnnualLeave", 0.85);

        let id = queue.add_entity(doc_id, entity);
        assert_eq!(queue.pending_entities().len(), 1);

        let result = queue.approve_entity(id, "reviewer@test.com", Some("Looks good"));
        assert!(result);
        assert_eq!(queue.pending_entities().len(), 0);
        assert_eq!(queue.approved_entities().len(), 1);
    }

    #[test]
    fn test_reject_entity() {
        let mut queue = VerificationQueue::new();
        let doc_id = Uuid::new_v4();
        let entity = create_entity("연차휴가", "AnnualLeave", 0.85);

        let id = queue.add_entity(doc_id, entity);
        let result = queue.reject_entity(id, "reviewer@test.com", "Incorrect entity type");
        assert!(result);

        assert_eq!(queue.pending_entities().len(), 0);
        assert_eq!(queue.approved_entities().len(), 0);

        let stats = queue.stats();
        assert_eq!(stats.rejected_entities, 1);
    }

    #[test]
    fn test_verification_stats() {
        let mut queue = VerificationQueue::new().with_threshold(0.9);
        let doc_id = Uuid::new_v4();

        queue.add_entity(doc_id, create_entity("Entity1", "Type1", 0.85));
        queue.add_entity(doc_id, create_entity("Entity2", "Type2", 0.95));
        queue.add_relation(doc_id, create_relation("Sub", "pred", "Obj", 0.80));

        let stats = queue.stats();
        assert_eq!(stats.pending_entities, 1);
        assert_eq!(stats.auto_approved_entities, 1);
        assert_eq!(stats.pending_relations, 1);
    }
}
