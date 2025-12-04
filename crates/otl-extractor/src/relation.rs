//! Relation Extraction (RE) module
//!
//! Extracts relations (triples) between entities from text.
//! Supports both rule-based and LLM-based extraction.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{ExtractedEntity, ExtractedRelation, RelationExtractor};
use otl_core::Result;

// ============================================================================
// Relation Types for HR Domain
// ============================================================================

/// Relation types recognized by the RE system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    // Employment relations
    WorksIn,     // Employee -> Department
    HasPosition, // Employee -> Position
    HasGrade,    // Employee -> Grade
    ManagedBy,   // Employee -> Manager
    ReportsTo,   // Employee -> Manager

    // Leave relations
    RequestsLeave,    // Employee -> LeaveType
    ApprovesLeave,    // Manager -> LeaveType
    RequiresDuration, // LeaveType -> Duration
    RequiresDocument, // LeaveType -> Document

    // Approval relations
    HasStep,    // ApprovalProcess -> ApprovalStep
    ApprovedBy, // ApprovalStep -> Role/Manager
    NextStep,   // ApprovalStep -> ApprovalStep

    // Document relations
    DefinedIn,  // Entity -> Regulation
    References, // Document -> Document/Regulation
    Requires,   // Process -> Document

    // Generic
    HasValue,  // Entity -> Amount/Duration
    RelatedTo, // Generic relation
}

impl RelationType {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WorksIn => "worksIn",
            Self::HasPosition => "hasPosition",
            Self::HasGrade => "hasGrade",
            Self::ManagedBy => "managedBy",
            Self::ReportsTo => "reportsTo",
            Self::RequestsLeave => "requestsLeave",
            Self::ApprovesLeave => "approvesLeave",
            Self::RequiresDuration => "requiresDuration",
            Self::RequiresDocument => "requiresDocument",
            Self::HasStep => "hasStep",
            Self::ApprovedBy => "approvedBy",
            Self::NextStep => "nextStep",
            Self::DefinedIn => "definedIn",
            Self::References => "references",
            Self::Requires => "requires",
            Self::HasValue => "hasValue",
            Self::RelatedTo => "relatedTo",
        }
    }

    /// Get from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "worksin" => Some(Self::WorksIn),
            "hasposition" => Some(Self::HasPosition),
            "hasgrade" => Some(Self::HasGrade),
            "managedby" => Some(Self::ManagedBy),
            "reportsto" => Some(Self::ReportsTo),
            "requestsleave" => Some(Self::RequestsLeave),
            "approvesleave" => Some(Self::ApprovesLeave),
            "requiresduration" => Some(Self::RequiresDuration),
            "requiresdocument" => Some(Self::RequiresDocument),
            "hasstep" => Some(Self::HasStep),
            "approvedby" => Some(Self::ApprovedBy),
            "nextstep" => Some(Self::NextStep),
            "definedin" => Some(Self::DefinedIn),
            "references" => Some(Self::References),
            "requires" => Some(Self::Requires),
            "hasvalue" => Some(Self::HasValue),
            "relatedto" => Some(Self::RelatedTo),
            _ => None,
        }
    }
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Rule-based RE
// ============================================================================

/// Pattern for extracting relations
#[derive(Debug, Clone)]
pub struct RelationPattern {
    /// Subject entity type
    pub subject_type: String,
    /// Object entity type
    pub object_type: String,
    /// Relation type
    pub relation: RelationType,
    /// Pattern keywords (between subject and object)
    pub keywords: Vec<String>,
    /// Maximum distance between entities (in characters)
    pub max_distance: usize,
    /// Confidence score
    pub confidence: f32,
}

/// Rule-based relation extractor
pub struct RuleBasedRe {
    patterns: Vec<RelationPattern>,
}

impl RuleBasedRe {
    /// Create a new rule-based RE with HR domain patterns
    pub fn new() -> Self {
        let mut re = Self {
            patterns: Vec::new(),
        };
        re.init_hr_patterns();
        re
    }

