//! Advanced Query Analysis Module
//!
//! This module provides LLM-based semantic query analysis with support for:
//! - Multi-intent detection and decomposition
//! - Confidence scoring
//! - Query rewriting for better retrieval
//! - Query expansion with synonyms
//!
//! Author: hephaex@gmail.com

use otl_core::{LlmClient, OtlError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ============================================================================
// Core Types
// ============================================================================

/// Enhanced query analysis with multi-intent support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedQueryAnalysis {
    /// Original question
    pub original_query: String,

    /// Detected intents (may be multiple for compound queries)
    pub intents: Vec<QueryIntentWithConfidence>,

    /// Sub-queries decomposed from the original query
    pub sub_queries: Vec<SubQuery>,

    /// Entities detected in the query
    pub entities: Vec<DetectedEntity>,

    /// Keywords extracted
    pub keywords: Vec<String>,

    /// Rewritten query for better retrieval
    pub rewritten_query: String,

    /// Expanded query with synonyms
    pub expanded_queries: Vec<String>,

    /// Overall analysis confidence
    pub overall_confidence: f32,
}

/// Intent with confidence score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryIntentWithConfidence {
    /// The detected intent type
    pub intent: QueryIntent,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,

    /// Entities relevant to this intent
    pub relevant_entities: Vec<String>,
}

/// Sub-query decomposed from a compound query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubQuery {
    /// The sub-query text
    pub query: String,

    /// Intent of this sub-query
    pub intent: QueryIntent,

    /// Confidence score
    pub confidence: f32,

    /// Index in the original query
    pub index: usize,
}

/// Type of user intent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryIntent {
    /// Looking for a procedure/process (how to do something)
    Procedural,
    /// Looking for a specific fact (who, what, when, where)
    Factual,
    /// Comparing things (differences, similarities)
    Comparative,
    /// Conditional question (if-then scenarios)
    Conditional,
    /// Definition/explanation (what is something)
    Definitional,
    /// Listing items (enumerate, show all)
    Listing,
    /// Analytical/reasoning (why, implications)
    Analytical,
    /// Unknown/general
    General,
}

impl std::fmt::Display for QueryIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Procedural => write!(f, "procedural"),
            Self::Factual => write!(f, "factual"),
            Self::Comparative => write!(f, "comparative"),
            Self::Conditional => write!(f, "conditional"),
            Self::Definitional => write!(f, "definitional"),
            Self::Listing => write!(f, "listing"),
            Self::Analytical => write!(f, "analytical"),
            Self::General => write!(f, "general"),
        }
    }
}

/// An entity detected in the query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedEntity {
    /// Entity text
    pub text: String,
    /// Entity type (if known)
    pub entity_type: Option<String>,
    /// Start position in query
    pub start: usize,
    /// End position in query
    pub end: usize,
    /// Confidence score
    pub confidence: f32,
}

// ============================================================================
// Query Analyzer
// ============================================================================

/// Advanced query analyzer using LLM
pub struct QueryAnalyzer {
    /// LLM client for semantic analysis
    llm_client: Arc<dyn LlmClient>,

    /// Whether to use LLM-based analysis (fallback to rule-based if false)
    use_llm: bool,
}

