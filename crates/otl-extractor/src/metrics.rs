//! Quality Metrics module
//!
//! Provides metrics for evaluating extraction quality including
//! precision, recall, F1 score, and accuracy measurements.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{ExtractedEntity, ExtractedRelation};

// ============================================================================
// Entity Metrics
// ============================================================================

/// Metrics for entity extraction evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityMetrics {
    /// True positives (correctly identified entities)
    pub true_positives: usize,
    /// False positives (incorrectly identified as entities)
    pub false_positives: usize,
    /// False negatives (missed entities)
    pub false_negatives: usize,
    /// Total entities in gold standard
    pub gold_total: usize,
    /// Total entities predicted
    pub predicted_total: usize,
}

impl EntityMetrics {
    /// Calculate precision (TP / (TP + FP))
    pub fn precision(&self) -> f32 {
        if self.true_positives + self.false_positives == 0 {
            0.0
        } else {
            self.true_positives as f32 / (self.true_positives + self.false_positives) as f32
        }
    }

    /// Calculate recall (TP / (TP + FN))
    pub fn recall(&self) -> f32 {
        if self.true_positives + self.false_negatives == 0 {
            0.0
        } else {
            self.true_positives as f32 / (self.true_positives + self.false_negatives) as f32
        }
    }

    /// Calculate F1 score (2 * P * R / (P + R))
    pub fn f1_score(&self) -> f32 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }

    /// Calculate accuracy ((TP) / (TP + FP + FN))
    pub fn accuracy(&self) -> f32 {
        let total = self.true_positives + self.false_positives + self.false_negatives;
        if total == 0 {
            0.0
        } else {
            self.true_positives as f32 / total as f32
        }
    }
}

// ============================================================================
// Relation Metrics
// ============================================================================

/// Metrics for relation extraction evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationMetrics {
    /// True positives (correctly identified relations)
    pub true_positives: usize,
    /// False positives (incorrectly identified relations)
    pub false_positives: usize,
    /// False negatives (missed relations)
    pub false_negatives: usize,
    /// Total relations in gold standard
    pub gold_total: usize,
    /// Total relations predicted
    pub predicted_total: usize,
}

impl RelationMetrics {
    /// Calculate precision
    pub fn precision(&self) -> f32 {
        if self.true_positives + self.false_positives == 0 {
            0.0
        } else {
            self.true_positives as f32 / (self.true_positives + self.false_positives) as f32
        }
    }

    /// Calculate recall
    pub fn recall(&self) -> f32 {
        if self.true_positives + self.false_negatives == 0 {
            0.0
        } else {
            self.true_positives as f32 / (self.true_positives + self.false_negatives) as f32
        }
    }

    /// Calculate F1 score
    pub fn f1_score(&self) -> f32 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }
}

// ============================================================================
// Gold Standard Types
// ============================================================================

/// A gold standard entity for evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GoldEntity {
    pub text: String,
    pub entity_type: String,
    pub start: usize,
    pub end: usize,
}

impl From<&ExtractedEntity> for GoldEntity {
    fn from(e: &ExtractedEntity) -> Self {
        Self {
            text: e.text.clone(),
            entity_type: e.entity_type.clone(),
            start: e.start,
            end: e.end,
        }
    }
}

/// A gold standard relation for evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GoldRelation {
    pub subject_text: String,
    pub predicate: String,
    pub object_text: String,
}

impl From<&ExtractedRelation> for GoldRelation {
    fn from(r: &ExtractedRelation) -> Self {
        Self {
            subject_text: r.subject.text.clone(),
            predicate: r.predicate.clone(),
            object_text: r.object.text.clone(),
        }
    }
}

// ============================================================================
// Evaluator
// ============================================================================

/// Evaluator for extraction quality
pub struct Evaluator {
    /// Strict matching (exact span match required)
    strict: bool,
    /// Type matching (entity type must match)
    match_types: bool,
}

impl Evaluator {
    /// Create a new evaluator with default settings
    pub fn new() -> Self {
        Self {
            strict: false,
            match_types: true,
        }
    }

    /// Enable strict span matching
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Enable/disable type matching
    pub fn with_type_matching(mut self, match_types: bool) -> Self {
        self.match_types = match_types;
        self
    }

    /// Check if two entities match
    fn entities_match(&self, predicted: &GoldEntity, gold: &GoldEntity) -> bool {
        // Check type match if required
        if self.match_types && predicted.entity_type != gold.entity_type {
            return false;
        }

        if self.strict {
            // Strict: exact span match
            predicted.start == gold.start && predicted.end == gold.end
        } else {
            // Relaxed: text match
            predicted.text == gold.text
        }
    }