    /// Initialize HR domain relation patterns
    fn init_hr_patterns(&mut self) {
        // Leave type requires duration
        self.add_pattern(
            "AnnualLeave",
            "Days",
            RelationType::RequiresDuration,
            vec!["최대", "기본", "부여", "사용"],
            100,
            0.85,
        );
        self.add_pattern(
            "SickLeave",
            "Days",
            RelationType::RequiresDuration,
            vec!["최대", "한도", "까지"],
            100,
            0.85,
        );
        self.add_pattern(
            "ParentalLeave",
            "Duration",
            RelationType::RequiresDuration,
            vec!["최대", "까지", "동안"],
            100,
            0.85,
        );

        // Leave requires document
        self.add_pattern(
            "SickLeave",
            "Document",
            RelationType::RequiresDocument,
            vec!["필요", "제출", "첨부"],
            150,
            0.80,
        );

        // Approval relations
        self.add_pattern(
            "ApprovalProcess",
            "ApprovalStep",
            RelationType::HasStep,
            vec!["단계", "거쳐", "순서"],
            100,
            0.80,
        );
        self.add_pattern(
            "ApprovalStep",
            "Manager",
            RelationType::ApprovedBy,
            vec!["승인", "결재", "허가"],
            80,
            0.85,
        );

        // Regulation references
        self.add_pattern(
            "LeaveType",
            "Regulation",
            RelationType::DefinedIn,
            vec!["정의", "규정", "따라", "의거"],
            100,
            0.80,
        );

        // Employee relations
        self.add_pattern(
            "Employee",
            "Department",
            RelationType::WorksIn,
            vec!["소속", "근무", "배치"],
            80,
            0.85,
        );
        self.add_pattern(
            "Employee",
            "Position",
            RelationType::HasPosition,
            vec!["직위", "직급", "임명"],
            80,
            0.85,
        );
    }

    /// Add a relation pattern
    fn add_pattern(
        &mut self,
        subject_type: &str,
        object_type: &str,
        relation: RelationType,
        keywords: Vec<&str>,
        max_distance: usize,
        confidence: f32,
    ) {
        self.patterns.push(RelationPattern {
            subject_type: subject_type.to_string(),
            object_type: object_type.to_string(),
            relation,
            keywords: keywords.iter().map(|s| s.to_string()).collect(),
            max_distance,
            confidence,
        });
    }

    /// Check if text between entities contains keywords
    fn contains_keywords(&self, text: &str, keywords: &[String]) -> bool {
        let text_lower = text.to_lowercase();
        keywords
            .iter()
            .any(|k| text_lower.contains(&k.to_lowercase()))
    }

    /// Find relations between entities based on patterns
    fn find_pattern_relations(
        &self,
        text: &str,
        entities: &[ExtractedEntity],
    ) -> Vec<ExtractedRelation> {
        let mut relations = Vec::new();

        for pattern in &self.patterns {
            // Find matching subject entities
            let subjects: Vec<&ExtractedEntity> = entities
                .iter()
                .filter(|e| e.entity_type == pattern.subject_type)
                .collect();

            // Find matching object entities
            let objects: Vec<&ExtractedEntity> = entities
                .iter()
                .filter(|e| e.entity_type == pattern.object_type)
                .collect();

            // Check each subject-object pair
            for subject in &subjects {
                for object in &objects {
                    // Skip if same entity
                    if subject.start == object.start {
                        continue;
                    }

                    // Calculate distance between entities
                    let (first, second) = if subject.start < object.start {
                        (subject, object)
                    } else {
                        (object, subject)
                    };

                    let distance = second.start.saturating_sub(first.end);

                    // Check if within max distance
                    if distance > pattern.max_distance {
                        continue;
                    }

                    // Check context: text between first and second entity,
                    // plus text after second entity (up to max_distance)
                    // Handle UTF-8 boundaries safely
                    let context_start = first.end;
                    let mut context_end = (second.end + pattern.max_distance).min(text.len());

                    // Ensure we're at a valid char boundary
                    while context_end > context_start && !text.is_char_boundary(context_end) {
                        context_end -= 1;
                    }

                    if context_end > context_start && text.is_char_boundary(context_start) {
                        let context_text = &text[context_start..context_end];

                        // Check for keywords in context
                        if self.contains_keywords(context_text, &pattern.keywords) {
                            relations.push(ExtractedRelation {
                                subject: (*subject).clone(),
                                predicate: pattern.relation.to_string(),
                                object: (*object).clone(),
                                confidence: pattern.confidence,
                            });
                        }
                    }
                }
            }
        }

        relations
    }
}

