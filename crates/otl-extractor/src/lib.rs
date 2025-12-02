//! OTL Extractor - Knowledge extraction pipeline
//!
//! Implements Named Entity Recognition (NER) and
//! Relation Extraction (RE) for building knowledge graphs.

use otl_core::Result;

/// Extracted entity from text
#[derive(Debug, Clone)]
pub struct ExtractedEntity {
    pub text: String,
    pub entity_type: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f32,
}

/// Extracted relation between entities
#[derive(Debug, Clone)]
pub struct ExtractedRelation {
    pub subject: ExtractedEntity,
    pub predicate: String,
    pub object: ExtractedEntity,
    pub confidence: f32,
}

/// Trait for entity extractors
pub trait EntityExtractor: Send + Sync {
    fn extract(&self, text: &str) -> Result<Vec<ExtractedEntity>>;
}

/// Trait for relation extractors
pub trait RelationExtractor: Send + Sync {
    fn extract(&self, text: &str, entities: &[ExtractedEntity]) -> Result<Vec<ExtractedRelation>>;
}

pub mod ner;
pub mod relation;
