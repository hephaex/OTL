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
// Confidence Thresholds and Review Queues
// ============================================================================

/// Review queue type based on confidence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewQueue {
    /// Auto-approved (confidence >= 0.9)
    AutoApprove,
    /// Quick review needed (0.7 <= confidence < 0.9)
    QuickReview,
    /// Detailed review needed (confidence < 0.7)
    DetailedReview,
}

impl ReviewQueue {
    /// Determine queue based on confidence score
    pub fn from_confidence(confidence: f32) -> Self {
        if confidence >= 0.9 {
            Self::AutoApprove
        } else if confidence >= 0.7 {
            Self::QuickReview
        } else {
            Self::DetailedReview
        }
    }

    /// Get priority level (lower = higher priority)
    pub fn priority(&self) -> i32 {
        match self {
            Self::DetailedReview => 1,  // Highest priority - needs careful review
            Self::QuickReview => 50,     // Medium priority
            Self::AutoApprove => 100,    // Lowest priority - already approved
        }
    }
}

/// Confidence thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceThresholds {
    /// Auto-approve threshold (default: 0.9)
    pub auto_approve: f32,
    /// Quick review threshold (default: 0.7)
    pub quick_review: f32,
}

impl Default for ConfidenceThresholds {
    fn default() -> Self {
        Self {
            auto_approve: 0.9,
            quick_review: 0.7,
        }
    }
}

impl ConfidenceThresholds {
    /// Create with custom thresholds
    pub fn new(auto_approve: f32, quick_review: f32) -> Self {
        Self {
            auto_approve: auto_approve.clamp(0.0, 1.0),
            quick_review: quick_review.clamp(0.0, 1.0),
        }
    }

    /// Determine review queue based on confidence
    pub fn queue_for_confidence(&self, confidence: f32) -> ReviewQueue {
        if confidence >= self.auto_approve {
            ReviewQueue::AutoApprove
        } else if confidence >= self.quick_review {
            ReviewQueue::QuickReview
        } else {
            ReviewQueue::DetailedReview
        }
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
    thresholds: ConfidenceThresholds,
}

impl VerificationQueue {
    /// Create a new verification queue
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            relations: Vec::new(),
            auto_approve_threshold: 0.9, // Legacy field, use thresholds instead
            thresholds: ConfidenceThresholds::default(),
        }
    }

    /// Set auto-approve threshold (legacy method)
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.auto_approve_threshold = threshold.clamp(0.0, 1.0);
        self.thresholds.auto_approve = self.auto_approve_threshold;
        self
    }

    /// Set confidence thresholds
    pub fn with_thresholds(mut self, thresholds: ConfidenceThresholds) -> Self {
        self.auto_approve_threshold = thresholds.auto_approve;
        self.thresholds = thresholds;
        self
    }

    /// Get current thresholds
    pub fn thresholds(&self) -> &ConfidenceThresholds {
        &self.thresholds
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

    /// Get entities by review queue type
    pub fn entities_by_queue(&self, queue: ReviewQueue) -> Vec<&PendingEntity> {
        self.entities
            .iter()
            .filter(|e| {
                e.status == VerificationStatus::Pending
                    && self.thresholds.queue_for_confidence(e.entity.confidence) == queue
            })
            .collect()
    }

    /// Get relations by review queue type
    pub fn relations_by_queue(&self, queue: ReviewQueue) -> Vec<&PendingRelation> {
        self.relations
            .iter()
            .filter(|r| {
                r.status == VerificationStatus::Pending
                    && self.thresholds.queue_for_confidence(r.relation.confidence) == queue
            })
            .collect()
    }

    /// Get all items requiring detailed review
    pub fn detailed_review_items(&self) -> (Vec<&PendingEntity>, Vec<&PendingRelation>) {
        (
            self.entities_by_queue(ReviewQueue::DetailedReview),
            self.relations_by_queue(ReviewQueue::DetailedReview),
        )
    }

    /// Get all items requiring quick review
    pub fn quick_review_items(&self) -> (Vec<&PendingEntity>, Vec<&PendingRelation>) {
        (
            self.entities_by_queue(ReviewQueue::QuickReview),
            self.relations_by_queue(ReviewQueue::QuickReview),
        )
    }

    /// Batch approve entities
    pub fn batch_approve_entities(
        &mut self,
        ids: &[Uuid],
        reviewer: &str,
        note: Option<&str>,
    ) -> BatchOperationResult {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for id in ids {
            if self.approve_entity(*id, reviewer, note) {
                succeeded.push(*id);
            } else {
                failed.push(*id);
            }
        }

        BatchOperationResult { succeeded, failed }
    }

    /// Batch reject entities
    pub fn batch_reject_entities(
        &mut self,
        ids: &[Uuid],
        reviewer: &str,
        reason: &str,
    ) -> BatchOperationResult {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for id in ids {
            if self.reject_entity(*id, reviewer, reason) {
                succeeded.push(*id);
            } else {
                failed.push(*id);
            }
        }

        BatchOperationResult { succeeded, failed }
    }

    /// Batch approve relations
    pub fn batch_approve_relations(
        &mut self,
        ids: &[Uuid],
        reviewer: &str,
        note: Option<&str>,
    ) -> BatchOperationResult {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for id in ids {
            if self.approve_relation(*id, reviewer, note) {
                succeeded.push(*id);
            } else {
                failed.push(*id);
            }
        }

        BatchOperationResult { succeeded, failed }
    }

    /// Batch reject relations
    pub fn batch_reject_relations(
        &mut self,
        ids: &[Uuid],
        reviewer: &str,
        reason: &str,
    ) -> BatchOperationResult {
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for id in ids {
            if self.reject_relation(*id, reviewer, reason) {
                succeeded.push(*id);
            } else {
                failed.push(*id);
            }
        }

        BatchOperationResult { succeeded, failed }
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

    /// Get quality metrics
    pub fn quality_metrics(&self) -> QualityMetrics {
        QualityMetrics::from_queue(self)
    }
}

/// Result of batch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub succeeded: Vec<Uuid>,
    pub failed: Vec<Uuid>,
}