    /// Evaluate entity extraction
    pub fn evaluate_entities(
        &self,
        predicted: &[ExtractedEntity],
        gold: &[GoldEntity],
    ) -> EntityMetrics {
        let predicted_set: Vec<GoldEntity> = predicted.iter().map(GoldEntity::from).collect();
        let _gold_set: HashSet<_> = gold.iter().collect();

        let mut true_positives = 0;
        let mut matched_gold: HashSet<usize> = HashSet::new();

        for pred in &predicted_set {
            for (idx, g) in gold.iter().enumerate() {
                if !matched_gold.contains(&idx) && self.entities_match(pred, g) {
                    true_positives += 1;
                    matched_gold.insert(idx);
                    break;
                }
            }
        }

        let false_positives = predicted_set.len() - true_positives;
        let false_negatives = gold.len() - matched_gold.len();

        EntityMetrics {
            true_positives,
            false_positives,
            false_negatives,
            gold_total: gold.len(),
            predicted_total: predicted_set.len(),
        }
    }

    /// Evaluate relation extraction
    pub fn evaluate_relations(
        &self,
        predicted: &[ExtractedRelation],
        gold: &[GoldRelation],
    ) -> RelationMetrics {
        let predicted_set: HashSet<GoldRelation> =
            predicted.iter().map(GoldRelation::from).collect();
        let gold_set: HashSet<GoldRelation> = gold.iter().cloned().collect();

        let true_positives = predicted_set.intersection(&gold_set).count();
        let false_positives = predicted_set.len() - true_positives;
        let false_negatives = gold_set.len() - true_positives;

        RelationMetrics {
            true_positives,
            false_positives,
            false_negatives,
            gold_total: gold.len(),
            predicted_total: predicted_set.len(),
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Aggregate Metrics
// ============================================================================

/// Aggregate metrics for a batch of evaluations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregateMetrics {
    pub entity_metrics: EntityMetrics,
    pub relation_metrics: RelationMetrics,
    pub num_documents: usize,
}

impl AggregateMetrics {
    /// Add entity metrics to aggregate
    pub fn add_entity_metrics(&mut self, metrics: &EntityMetrics) {
        self.entity_metrics.true_positives += metrics.true_positives;
        self.entity_metrics.false_positives += metrics.false_positives;
        self.entity_metrics.false_negatives += metrics.false_negatives;
        self.entity_metrics.gold_total += metrics.gold_total;
        self.entity_metrics.predicted_total += metrics.predicted_total;
    }

    /// Add relation metrics to aggregate
    pub fn add_relation_metrics(&mut self, metrics: &RelationMetrics) {
        self.relation_metrics.true_positives += metrics.true_positives;
        self.relation_metrics.false_positives += metrics.false_positives;
        self.relation_metrics.false_negatives += metrics.false_negatives;
        self.relation_metrics.gold_total += metrics.gold_total;
        self.relation_metrics.predicted_total += metrics.predicted_total;
    }

    /// Print a summary report
    pub fn report(&self) -> String {
        format!(
            "=== Extraction Quality Report ===\n\n\
             Documents evaluated: {}\n\n\
             Entity Extraction:\n\
               Precision: {:.1}%\n\
               Recall:    {:.1}%\n\
               F1 Score:  {:.1}%\n\
               Gold: {} | Predicted: {} | TP: {} | FP: {} | FN: {}\n\n\
             Relation Extraction:\n\
               Precision: {:.1}%\n\
               Recall:    {:.1}%\n\
               F1 Score:  {:.1}%\n\
               Gold: {} | Predicted: {} | TP: {} | FP: {} | FN: {}\n",
            self.num_documents,
            self.entity_metrics.precision() * 100.0,
            self.entity_metrics.recall() * 100.0,
            self.entity_metrics.f1_score() * 100.0,
            self.entity_metrics.gold_total,
            self.entity_metrics.predicted_total,
            self.entity_metrics.true_positives,
            self.entity_metrics.false_positives,
            self.entity_metrics.false_negatives,
            self.relation_metrics.precision() * 100.0,
            self.relation_metrics.recall() * 100.0,
            self.relation_metrics.f1_score() * 100.0,
            self.relation_metrics.gold_total,
            self.relation_metrics.predicted_total,
            self.relation_metrics.true_positives,
            self.relation_metrics.false_positives,
            self.relation_metrics.false_negatives,
        )
    }

    /// Check if metrics meet Sprint 2 criteria
    pub fn meets_sprint2_criteria(&self) -> (bool, bool) {
        let ner_ok = self.entity_metrics.precision() >= 0.80;
        let re_ok = self.relation_metrics.precision() >= 0.70;
        (ner_ok, re_ok)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_entity(text: &str, entity_type: &str, start: usize) -> ExtractedEntity {
        ExtractedEntity {
            text: text.to_string(),
            entity_type: entity_type.to_string(),
            start,
            end: start + text.len(),
            confidence: 0.9,
        }
    }

    fn create_gold_entity(text: &str, entity_type: &str, start: usize) -> GoldEntity {
        GoldEntity {
            text: text.to_string(),
            entity_type: entity_type.to_string(),
            start,
            end: start + text.len(),
        }
    }

    #[test]
    fn test_entity_metrics_precision() {
        let metrics = EntityMetrics {
            true_positives: 8,
            false_positives: 2,
            false_negatives: 0,
            gold_total: 8,
            predicted_total: 10,
        };

        assert!((metrics.precision() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_entity_metrics_recall() {
        let metrics = EntityMetrics {
            true_positives: 8,
            false_positives: 0,
            false_negatives: 2,
            gold_total: 10,
            predicted_total: 8,
        };

        assert!((metrics.recall() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_entity_metrics_f1() {
        let metrics = EntityMetrics {
            true_positives: 80,
            false_positives: 20,
            false_negatives: 20,
            gold_total: 100,
            predicted_total: 100,
        };

        // P = 0.8, R = 0.8, F1 = 0.8
        assert!((metrics.f1_score() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_evaluate_entities_perfect() {
        let evaluator = Evaluator::new();

        let predicted = vec![
            create_entity("연차휴가", "AnnualLeave", 0),
            create_entity("병가", "SickLeave", 10),
        ];

        let gold = vec![
            create_gold_entity("연차휴가", "AnnualLeave", 0),
            create_gold_entity("병가", "SickLeave", 10),
        ];

        let metrics = evaluator.evaluate_entities(&predicted, &gold);

        assert_eq!(metrics.true_positives, 2);
        assert_eq!(metrics.false_positives, 0);
        assert_eq!(metrics.false_negatives, 0);
        assert!((metrics.precision() - 1.0).abs() < 0.001);
        assert!((metrics.recall() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_evaluate_entities_partial() {
        let evaluator = Evaluator::new();

        let predicted = vec![
            create_entity("연차휴가", "AnnualLeave", 0),
            create_entity("잘못된것", "Wrong", 20), // False positive
        ];

        let gold = vec![
            create_gold_entity("연차휴가", "AnnualLeave", 0),
            create_gold_entity("병가", "SickLeave", 10), // False negative
        ];

        let metrics = evaluator.evaluate_entities(&predicted, &gold);

        assert_eq!(metrics.true_positives, 1);
        assert_eq!(metrics.false_positives, 1);
        assert_eq!(metrics.false_negatives, 1);
        assert!((metrics.precision() - 0.5).abs() < 0.001);
        assert!((metrics.recall() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_aggregate_metrics_report() {
        let aggregate = AggregateMetrics {
            num_documents: 5,
            entity_metrics: EntityMetrics {
                true_positives: 80,
                false_positives: 10,
                false_negatives: 10,
                gold_total: 90,
                predicted_total: 90,
            },
            relation_metrics: RelationMetrics {
                true_positives: 35,
                false_positives: 15,
                false_negatives: 10,
                gold_total: 45,
                predicted_total: 50,
            },
        };

        let report = aggregate.report();
        assert!(report.contains("Documents evaluated: 5"));
        assert!(report.contains("Entity Extraction:"));
        assert!(report.contains("Relation Extraction:"));
    }

    #[test]
    fn test_sprint2_criteria() {
        // Good metrics
        let aggregate = AggregateMetrics {
            num_documents: 0,
            entity_metrics: EntityMetrics {
                true_positives: 85,
                false_positives: 15,
                false_negatives: 10,
                gold_total: 95,
                predicted_total: 100,
            },
            relation_metrics: RelationMetrics {
                true_positives: 75,
                false_positives: 25,
                false_negatives: 10,
                gold_total: 85,
                predicted_total: 100,
            },
        };

        let (ner_ok, re_ok) = aggregate.meets_sprint2_criteria();
        assert!(ner_ok, "NER precision should be >= 80%");
        assert!(re_ok, "RE precision should be >= 70%");
    }
}