impl Default for RuleBasedRe {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationExtractor for RuleBasedRe {
    fn extract(&self, text: &str, entities: &[ExtractedEntity]) -> Result<Vec<ExtractedRelation>> {
        let relations = self.find_pattern_relations(text, entities);
        Ok(relations)
    }
}

// ============================================================================
// LLM-based RE
// ============================================================================

/// Configuration for LLM-based RE
#[derive(Debug, Clone)]
pub struct LlmReConfig {
    /// System prompt
    pub system_prompt: String,
    /// Expected relation types
    pub relation_types: Vec<RelationType>,
    /// Maximum tokens
    pub max_tokens: usize,
    /// Temperature
    pub temperature: f32,
}

impl Default for LlmReConfig {
    fn default() -> Self {
        Self {
            system_prompt: include_str!("prompts/re_system.txt").to_string(),
            relation_types: vec![
                RelationType::RequiresDuration,
                RelationType::RequiresDocument,
                RelationType::DefinedIn,
                RelationType::HasStep,
                RelationType::ApprovedBy,
            ],
            max_tokens: 1024,
            temperature: 0.1,
        }
    }
}

/// LLM-based relation extractor
pub struct LlmRe {
    pub config: LlmReConfig,
}

impl LlmRe {
    /// Create a new LLM RE with default config
    pub fn new() -> Self {
        Self {
            config: LlmReConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: LlmReConfig) -> Self {
        Self { config }
    }

    /// Build the extraction prompt
    pub fn build_prompt(&self, text: &str, entities: &[ExtractedEntity]) -> String {
        let relation_types: Vec<&str> = self
            .config
            .relation_types
            .iter()
            .map(|r| r.as_str())
            .collect();

        let entities_json: Vec<serde_json::Value> = entities
            .iter()
            .map(|e| {
                serde_json::json!({
                    "text": e.text,
                    "type": e.entity_type,
                    "start": e.start,
                    "end": e.end
                })
            })
            .collect();

        format!(
            "{}\n\nRelation types to extract: {}\n\nEntities:\n{}\n\nText:\n{}\n\nExtract relations in JSON format:",
            self.config.system_prompt,
            relation_types.join(", "),
            serde_json::to_string_pretty(&entities_json).unwrap_or_default(),
            text
        )
    }

    /// Parse LLM response into relations
    pub fn parse_response(
        &self,
        response: &str,
        entities: &[ExtractedEntity],
    ) -> Vec<ExtractedRelation> {
        let parsed: Vec<LlmRelation> = serde_json::from_str(response).unwrap_or_default();

        parsed
            .into_iter()
            .filter_map(|r| {
                // Find subject and object entities
                let subject = entities.iter().find(|e| e.text == r.subject)?;
                let object = entities.iter().find(|e| e.text == r.object)?;

                Some(ExtractedRelation {
                    subject: subject.clone(),
                    predicate: r.predicate,
                    object: object.clone(),
                    confidence: r.confidence.unwrap_or(0.8),
                })
            })
            .collect()
    }
}

impl Default for LlmRe {
    fn default() -> Self {
        Self::new()
    }
}

/// Relation structure for LLM JSON output
#[derive(Debug, Deserialize)]
struct LlmRelation {
    subject: String,
    predicate: String,
    object: String,
    confidence: Option<f32>,
}

// ============================================================================
// Hybrid RE
// ============================================================================

/// Hybrid RE combining rule-based and LLM approaches
pub struct HybridRe {
    rule_re: RuleBasedRe,
    llm_re: Option<LlmRe>,
    /// Weight for rule-based results
    rule_weight: f32,
    /// Confidence threshold
    confidence_threshold: f32,
}

impl HybridRe {
    /// Create a new hybrid RE
    pub fn new() -> Self {
        Self {
            rule_re: RuleBasedRe::new(),
            llm_re: None,
            rule_weight: 0.6,
            confidence_threshold: 0.5,
        }
    }

    /// Enable LLM extraction
    pub fn with_llm(mut self, llm_re: LlmRe) -> Self {
        self.llm_re = Some(llm_re);
        self
    }