impl BatchOperationResult {
    /// Check if all operations succeeded
    pub fn all_succeeded(&self) -> bool {
        self.failed.is_empty()
    }

    /// Get success count
    pub fn success_count(&self) -> usize {
        self.succeeded.len()
    }

    /// Get failure count
    pub fn failure_count(&self) -> usize {
        self.failed.len()
    }

    /// Get total operations
    pub fn total(&self) -> usize {
        self.succeeded.len() + self.failed.len()
    }

    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        if self.total() == 0 {
            0.0
        } else {
            self.success_count() as f32 / self.total() as f32
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
// Quality Metrics
// ============================================================================

/// Quality metrics for HITL verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Precision: (approved) / (approved + rejected)
    pub precision: f32,

    /// Auto-approval rate: (auto_approved) / (total_reviewed)
    pub auto_approval_rate: f32,

    /// Correction rate: (manually_corrected) / (total_reviewed)
    pub correction_rate: f32,

    /// Average confidence of approved items
    pub avg_approved_confidence: f32,

    /// Average confidence of rejected items
    pub avg_rejected_confidence: f32,

    /// Queue distribution
    pub queue_distribution: QueueDistribution,

    /// Throughput metrics
    pub throughput: ThroughputMetrics,
}

/// Distribution across review queues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDistribution {
    pub auto_approve: usize,
    pub quick_review: usize,
    pub detailed_review: usize,
}

/// Throughput metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub total_processed: usize,
    pub pending: usize,
    pub processing_rate: f32,  // items per hour (if timestamps available)
}

