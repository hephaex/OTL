//! Named Entity Recognition (NER) module
//!
//! Provides multiple NER strategies:
//! - Rule-based: regex patterns + dictionary matching
//! - LLM-based: prompt engineering with language models
//! - Hybrid: combination of rule + LLM with confidence scoring

use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{EntityExtractor, ExtractedEntity};
use otl_core::Result;

// ============================================================================
// Entity Types for HR Domain
// ============================================================================

/// Entity types recognized by the NER system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    // Person types
    Employee,
    Manager,
    HrStaff,

    // Organization
    Department,
    Position,
    Grade,

    // Leave types
    LeaveType,
    AnnualLeave,
    SickLeave,
    ParentalLeave,
    CongratulatoryLeave,

    // Process
    ApprovalProcess,
    ApprovalStep,

    // Documents
    Regulation,
    Form,
    Document,

    // Time/Duration
    Duration,
    Date,
    Days,

    // Monetary
    Amount,
    Expense,

    // Generic
    Organization,
    Person,
    Unknown,
}

impl EntityType {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Employee => "Employee",
            Self::Manager => "Manager",
            Self::HrStaff => "HRStaff",
            Self::Department => "Department",
            Self::Position => "Position",
            Self::Grade => "Grade",
            Self::LeaveType => "LeaveType",
            Self::AnnualLeave => "AnnualLeave",
            Self::SickLeave => "SickLeave",
            Self::ParentalLeave => "ParentalLeave",
            Self::CongratulatoryLeave => "CongratulatoryLeave",
            Self::ApprovalProcess => "ApprovalProcess",
            Self::ApprovalStep => "ApprovalStep",
            Self::Regulation => "Regulation",
            Self::Form => "Form",
            Self::Document => "Document",
            Self::Duration => "Duration",
            Self::Date => "Date",
            Self::Days => "Days",
            Self::Amount => "Amount",
            Self::Expense => "Expense",
            Self::Organization => "Organization",
            Self::Person => "Person",
            Self::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Rule-based NER
// ============================================================================

/// Dictionary entry for entity matching
#[derive(Debug, Clone)]
pub struct DictionaryEntry {
    pub term: String,
    pub entity_type: EntityType,
    pub aliases: Vec<String>,
}

/// Rule-based NER using regex patterns and dictionaries
pub struct RuleBasedNer {
    /// Pattern rules (regex -> entity type)
    patterns: Vec<(Regex, EntityType, f32)>,
    /// Dictionary of known terms
    dictionary: HashMap<String, DictionaryEntry>,
    /// Lookup index (lowercase term -> entry key)
    lookup: HashMap<String, String>,
}

impl RuleBasedNer {
    /// Create a new rule-based NER with default HR domain rules
    pub fn new() -> Self {
        let mut ner = Self {
            patterns: Vec::new(),
            dictionary: HashMap::new(),
            lookup: HashMap::new(),
        };

        ner.init_hr_patterns();
        ner.init_hr_dictionary();
        ner
    }