impl QueryAnalyzer {
    /// Create a new query analyzer
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self {
            llm_client,
            use_llm: true,
        }
    }

    /// Create a rule-based analyzer (no LLM)
    pub fn rule_based() -> Self {
        Self {
            llm_client: Arc::new(DummyLlmClient),
            use_llm: false,
        }
    }

    /// Analyze a query with full semantic understanding
    pub async fn analyze(&self, query: &str) -> Result<EnhancedQueryAnalysis> {
        if self.use_llm {
            self.llm_based_analysis(query).await
        } else {
            self.rule_based_analysis(query).await
        }
    }

    /// LLM-based semantic analysis
    async fn llm_based_analysis(&self, query: &str) -> Result<EnhancedQueryAnalysis> {
        let prompt = self.build_analysis_prompt(query);

        tracing::debug!("Query analysis prompt: {} chars", prompt.len());

        let response = self.llm_client.generate(&prompt).await?;

        tracing::debug!("Query analysis response: {} chars", response.len());

        // Parse the LLM response
        self.parse_llm_response(query, &response).await
    }

    /// Build the prompt for query analysis
    fn build_analysis_prompt(&self, query: &str) -> String {
        format!(
            r#"<s>
You are a query analysis expert. Analyze the following user query and provide structured information.

Query: "{query}"

Provide your analysis in the following JSON format:
{{
  "intents": [
    {{
      "intent": "procedural|factual|comparative|conditional|definitional|listing|analytical|general",
      "confidence": 0.0-1.0,
      "relevant_entities": ["entity1", "entity2"]
    }}
  ],
  "sub_queries": [
    {{
      "query": "sub-query text",
      "intent": "intent_type",
      "confidence": 0.0-1.0
    }}
  ],
  "entities": [
    {{
      "text": "entity text",
      "entity_type": "type or null",
      "confidence": 0.0-1.0
    }}
  ],
  "keywords": ["keyword1", "keyword2"],
  "rewritten_query": "clearer version of the query",
  "expanded_queries": ["synonym variation 1", "synonym variation 2"]
}}

Intent types:
- procedural: how to do something (e.g., "How do I apply for vacation?")
- factual: specific facts (e.g., "Who is the HR manager?")
- comparative: comparisons (e.g., "What's the difference between X and Y?")
- conditional: if-then scenarios (e.g., "What happens if I work overtime?")
- definitional: definitions (e.g., "What is a sabbatical?")
- listing: enumerate items (e.g., "Show all benefits")
- analytical: reasoning/implications (e.g., "Why do we need approval?")
- general: other questions

For compound queries (e.g., "Compare X AND show Y"), detect multiple intents and decompose into sub-queries.

Respond with ONLY the JSON, no additional text.
</s>"#
        )
    }

    /// Parse LLM response into structured analysis
    async fn parse_llm_response(
        &self,
        original_query: &str,
        response: &str,
    ) -> Result<EnhancedQueryAnalysis> {
        // Try to extract JSON from the response
        let json_str = self.extract_json(response)?;

        #[derive(Deserialize)]
        struct LlmAnalysisResponse {
            intents: Vec<LlmIntent>,
            sub_queries: Option<Vec<LlmSubQuery>>,
            entities: Option<Vec<LlmEntity>>,
            keywords: Vec<String>,
            rewritten_query: String,
            expanded_queries: Option<Vec<String>>,
        }

        #[derive(Deserialize)]
        struct LlmIntent {
            intent: String,
            confidence: f32,
            relevant_entities: Option<Vec<String>>,
        }

        #[derive(Deserialize)]
        struct LlmSubQuery {
            query: String,
            intent: String,
            confidence: f32,
        }

        #[derive(Deserialize)]
        struct LlmEntity {
            text: String,
            entity_type: Option<String>,
            confidence: Option<f32>,
        }

        let parsed: LlmAnalysisResponse = serde_json::from_str(&json_str)
            .map_err(|e| OtlError::LlmError(format!("Failed to parse analysis JSON: {e}")))?;

        // Convert LLM response to our types
        let intents: Vec<QueryIntentWithConfidence> = parsed
            .intents
            .into_iter()
            .map(|i| QueryIntentWithConfidence {
                intent: self.parse_intent(&i.intent),
                confidence: i.confidence,
                relevant_entities: i.relevant_entities.unwrap_or_default(),
            })
            .collect();

        let sub_queries: Vec<SubQuery> = parsed
            .sub_queries
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(idx, sq)| SubQuery {
                query: sq.query,
                intent: self.parse_intent(&sq.intent),
                confidence: sq.confidence,
                index: idx,
            })
            .collect();

        let entities: Vec<DetectedEntity> = parsed
            .entities
            .unwrap_or_default()
            .into_iter()
            .map(|e| {
                // Find position in original query
                let start = original_query
                    .to_lowercase()
                    .find(&e.text.to_lowercase())
                    .unwrap_or(0);
                let end = start + e.text.len();

                DetectedEntity {
                    text: e.text,
                    entity_type: e.entity_type,
                    start,
                    end,
                    confidence: e.confidence.unwrap_or(0.8),
                }
            })
            .collect();

        // Calculate overall confidence
        let overall_confidence = if !intents.is_empty() {
            intents.iter().map(|i| i.confidence).sum::<f32>() / intents.len() as f32
        } else {
            0.5
        };

        Ok(EnhancedQueryAnalysis {
            original_query: original_query.to_string(),
            intents,
            sub_queries,
            entities,
            keywords: parsed.keywords,
            rewritten_query: parsed.rewritten_query,
            expanded_queries: parsed.expanded_queries.unwrap_or_default(),
            overall_confidence,
        })
    }

    /// Extract JSON from LLM response (may have extra text)
    fn extract_json(&self, response: &str) -> Result<String> {
        // Try to find JSON object in response
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                return Ok(response[start..=end].to_string());
            }
        }

        // If no JSON found, try the whole response
        Ok(response.trim().to_string())
    }

    /// Parse intent string to enum
    fn parse_intent(&self, intent_str: &str) -> QueryIntent {
        match intent_str.to_lowercase().as_str() {
            "procedural" => QueryIntent::Procedural,
            "factual" => QueryIntent::Factual,
            "comparative" => QueryIntent::Comparative,
            "conditional" => QueryIntent::Conditional,
            "definitional" => QueryIntent::Definitional,
            "listing" => QueryIntent::Listing,
            "analytical" => QueryIntent::Analytical,
            _ => QueryIntent::General,
        }
    }

    /// Rule-based analysis (fallback when LLM not available)
    async fn rule_based_analysis(&self, query: &str) -> Result<EnhancedQueryAnalysis> {
        let query_lower = query.to_lowercase();

        // Detect intent using keyword patterns
        let (intent, confidence) = self.detect_intent_rule_based(&query_lower);

        // Extract keywords (simple whitespace tokenization)
        let stopwords = [
            "은", "는", "이", "가", "를", "을", "의", "에", "와", "과", "the", "a", "is", "are",
            "what", "how", "and", "or", "show", "tell",
        ];
        let keywords: Vec<String> = query
            .split_whitespace()
            .filter(|w| w.len() > 1 && !stopwords.contains(&w.to_lowercase().as_str()))
            .map(|s| s.to_string())
            .collect();

        // Detect compound queries (AND, OR, comma separation)
        let sub_queries = self.decompose_compound_query(query, &intent);

        // Simple rewriting (remove question words)
        let rewritten_query = self.rewrite_query_simple(query);

        // No entity detection or query expansion in rule-based mode
        Ok(EnhancedQueryAnalysis {
            original_query: query.to_string(),
            intents: vec![QueryIntentWithConfidence {
                intent,
                confidence,
                relevant_entities: Vec::new(),
            }],
            sub_queries,
            entities: Vec::new(),
            keywords,
            rewritten_query,
            expanded_queries: Vec::new(),
            overall_confidence: confidence,
        })
    }

    /// Detect intent using keyword rules
    fn detect_intent_rule_based(&self, query_lower: &str) -> (QueryIntent, f32) {
        // Procedural patterns
        if query_lower.contains("어떻게")
            || query_lower.contains("how")
            || query_lower.contains("절차")
            || query_lower.contains("방법")
            || query_lower.contains("process")
        {
            return (QueryIntent::Procedural, 0.8);
        }

        // Comparative patterns
        if query_lower.contains("차이")
            || query_lower.contains("비교")
            || query_lower.contains("compare")
            || query_lower.contains("difference")
            || query_lower.contains("vs")
        {
            return (QueryIntent::Comparative, 0.85);
        }

        // Definitional patterns
        if query_lower.contains("무엇")
            || query_lower.contains("뭐")
            || query_lower.contains("what is")
            || query_lower.contains("define")
        {
            return (QueryIntent::Definitional, 0.75);
        }

        // Factual patterns
        if query_lower.contains("며칠")
            || query_lower.contains("몇")
            || query_lower.contains("언제")
            || query_lower.contains("when")
            || query_lower.contains("who")
            || query_lower.contains("where")
        {
            return (QueryIntent::Factual, 0.8);
        }

        // Conditional patterns
        if query_lower.contains("경우")
            || query_lower.contains("만약")
            || query_lower.contains("if")
            || query_lower.contains("when")
        {
            return (QueryIntent::Conditional, 0.7);
        }

        // Listing patterns
        if query_lower.contains("목록")
            || query_lower.contains("보여")
            || query_lower.contains("list")
            || query_lower.contains("show all")
            || query_lower.contains("enumerate")
        {
            return (QueryIntent::Listing, 0.75);
        }

        // Analytical patterns
        if query_lower.contains("왜")
            || query_lower.contains("why")
            || query_lower.contains("reason")
            || query_lower.contains("because")
        {
            return (QueryIntent::Analytical, 0.7);
        }

        // Default to general
        (QueryIntent::General, 0.5)
    }

    /// Decompose compound queries (e.g., "Compare X AND show Y")
    fn decompose_compound_query(&self, query: &str, default_intent: &QueryIntent) -> Vec<SubQuery> {
        let mut sub_queries = Vec::new();

        // Split by common conjunctions
        let conjunctions = [" AND ", " OR ", " and ", " or ", ", ", " 그리고 ", " 또는 "];

        let mut parts = vec![query.to_string()];
        for conj in &conjunctions {
            let mut new_parts = Vec::new();
            for part in parts {
                new_parts.extend(part.split(conj).map(|s| s.trim().to_string()));
            }
            parts = new_parts;
        }

        // If we found multiple parts, create sub-queries
        if parts.len() > 1 {
            for (idx, part) in parts.into_iter().enumerate() {
                if part.is_empty() {
                    continue;
                }

                let (intent, confidence) = self.detect_intent_rule_based(&part.to_lowercase());

                sub_queries.push(SubQuery {
                    query: part,
                    intent,
                    confidence,
                    index: idx,
                });
            }
        } else {
            // Single intent query
            sub_queries.push(SubQuery {
                query: query.to_string(),
                intent: default_intent.clone(),
                confidence: 0.9,
                index: 0,
            });
        }

        sub_queries
    }

    /// Simple query rewriting (remove question words)
    fn rewrite_query_simple(&self, query: &str) -> String {
        let question_words = [
            "어떻게", "무엇", "뭐", "언제", "어디", "누구", "왜", "how", "what", "when", "where",
            "who", "why", "is", "are", "the", "a",
        ];

        let words: Vec<&str> = query
            .split_whitespace()
            .filter(|w| !question_words.contains(&w.to_lowercase().as_str()))
            .collect();

        words.join(" ")
    }
}