impl QualityMetrics {
    /// Calculate quality metrics from verification queue
    pub fn from_queue(queue: &VerificationQueue) -> Self {
        let approved_entities = queue.approved_entities();
        let approved_relations = queue.approved_relations();
        let approved_count = approved_entities.len() + approved_relations.len();

        let rejected_entities: Vec<_> = queue.entities.iter()
            .filter(|e| e.status == VerificationStatus::Rejected)
            .collect();

        let rejected_relations: Vec<_> = queue.relations.iter()
            .filter(|r| r.status == VerificationStatus::Rejected)
            .collect();

        let total_reviewed = approved_count + rejected_entities.len() + rejected_relations.len();

        // Calculate precision
        let precision = if total_reviewed > 0 {
            approved_count as f32 / total_reviewed as f32
        } else {
            0.0
        };

        // Calculate auto-approval rate
        let auto_approved_count = queue.entities.iter()
            .filter(|e| e.status == VerificationStatus::AutoApproved)
            .count()
            + queue.relations.iter()
            .filter(|r| r.status == VerificationStatus::AutoApproved)
            .count();

        let auto_approval_rate = if total_reviewed > 0 {
            auto_approved_count as f32 / total_reviewed as f32
        } else {
            0.0
        };

        // Calculate correction rate (items with review notes indicating corrections)
        let corrected_count = queue.entities.iter()
            .filter(|e| e.status == VerificationStatus::Approved && e.review_note.is_some())
            .count()
            + queue.relations.iter()
            .filter(|r| r.status == VerificationStatus::Approved && r.review_note.is_some())
            .count();

        let correction_rate = if total_reviewed > 0 {
            corrected_count as f32 / total_reviewed as f32
        } else {
            0.0
        };

        // Calculate average confidences
        let avg_approved_confidence = if approved_count > 0 {
            let sum: f32 = queue.entities.iter()
                .filter(|e| e.status == VerificationStatus::Approved || e.status == VerificationStatus::AutoApproved)
                .map(|e| e.entity.confidence)
                .sum::<f32>()
                + queue.relations.iter()
                .filter(|r| r.status == VerificationStatus::Approved || r.status == VerificationStatus::AutoApproved)
                .map(|r| r.relation.confidence)
                .sum::<f32>();
            sum / approved_count as f32
        } else {
            0.0
        };

        let avg_rejected_confidence = if !rejected_entities.is_empty() || !rejected_relations.is_empty() {
            let sum: f32 = rejected_entities.iter()
                .map(|e| e.entity.confidence)
                .sum::<f32>()
                + rejected_relations.iter()
                .map(|r| r.relation.confidence)
                .sum::<f32>();
            sum / (rejected_entities.len() + rejected_relations.len()) as f32
        } else {
            0.0
        };

        // Calculate queue distribution
        let auto_approve_count = queue.entities_by_queue(ReviewQueue::AutoApprove).len()
            + queue.relations_by_queue(ReviewQueue::AutoApprove).len();
        let quick_review_count = queue.entities_by_queue(ReviewQueue::QuickReview).len()
            + queue.relations_by_queue(ReviewQueue::QuickReview).len();
        let detailed_review_count = queue.entities_by_queue(ReviewQueue::DetailedReview).len()
            + queue.relations_by_queue(ReviewQueue::DetailedReview).len();

        let queue_distribution = QueueDistribution {
            auto_approve: auto_approve_count,
            quick_review: quick_review_count,
            detailed_review: detailed_review_count,
        };

        // Calculate throughput
        let total_processed = total_reviewed;
        let pending = queue.pending_entities().len() + queue.pending_relations().len();

        Self {
            precision,
            auto_approval_rate,
            correction_rate,
            avg_approved_confidence,
            avg_rejected_confidence,
            queue_distribution,
            throughput: ThroughputMetrics {
                total_processed,
                pending,
                processing_rate: 0.0, // Would need timestamps to calculate
            },
        }
    }
}

// ============================================================================
// Feedback Signals for Model Improvement
// ============================================================================

/// Feedback signal for improving extraction models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionFeedback {
    pub id: Uuid,
    pub extraction_type: ExtractionType,
    pub original_confidence: f32,
    pub was_correct: bool,
    pub correction: Option<String>,
    pub context: String,
    pub timestamp: DateTime<Utc>,
}

/// Type of extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionType {
    Entity,
    Relation,
}

impl ExtractionFeedback {
    /// Create feedback from approved entity
    pub fn from_approved_entity(entity: &PendingEntity, correction: Option<String>) -> Self {
        Self {
            id: entity.id,
            extraction_type: ExtractionType::Entity,
            original_confidence: entity.entity.confidence,
            was_correct: correction.is_none(),
            correction,
            context: format!("{}: {}", entity.entity.entity_type, entity.entity.text),
            timestamp: Utc::now(),
        }
    }

    /// Create feedback from rejected entity
    pub fn from_rejected_entity(entity: &PendingEntity, reason: String) -> Self {
        Self {
            id: entity.id,
            extraction_type: ExtractionType::Entity,
            original_confidence: entity.entity.confidence,
            was_correct: false,
            correction: Some(reason),
            context: format!("{}: {}", entity.entity.entity_type, entity.entity.text),
            timestamp: Utc::now(),
        }
    }

    /// Create feedback from approved relation
    pub fn from_approved_relation(relation: &PendingRelation, correction: Option<String>) -> Self {
        Self {
            id: relation.id,
            extraction_type: ExtractionType::Relation,
            original_confidence: relation.relation.confidence,
            was_correct: correction.is_none(),
            correction,
            context: format!(
                "{} -> {} -> {}",
                relation.relation.subject.text,
                relation.relation.predicate,
                relation.relation.object.text
            ),
            timestamp: Utc::now(),
        }
    }