    /// Initialize regex patterns for HR domain
    fn init_hr_patterns(&mut self) {
        // Duration patterns (Korean)
        self.add_pattern(r"\d+일", EntityType::Days, 0.9);
        self.add_pattern(r"\d+개월", EntityType::Duration, 0.9);
        self.add_pattern(r"\d+년", EntityType::Duration, 0.9);
        self.add_pattern(r"\d+주", EntityType::Duration, 0.9);

        // Duration patterns (English)
        self.add_pattern(r"\d+\s*days?", EntityType::Days, 0.9);
        self.add_pattern(r"\d+\s*months?", EntityType::Duration, 0.9);
        self.add_pattern(r"\d+\s*years?", EntityType::Duration, 0.9);
        self.add_pattern(r"\d+\s*weeks?", EntityType::Duration, 0.9);

        // Date patterns
        self.add_pattern(r"\d{4}[-/]\d{1,2}[-/]\d{1,2}", EntityType::Date, 0.95);
        self.add_pattern(r"\d{1,2}[-/]\d{1,2}[-/]\d{4}", EntityType::Date, 0.95);

        // Amount patterns (Korean Won)
        self.add_pattern(r"\d{1,3}(,\d{3})*원", EntityType::Amount, 0.9);
        self.add_pattern(r"\d+만원", EntityType::Amount, 0.9);

        // Regulation reference patterns
        self.add_pattern(r"제\d+조", EntityType::Regulation, 0.85);
        self.add_pattern(r"제\d+항", EntityType::Regulation, 0.85);

        // Form patterns
        self.add_pattern(r"\S+신청서", EntityType::Form, 0.8);
        self.add_pattern(r"\S+양식", EntityType::Form, 0.8);

        // Grade patterns
        self.add_pattern(r"[1-9]급", EntityType::Grade, 0.85);
        self.add_pattern(
            r"(사원|대리|과장|차장|부장|이사|상무|전무|부사장|사장)",
            EntityType::Position,
            0.9,
        );
    }

    /// Initialize dictionary for HR domain terms
    fn init_hr_dictionary(&mut self) {
        // Leave types
        self.add_term(
            "연차",
            EntityType::AnnualLeave,
            vec!["연차휴가", "연가", "유급휴가"],
        );
        self.add_term("병가", EntityType::SickLeave, vec!["병가휴가", "질병휴가"]);
        self.add_term(
            "육아휴직",
            EntityType::ParentalLeave,
            vec!["육아휴가", "출산휴가", "육휴"],
        );
        self.add_term(
            "경조휴가",
            EntityType::CongratulatoryLeave,
            vec!["경조사휴가", "경조"],
        );
        self.add_term("특별휴가", EntityType::LeaveType, vec!["특휴"]);
        self.add_term("공가", EntityType::LeaveType, vec!["공무휴가"]);
        self.add_term("대체휴가", EntityType::LeaveType, vec!["대휴"]);

        // Approval related
        self.add_term("승인", EntityType::ApprovalProcess, vec!["결재", "허가"]);
        self.add_term(
            "팀장승인",
            EntityType::ApprovalStep,
            vec!["팀장결재", "팀장허가"],
        );
        self.add_term("부서장승인", EntityType::ApprovalStep, vec!["부서장결재"]);
        self.add_term(
            "인사팀승인",
            EntityType::ApprovalStep,
            vec!["인사팀결재", "HR승인"],
        );

        // Position/Role
        self.add_term("팀장", EntityType::Manager, vec!["TL", "팀리더"]);
        self.add_term("부서장", EntityType::Manager, vec!["본부장"]);
        self.add_term(
            "인사담당자",
            EntityType::HrStaff,
            vec!["HR담당", "인사담당"],
        );

        // Documents
        self.add_term("취업규칙", EntityType::Regulation, vec!["근로규칙"]);
        self.add_term("인사규정", EntityType::Regulation, vec!["인사규칙"]);
        self.add_term(
            "급여규정",
            EntityType::Regulation,
            vec!["임금규정", "보수규정"],
        );
        self.add_term("복리후생규정", EntityType::Regulation, vec!["복지규정"]);
        self.add_term(
            "진단서",
            EntityType::Document,
            vec!["의사소견서", "진료확인서"],
        );

        // Expense types
        self.add_term("출장비", EntityType::Expense, vec!["출장경비", "여비"]);
        self.add_term("교통비", EntityType::Expense, vec!["차비"]);
        self.add_term("식대", EntityType::Expense, vec!["식비", "중식비"]);

        // Departments
        self.add_term("인사팀", EntityType::Department, vec!["인사부", "HR팀"]);
        self.add_term("총무팀", EntityType::Department, vec!["총무부", "관리팀"]);
        self.add_term("재무팀", EntityType::Department, vec!["재무부", "경리팀"]);
    }