    /// Merge relations from multiple sources
    pub fn merge_relations(
        &self,
        rule_relations: Vec<ExtractedRelation>,
        llm_relations: Vec<ExtractedRelation>,
    ) -> Vec<ExtractedRelation> {
        let mut merged: HashMap<(String, String, String), ExtractedRelation> = HashMap::new();

        // Add rule-based relations
        for mut rel in rule_relations {
            rel.confidence *= self.rule_weight;
            let key = (
                rel.subject.text.clone(),
                rel.predicate.clone(),
                rel.object.text.clone(),
            );

            if let Some(existing) = merged.get_mut(&key) {
                existing.confidence = (existing.confidence + rel.confidence).min(1.0);
            } else {
                merged.insert(key, rel);
            }
        }

        // Add LLM relations
        let llm_weight = 1.0 - self.rule_weight;
        for mut rel in llm_relations {
            rel.confidence *= llm_weight;
            let key = (
                rel.subject.text.clone(),
                rel.predicate.clone(),
                rel.object.text.clone(),
            );

            if let Some(existing) = merged.get_mut(&key) {
                existing.confidence = (existing.confidence + rel.confidence).min(1.0);
            } else {
                merged.insert(key, rel);
            }
        }

        // Filter by threshold and collect
        merged
            .into_values()
            .filter(|r| r.confidence >= self.confidence_threshold)
            .collect()
    }
}

impl Default for HybridRe {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationExtractor for HybridRe {
    fn extract(&self, text: &str, entities: &[ExtractedEntity]) -> Result<Vec<ExtractedRelation>> {
        // Always run rule-based extraction
        let rule_relations = self.rule_re.extract(text, entities)?;

        // If LLM not configured, return rule-based only
        if self.llm_re.is_none() {
            return Ok(rule_relations);
        }

        // For now, return rule-based only
        // LLM integration requires async runtime
        Ok(rule_relations)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_entity(text: &str, entity_type: &str, start: usize, end: usize) -> ExtractedEntity {
        ExtractedEntity {
            text: text.to_string(),
            entity_type: entity_type.to_string(),
            start,
            end,
            confidence: 0.9,
        }
    }

    #[test]
    fn test_relation_type_display() {
        assert_eq!(RelationType::WorksIn.to_string(), "worksIn");
        assert_eq!(RelationType::RequiresDuration.as_str(), "requiresDuration");
    }

    #[test]
    fn test_relation_type_from_str() {
        assert_eq!(
            RelationType::from_str("worksIn"),
            Some(RelationType::WorksIn)
        );
        assert_eq!(RelationType::from_str("unknown"), None);
    }

    #[test]
    fn test_rule_based_re_leave_duration() {
        let re = RuleBasedRe::new();

        // Korean characters use 3 bytes each in UTF-8
        // "연차휴가" = 4 chars * 3 bytes = 12 bytes (0..12)
        // "는 최대 " = 4 chars (3+1+3+3+1) = 11 bytes (12..23)
        // "15일" = 4 bytes (2+1+3) (23..27)
        let text = "연차휴가는 최대 15일까지 사용할 수 있습니다.";
        let annual_leave_end = "연차휴가".len(); // 12 bytes
        let days_start = text.find("15일").unwrap();
        let days_end = days_start + "15일".len();

        let entities = vec![
            create_entity("연차휴가", "AnnualLeave", 0, annual_leave_end),
            create_entity("15일", "Days", days_start, days_end),
        ];

        let relations = re.extract(text, &entities).unwrap();

        assert!(!relations.is_empty());
        assert_eq!(relations[0].predicate, "requiresDuration");
    }

    #[test]
    fn test_rule_based_re_sick_leave_document() {
        let re = RuleBasedRe::new();

        // Use text that contains the keyword "필요" BETWEEN the entities
        let text = "병가 신청에는 진단서가 필요합니다.";
        let sick_leave_end = "병가".len(); // 6 bytes
        let doc_start = text.find("진단서").unwrap();
        let doc_end = doc_start + "진단서".len();

        let entities = vec![
            create_entity("병가", "SickLeave", 0, sick_leave_end),
            create_entity("진단서", "Document", doc_start, doc_end),
        ];

        let relations = re.extract(text, &entities).unwrap();

        assert!(!relations.is_empty());
        assert_eq!(relations[0].predicate, "requiresDocument");
    }

    #[test]
    fn test_hybrid_re_merge() {
        let re = HybridRe::new();

        let entity1 = create_entity("연차휴가", "AnnualLeave", 0, 8);
        let entity2 = create_entity("15일", "Days", 13, 16);

        let rule_relations = vec![ExtractedRelation {
            subject: entity1.clone(),
            predicate: "requiresDuration".to_string(),
            object: entity2.clone(),
            confidence: 0.85,
        }];

        let llm_relations = vec![ExtractedRelation {
            subject: entity1,
            predicate: "requiresDuration".to_string(),
            object: entity2,
            confidence: 0.90,
        }];

        let merged = re.merge_relations(rule_relations, llm_relations);
        assert_eq!(merged.len(), 1);
        // Confidence should be boosted
        assert!(merged[0].confidence > 0.5);
    }

    #[test]
    fn test_llm_re_prompt() {
        let re = LlmRe::new();
        let text = "연차휴가는 최대 15일";
        let entities = vec![create_entity("연차휴가", "AnnualLeave", 0, 8)];

        let prompt = re.build_prompt(text, &entities);
        assert!(prompt.contains("Relation types to extract"));
        assert!(prompt.contains("연차휴가"));
    }
}