// ============================================================================
// Dummy LLM Client (for rule-based mode)
// ============================================================================

struct DummyLlmClient;

#[async_trait::async_trait]
impl otl_core::LlmClient for DummyLlmClient {
    async fn generate(&self, _prompt: &str) -> Result<String> {
        Err(OtlError::LlmError(
            "LLM not available in rule-based mode".to_string(),
        ))
    }

    async fn generate_stream(
        &self,
        _prompt: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<String>>> {
        Err(OtlError::LlmError(
            "LLM not available in rule-based mode".to_string(),
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rule_based_intent_detection() {
        let analyzer = QueryAnalyzer::rule_based();

        // Test procedural
        let (intent, _) = analyzer.detect_intent_rule_based("how do i apply for vacation");
        assert_eq!(intent, QueryIntent::Procedural);

        // Test comparative
        let (intent, _) = analyzer.detect_intent_rule_based("what is the difference between x and y");
        assert_eq!(intent, QueryIntent::Comparative);

        // Test definitional
        let (intent, _) = analyzer.detect_intent_rule_based("what is a sabbatical");
        assert_eq!(intent, QueryIntent::Definitional);
    }

    #[tokio::test]
    async fn test_compound_query_decomposition() {
        let analyzer = QueryAnalyzer::rule_based();

        let analysis = analyzer
            .analyze("Compare vacation policies AND show approval steps")
            .await
            .unwrap();

        assert!(
            analysis.sub_queries.len() >= 2,
            "Should detect multiple sub-queries"
        );

        // Check that both intents are detected
        let has_comparative = analysis
            .sub_queries
            .iter()
            .any(|sq| sq.intent == QueryIntent::Comparative);
        let has_procedural = analysis
            .sub_queries
            .iter()
            .any(|sq| sq.intent == QueryIntent::Procedural || sq.intent == QueryIntent::Listing);

        assert!(has_comparative || has_procedural, "Should detect different intents");
    }

    #[tokio::test]
    async fn test_simple_query_rewriting() {
        let analyzer = QueryAnalyzer::rule_based();

        let rewritten = analyzer.rewrite_query_simple("What is the vacation policy?");
        assert!(!rewritten.contains("What"));
        assert!(rewritten.contains("vacation"));
        assert!(rewritten.contains("policy"));
    }

    #[tokio::test]
    async fn test_keyword_extraction() {
        let analyzer = QueryAnalyzer::rule_based();

        let analysis = analyzer
            .analyze("Show me the vacation approval process")
            .await
            .unwrap();

        assert!(analysis.keywords.contains(&"vacation".to_string()));
        assert!(analysis.keywords.contains(&"approval".to_string()));
        assert!(analysis.keywords.contains(&"process".to_string()));
    }

    #[test]
    fn test_intent_display() {
        assert_eq!(QueryIntent::Procedural.to_string(), "procedural");
        assert_eq!(QueryIntent::Comparative.to_string(), "comparative");
        assert_eq!(QueryIntent::Definitional.to_string(), "definitional");
    }

    #[test]
    fn test_json_extraction() {
        let analyzer = QueryAnalyzer::rule_based();

        let response = r#"Here is the analysis: {"intents": [{"intent": "procedural", "confidence": 0.9}]}"#;
        let json = analyzer.extract_json(response).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }
}