    /// Add a regex pattern
    fn add_pattern(&mut self, pattern: &str, entity_type: EntityType, confidence: f32) {
        if let Ok(regex) = Regex::new(pattern) {
            self.patterns.push((regex, entity_type, confidence));
        }
    }

    /// Add a dictionary term
    fn add_term(&mut self, term: &str, entity_type: EntityType, aliases: Vec<&str>) {
        let entry = DictionaryEntry {
            term: term.to_string(),
            entity_type,
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
        };

        // Add main term to lookup
        self.lookup.insert(term.to_lowercase(), term.to_string());

        // Add aliases to lookup
        for alias in &entry.aliases {
            self.lookup.insert(alias.to_lowercase(), term.to_string());
        }

        self.dictionary.insert(term.to_string(), entry);
    }

    /// Extract entities using pattern matching
    fn extract_by_patterns(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();

        for (regex, entity_type, confidence) in &self.patterns {
            for mat in regex.find_iter(text) {
                entities.push(ExtractedEntity {
                    text: mat.as_str().to_string(),
                    entity_type: entity_type.to_string(),
                    start: mat.start(),
                    end: mat.end(),
                    confidence: *confidence,
                });
            }
        }

        entities
    }

    /// Extract entities using dictionary lookup
    fn extract_by_dictionary(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();
        let text_lower = text.to_lowercase();

        // Check each dictionary entry
        for (key, entry) in &self.dictionary {
            // Check main term
            for (start, _) in text_lower.match_indices(&key.to_lowercase()) {
                entities.push(ExtractedEntity {
                    text: text[start..start + key.len()].to_string(),
                    entity_type: entry.entity_type.to_string(),
                    start,
                    end: start + key.len(),
                    confidence: 0.95,
                });
            }

            // Check aliases
            for alias in &entry.aliases {
                let alias_lower = alias.to_lowercase();
                for (start, _) in text_lower.match_indices(&alias_lower) {
                    entities.push(ExtractedEntity {
                        text: text[start..start + alias.len()].to_string(),
                        entity_type: entry.entity_type.to_string(),
                        start,
                        end: start + alias.len(),
                        confidence: 0.9,
                    });
                }
            }
        }

        entities
    }