    /// Create feedback from rejected relation
    pub fn from_rejected_relation(relation: &PendingRelation, reason: String) -> Self {
        Self {
            id: relation.id,
            extraction_type: ExtractionType::Relation,
            original_confidence: relation.relation.confidence,
            was_correct: false,
            correction: Some(reason),
            context: format!(
                "{} -> {} -> {}",
                relation.relation.subject.text,
                relation.relation.predicate,
                relation.relation.object.text
            ),
            timestamp: Utc::now(),
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

    #[test]
    fn test_confidence_thresholds() {
        let thresholds = ConfidenceThresholds::new(0.9, 0.7);

        assert_eq!(thresholds.queue_for_confidence(0.95), ReviewQueue::AutoApprove);
        assert_eq!(thresholds.queue_for_confidence(0.80), ReviewQueue::QuickReview);
        assert_eq!(thresholds.queue_for_confidence(0.65), ReviewQueue::DetailedReview);
    }

    #[test]
    fn test_review_queue_from_confidence() {
        assert_eq!(ReviewQueue::from_confidence(0.95), ReviewQueue::AutoApprove);
        assert_eq!(ReviewQueue::from_confidence(0.85), ReviewQueue::QuickReview);
        assert_eq!(ReviewQueue::from_confidence(0.65), ReviewQueue::DetailedReview);
        assert_eq!(ReviewQueue::from_confidence(0.9), ReviewQueue::AutoApprove); // Boundary
        assert_eq!(ReviewQueue::from_confidence(0.7), ReviewQueue::QuickReview); // Boundary
    }

    #[test]
    fn test_review_queue_priority() {
        assert_eq!(ReviewQueue::DetailedReview.priority(), 1);
        assert_eq!(ReviewQueue::QuickReview.priority(), 50);
        assert_eq!(ReviewQueue::AutoApprove.priority(), 100);

        // Detailed review should have highest priority (lowest number)
        assert!(ReviewQueue::DetailedReview.priority() < ReviewQueue::QuickReview.priority());
        assert!(ReviewQueue::QuickReview.priority() < ReviewQueue::AutoApprove.priority());
    }

    #[test]
    fn test_entities_by_queue() {
        // Use a very high threshold so items don't get auto-approved
        let thresholds = ConfidenceThresholds::new(0.99, 0.7);
        let mut queue = VerificationQueue::new().with_thresholds(thresholds);
        let doc_id = Uuid::new_v4();

        // Add entities with different confidence levels
        queue.add_entity(doc_id, create_entity("High", "Type", 0.95));    // Quick review (not auto-approved due to 0.99 threshold)
        queue.add_entity(doc_id, create_entity("Medium", "Type", 0.80));  // Quick review
        queue.add_entity(doc_id, create_entity("Low", "Type", 0.60));     // Detailed review

        let auto_approve = queue.entities_by_queue(ReviewQueue::AutoApprove);
        let quick_review = queue.entities_by_queue(ReviewQueue::QuickReview);
        let detailed_review = queue.entities_by_queue(ReviewQueue::DetailedReview);

        assert_eq!(auto_approve.len(), 0); // None reach 0.99
        assert_eq!(quick_review.len(), 2); // 0.95 and 0.80
        assert_eq!(detailed_review.len(), 1); // 0.60
    }

    #[test]
    fn test_detailed_and_quick_review_items() {
        let mut queue = VerificationQueue::new();
        let doc_id = Uuid::new_v4();

        queue.add_entity(doc_id, create_entity("High", "Type", 0.95));
        queue.add_entity(doc_id, create_entity("Medium1", "Type", 0.80));
        queue.add_entity(doc_id, create_entity("Medium2", "Type", 0.75));
        queue.add_entity(doc_id, create_entity("Low1", "Type", 0.60));
        queue.add_entity(doc_id, create_entity("Low2", "Type", 0.50));

        let (detailed_entities, detailed_relations) = queue.detailed_review_items();
        let (quick_entities, quick_relations) = queue.quick_review_items();

        assert_eq!(detailed_entities.len(), 2);
        assert_eq!(detailed_relations.len(), 0);
        assert_eq!(quick_entities.len(), 2);
        assert_eq!(quick_relations.len(), 0);
    }

    #[test]
    fn test_batch_approve_entities() {
        let mut queue = VerificationQueue::new();
        let doc_id = Uuid::new_v4();

        // Add multiple entities
        let id1 = queue.add_entity(doc_id, create_entity("Entity1", "Type", 0.80));
        let id2 = queue.add_entity(doc_id, create_entity("Entity2", "Type", 0.75));
        let id3 = queue.add_entity(doc_id, create_entity("Entity3", "Type", 0.85));

        // Batch approve
        let result = queue.batch_approve_entities(
            &[id1, id2, id3],
            "reviewer@test.com",
            Some("Batch approved"),
        );

        assert_eq!(result.success_count(), 3);
        assert_eq!(result.failure_count(), 0);
        assert!(result.all_succeeded());
        assert_eq!(result.success_rate(), 1.0);
        assert_eq!(queue.pending_entities().len(), 0);
        assert_eq!(queue.approved_entities().len(), 3);
    }

    #[test]
    fn test_batch_reject_entities() {
        let mut queue = VerificationQueue::new();
        let doc_id = Uuid::new_v4();

        let id1 = queue.add_entity(doc_id, create_entity("Entity1", "Type", 0.80));
        let id2 = queue.add_entity(doc_id, create_entity("Entity2", "Type", 0.75));

        let result = queue.batch_reject_entities(
            &[id1, id2],
            "reviewer@test.com",
            "Incorrect extraction",
        );

        assert_eq!(result.success_count(), 2);
        assert_eq!(result.failure_count(), 0);
        assert_eq!(queue.pending_entities().len(), 0);

        let stats = queue.stats();
        assert_eq!(stats.rejected_entities, 2);
    }

    #[test]
    fn test_batch_operation_with_invalid_ids() {
        let mut queue = VerificationQueue::new();
        let doc_id = Uuid::new_v4();

        let id1 = queue.add_entity(doc_id, create_entity("Entity1", "Type", 0.80));
        let invalid_id = Uuid::new_v4();

        let result = queue.batch_approve_entities(
            &[id1, invalid_id],
            "reviewer@test.com",
            Some("Batch approved"),
        );

        assert_eq!(result.success_count(), 1);
        assert_eq!(result.failure_count(), 1);
        assert!(!result.all_succeeded());
        assert_eq!(result.success_rate(), 0.5);
    }

    #[test]
    fn test_quality_metrics() {
        let mut queue = VerificationQueue::new().with_threshold(0.9);
        let doc_id = Uuid::new_v4();

        // Add and approve some entities
        let id1 = queue.add_entity(doc_id, create_entity("Entity1", "Type", 0.85));
        let _id2 = queue.add_entity(doc_id, create_entity("Entity2", "Type", 0.95)); // Auto-approved

        queue.approve_entity(id1, "reviewer", Some("Corrected"));

        // Add and reject one
        let id3 = queue.add_entity(doc_id, create_entity("Entity3", "Type", 0.70));
        queue.reject_entity(id3, "reviewer", "Wrong type");

        let metrics = queue.quality_metrics();

        // Check metrics
        assert!(metrics.precision > 0.5); // 2 approved, 1 rejected
        assert!(metrics.auto_approval_rate > 0.0);
        assert!(metrics.correction_rate > 0.0); // id1 has correction note
        assert!(metrics.avg_approved_confidence > 0.0);
        assert!(metrics.avg_rejected_confidence > 0.0);
    }

    #[test]
    fn test_extraction_feedback_from_approved() {
        let mut queue = VerificationQueue::new();
        let doc_id = Uuid::new_v4();
        let id = queue.add_entity(doc_id, create_entity("Entity", "Type", 0.85));

        let entity = queue.get_entity(id).unwrap();
        let feedback = ExtractionFeedback::from_approved_entity(entity, None);

        assert_eq!(feedback.extraction_type, ExtractionType::Entity);
        assert_eq!(feedback.original_confidence, 0.85);
        assert!(feedback.was_correct);
        assert!(feedback.correction.is_none());
    }

    #[test]
    fn test_extraction_feedback_from_rejected() {
        let mut queue = VerificationQueue::new();
        let doc_id = Uuid::new_v4();
        let id = queue.add_entity(doc_id, create_entity("Entity", "Type", 0.85));

        let entity = queue.get_entity(id).unwrap();
        let feedback = ExtractionFeedback::from_rejected_entity(
            entity,
            "Wrong entity type".to_string(),
        );

        assert_eq!(feedback.extraction_type, ExtractionType::Entity);
        assert_eq!(feedback.original_confidence, 0.85);
        assert!(!feedback.was_correct);
        assert!(feedback.correction.is_some());
    }

    #[test]
    fn test_batch_operation_result() {
        let result = BatchOperationResult {
            succeeded: vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()],
            failed: vec![Uuid::new_v4()],
        };

        assert_eq!(result.success_count(), 3);
        assert_eq!(result.failure_count(), 1);
        assert_eq!(result.total(), 4);
        assert_eq!(result.success_rate(), 0.75);
        assert!(!result.all_succeeded());
    }

    #[test]
    fn test_with_thresholds() {
        let custom_thresholds = ConfidenceThresholds::new(0.95, 0.8);
        let queue = VerificationQueue::new().with_thresholds(custom_thresholds);

        assert_eq!(queue.thresholds().auto_approve, 0.95);
        assert_eq!(queue.thresholds().quick_review, 0.8);
    }
}