    /// Remove duplicate/overlapping entities, keeping highest confidence
    fn deduplicate(&self, mut entities: Vec<ExtractedEntity>) -> Vec<ExtractedEntity> {
        // Sort by start position, then by confidence (descending)
        entities.sort_by(|a, b| {
            a.start
                .cmp(&b.start)
                .then(b.confidence.partial_cmp(&a.confidence).unwrap())
        });

        let mut result = Vec::new();
        let mut covered: HashSet<usize> = HashSet::new();

        for entity in entities {
            // Check if this entity overlaps with already selected ones
            let overlaps = (entity.start..entity.end).any(|i| covered.contains(&i));

            if !overlaps {
                // Mark positions as covered
                for i in entity.start..entity.end {
                    covered.insert(i);
                }
                result.push(entity);
            }
        }

        // Sort by position
        result.sort_by_key(|e| e.start);
        result
    }
}

impl Default for RuleBasedNer {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityExtractor for RuleBasedNer {
    fn extract(&self, text: &str) -> Result<Vec<ExtractedEntity>> {
        let mut entities = Vec::new();

        // Extract by patterns
        entities.extend(self.extract_by_patterns(text));

        // Extract by dictionary
        entities.extend(self.extract_by_dictionary(text));

        // Deduplicate
        let entities = self.deduplicate(entities);

        Ok(entities)
    }
}

// ============================================================================
// LLM-based NER (placeholder for async implementation)
// ============================================================================

/// Configuration for LLM-based NER
#[derive(Debug, Clone)]
pub struct LlmNerConfig {
    /// System prompt for entity extraction
    pub system_prompt: String,
    /// Expected entity types
    pub entity_types: Vec<EntityType>,
    /// Maximum tokens in response
    pub max_tokens: usize,
    /// Temperature for generation
    pub temperature: f32,
}

impl Default for LlmNerConfig {
    fn default() -> Self {
        Self {
            system_prompt: include_str!("prompts/ner_system.txt").to_string(),
            entity_types: vec![
                EntityType::Employee,
                EntityType::Department,
                EntityType::LeaveType,
                EntityType::ApprovalProcess,
                EntityType::Regulation,
                EntityType::Duration,
            ],
            max_tokens: 1024,
            temperature: 0.1,
        }
    }
}

/// LLM-based NER extractor
pub struct LlmNer {
    pub config: LlmNerConfig,
}

impl LlmNer {
    /// Create a new LLM NER with default config
    pub fn new() -> Self {
        Self {
            config: LlmNerConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: LlmNerConfig) -> Self {
        Self { config }
    }

    /// Build the extraction prompt
    pub fn build_prompt(&self, text: &str) -> String {
        let entity_types: Vec<&str> = self
            .config
            .entity_types
            .iter()
            .map(|e| e.as_str())
            .collect();

        format!(
            "{}\n\nEntity types to extract: {}\n\nText:\n{}\n\nExtract entities in JSON format:",
            self.config.system_prompt,
            entity_types.join(", "),
            text
        )
    }

    /// Parse LLM response into entities
    pub fn parse_response(&self, response: &str, original_text: &str) -> Vec<ExtractedEntity> {
        // Try to parse JSON from response
        let entities: Vec<LlmEntity> = serde_json::from_str(response).unwrap_or_default();

        entities
            .into_iter()
            .filter_map(|e| {
                // Find the entity in original text
                if let Some(start) = original_text.find(&e.text) {
                    let len = e.text.len();
                    Some(ExtractedEntity {
                        text: e.text,
                        entity_type: e.entity_type,
                        start,
                        end: start + len,
                        confidence: e.confidence.unwrap_or(0.8),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for LlmNer {
    fn default() -> Self {
        Self::new()
    }
}

/// Entity structure for LLM JSON output
#[derive(Debug, Deserialize)]
struct LlmEntity {
    text: String,
    entity_type: String,
    confidence: Option<f32>,
}

// ============================================================================
// Hybrid NER
// ============================================================================

/// Hybrid NER combining rule-based and LLM approaches
pub struct HybridNer {
    rule_ner: RuleBasedNer,
    llm_ner: Option<LlmNer>,
    /// Weight for rule-based results (0.0 - 1.0)
    rule_weight: f32,
    /// Confidence threshold for accepting entities
    confidence_threshold: f32,
}

impl HybridNer {
    /// Create a new hybrid NER (rule-based only)
    pub fn new() -> Self {
        Self {
            rule_ner: RuleBasedNer::new(),
            llm_ner: None,
            rule_weight: 0.6,
            confidence_threshold: 0.5,
        }
    }

    /// Enable LLM extraction
    pub fn with_llm(mut self, llm_ner: LlmNer) -> Self {
        self.llm_ner = Some(llm_ner);
        self
    }

    /// Set rule weight
    pub fn with_rule_weight(mut self, weight: f32) -> Self {
        self.rule_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Set confidence threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Merge entities from multiple sources
    pub fn merge_entities(
        &self,
        rule_entities: Vec<ExtractedEntity>,
        llm_entities: Vec<ExtractedEntity>,
    ) -> Vec<ExtractedEntity> {
        let mut merged: HashMap<(usize, usize), ExtractedEntity> = HashMap::new();

        // Add rule-based entities with weight adjustment
        for mut entity in rule_entities {
            entity.confidence *= self.rule_weight;
            let key = (entity.start, entity.end);

            if let Some(existing) = merged.get_mut(&key) {
                // If same span, boost confidence
                if existing.entity_type == entity.entity_type {
                    existing.confidence = (existing.confidence + entity.confidence).min(1.0);
                } else if entity.confidence > existing.confidence {
                    *existing = entity;
                }
            } else {
                merged.insert(key, entity);
            }
        }

        // Add LLM entities with weight adjustment
        let llm_weight = 1.0 - self.rule_weight;
        for mut entity in llm_entities {
            entity.confidence *= llm_weight;
            let key = (entity.start, entity.end);

            if let Some(existing) = merged.get_mut(&key) {
                if existing.entity_type == entity.entity_type {
                    existing.confidence = (existing.confidence + entity.confidence).min(1.0);
                } else if entity.confidence > existing.confidence {
                    *existing = entity;
                }
            } else {
                merged.insert(key, entity);
            }
        }

        // Filter by confidence threshold and sort
        let mut result: Vec<ExtractedEntity> = merged
            .into_values()
            .filter(|e| e.confidence >= self.confidence_threshold)
            .collect();

        result.sort_by_key(|e| e.start);
        result
    }
}

impl Default for HybridNer {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityExtractor for HybridNer {
    fn extract(&self, text: &str) -> Result<Vec<ExtractedEntity>> {
        // Always run rule-based extraction
        let rule_entities = self.rule_ner.extract(text)?;

        // If LLM is not configured, return rule-based results
        if self.llm_ner.is_none() {
            return Ok(rule_entities);
        }

        // For now, return rule-based only
        // LLM integration requires async runtime which is handled separately
        Ok(rule_entities)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_based_ner_patterns() {
        let ner = RuleBasedNer::new();

        let text = "연차휴가는 15일이 기본 부여됩니다.";
        let entities = ner.extract(text).unwrap();

        assert!(!entities.is_empty());

        // Should find "연차휴가" and "15일"
        let entity_texts: Vec<&str> = entities.iter().map(|e| e.text.as_str()).collect();
        assert!(entity_texts.contains(&"15일"));
    }

    #[test]
    fn test_rule_based_ner_dictionary() {
        let ner = RuleBasedNer::new();

        let text = "병가 신청시 진단서가 필요합니다.";
        let entities = ner.extract(text).unwrap();

        let types: Vec<&str> = entities.iter().map(|e| e.entity_type.as_str()).collect();
        assert!(types.contains(&"SickLeave") || types.contains(&"Document"));
    }

    #[test]
    fn test_entity_type_display() {
        assert_eq!(EntityType::AnnualLeave.to_string(), "AnnualLeave");
        assert_eq!(EntityType::Manager.as_str(), "Manager");
    }

    #[test]
    fn test_hybrid_ner_merge() {
        let ner = HybridNer::new();

        let rule_entities = vec![ExtractedEntity {
            text: "연차".to_string(),
            entity_type: "AnnualLeave".to_string(),
            start: 0,
            end: 2,
            confidence: 0.9,
        }];

        let llm_entities = vec![ExtractedEntity {
            text: "연차".to_string(),
            entity_type: "AnnualLeave".to_string(),
            start: 0,
            end: 2,
            confidence: 0.85,
        }];

        let merged = ner.merge_entities(rule_entities, llm_entities);
        assert_eq!(merged.len(), 1);
        // Confidence should be boosted
        assert!(merged[0].confidence > 0.5);
    }

    #[test]
    fn test_llm_ner_prompt() {
        let ner = LlmNer::new();
        let prompt = ner.build_prompt("연차휴가 신청");

        assert!(prompt.contains("Entity types to extract"));
        assert!(prompt.contains("연차휴가 신청"));
    }

    #[test]
    fn test_korean_leave_extraction() {
        let ner = RuleBasedNer::new();

        let text = "육아휴직은 최대 2년까지 사용 가능합니다.";
        let entities = ner.extract(text).unwrap();

        let found_parental = entities.iter().any(|e| e.entity_type == "ParentalLeave");
        let found_duration = entities.iter().any(|e| e.entity_type == "Duration");

        assert!(found_parental, "Should find ParentalLeave entity");
        assert!(found_duration, "Should find Duration entity");
    }
}
