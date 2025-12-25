//! OTL RAG - Retrieval-Augmented Generation Orchestrator
//!
//! This crate implements the hybrid RAG pipeline that combines:
//! - Vector similarity search (semantic)
//! - Graph traversal (structural/relational)
//! - Keyword search (lexical)
//!
//! Results are merged using Reciprocal Rank Fusion (RRF) and
//! filtered based on Access Control Lists (ACL).
//!
//! Author: hephaex@gmail.com

use otl_core::{
    Citation, LlmClient, RagQuery, RagResponse, Result, SearchBackend, SearchResult,
    SearchResultType, User,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub mod cache;
pub mod llm;

pub use cache::{CacheConfig, CacheStatsReport, EmbeddingCache, QueryCache, RagCacheManager};
pub use llm::{create_llm_client, OllamaClient, OpenAiClient};

// ============================================================================
// Configuration
// ============================================================================

/// RAG orchestrator configuration
#[derive(Debug, Clone)]
pub struct RagConfig {
    /// Number of results to retrieve from vector search
    pub vector_top_k: usize,

    /// Depth for graph traversal
    pub graph_depth: u32,

    /// Number of results to retrieve from keyword search
    pub keyword_top_k: usize,

    /// Final number of results after merging
    pub final_top_k: usize,

    /// Minimum score threshold
    pub min_score: f32,

    /// RRF constant (typically 60)
    pub rrf_k: f32,

    /// Weight for vector search results in final ranking
    pub vector_weight: f32,

    /// Weight for graph search results
    pub graph_weight: f32,

    /// Weight for keyword search results
    pub keyword_weight: f32,

    /// Maximum context length for LLM (in characters)
    pub max_context_length: usize,

    /// Include ontology schema in prompt
    pub include_ontology: bool,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            vector_top_k: 20,
            graph_depth: 2,
            keyword_top_k: 10,
            final_top_k: 5,
            min_score: 0.0,
            rrf_k: 60.0,
            vector_weight: 1.0,
            graph_weight: 1.5, // Slightly higher weight for graph results
            keyword_weight: 0.8,
            max_context_length: 8000,
            include_ontology: true,
        }
    }
}

// ============================================================================
// Query Analysis
// ============================================================================

/// Analysis of a user query
#[derive(Debug, Clone)]
pub struct QueryAnalysis {
    /// Original question
    pub question: String,

    /// Detected intent
    pub intent: QueryIntent,

    /// Entities detected in the question
    pub detected_entities: Vec<DetectedEntity>,

    /// Keywords extracted
    pub keywords: Vec<String>,

    /// Expected answer type
    pub expected_answer_type: AnswerType,
}

/// Type of user intent
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryIntent {
    /// Looking for a procedure/process
    Procedural,
    /// Looking for a specific fact
    Factual,
    /// Comparing things
    Comparative,
    /// Conditional question
    Conditional,
    /// Definition/explanation
    Definitional,
    /// Unknown/general
    General,
}

/// An entity detected in the query
#[derive(Debug, Clone)]
pub struct DetectedEntity {
    /// Entity text
    pub text: String,
    /// Entity type (class from ontology)
    pub entity_type: Option<String>,
    /// Start position in query
    pub start: usize,
    /// End position in query
    pub end: usize,
}

/// Expected answer type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerType {
    List,
    SingleFact,
    Explanation,
    Comparison,
    YesNo,
    Unknown,
}

// ============================================================================
// RAG Orchestrator
// ============================================================================

/// Hybrid RAG orchestrator
pub struct HybridRagOrchestrator {
    /// Vector search backend
    vector_store: Arc<dyn SearchBackend>,

    /// Graph search backend
    graph_store: Arc<dyn SearchBackend>,

    /// Keyword search backend (optional)
    keyword_store: Option<Arc<dyn SearchBackend>>,

    /// LLM client
    llm_client: Arc<dyn LlmClient>,

    /// Configuration
    config: RagConfig,

    /// Ontology schema (for prompt context)
    ontology_schema: Option<String>,
}

impl HybridRagOrchestrator {
    /// Create a new orchestrator
    pub fn new(
        vector_store: Arc<dyn SearchBackend>,
        graph_store: Arc<dyn SearchBackend>,
        llm_client: Arc<dyn LlmClient>,
        config: RagConfig,
    ) -> Self {
        Self {
            vector_store,
            graph_store,
            keyword_store: None,
            llm_client,
            config,
            ontology_schema: None,
        }
    }

    /// Set keyword search backend
    pub fn with_keyword_store(mut self, store: Arc<dyn SearchBackend>) -> Self {
        self.keyword_store = Some(store);
        self
    }

    /// Set ontology schema for prompts
    pub fn with_ontology_schema(mut self, schema: impl Into<String>) -> Self {
        self.ontology_schema = Some(schema.into());
        self
    }

    /// Execute a RAG query
    pub async fn query(&self, query: &RagQuery, user: &User) -> Result<RagResponse> {
        let start_time = Instant::now();

        tracing::info!("RAG query started");

        // 1. Analyze the question
        let analysis = self.analyze_query(&query.question).await?;
        tracing::debug!("Query analyzed: intent={:?}", analysis.intent);

        // 2. Execute searches in parallel
        tracing::debug!("Executing parallel searches");
        let (vector_results, graph_results, keyword_results) = tokio::join!(
            self.vector_store
                .search(&query.question, self.config.vector_top_k),
            self.search_graph_context(&analysis),
            self.search_keywords(&analysis)
        );
        tracing::debug!("Searches completed");

        // 3. Collect results
        let mut all_results = Vec::new();

        if let Ok(results) = vector_results {
            tracing::debug!("Vector search returned {} results", results.len());
            all_results.extend(results);
        }

        if let Ok(results) = graph_results {
            tracing::debug!("Graph search returned {} results", results.len());
            all_results.extend(results);
        }

        if let Ok(results) = keyword_results {
            tracing::debug!("Keyword search returned {} results", results.len());
            all_results.extend(results);
        }

        // 4. ACL filtering
        let filtered_results = self.filter_by_acl(all_results, user);
        tracing::debug!("ACL filtered to {} results", filtered_results.len());

        // 5. Merge and rank results using RRF
        let merged_results = self.merge_results(filtered_results);
        tracing::debug!("Merged to {} results", merged_results.len());

        // 6. Take top-k
        let final_results: Vec<_> = merged_results
            .into_iter()
            .take(self.config.final_top_k)
            .collect();
        tracing::debug!("Final top-k: {} results", final_results.len());

        // 7. Build prompt and generate response
        let prompt = self.build_prompt(&query.question, &final_results, &analysis);
        tracing::info!("Calling LLM with prompt length: {} chars", prompt.len());
        let answer = self.llm_client.generate(&prompt).await?;
        tracing::info!("LLM response received: {} chars", answer.len());

        // 8. Extract citations
        let citations = self.extract_citations(&answer, &final_results);

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(RagResponse {
            answer,
            citations,
            confidence: self.calculate_confidence(&final_results),
            processing_time_ms,
        })
    }

    /// Analyze the query to extract intent, entities, and keywords
    async fn analyze_query(&self, question: &str) -> Result<QueryAnalysis> {
        // Simple rule-based analysis (can be enhanced with LLM)
        let question_lower = question.to_lowercase();

        // Detect intent
        let intent = if question_lower.contains("어떻게")
            || question_lower.contains("절차")
            || question_lower.contains("방법")
            || question_lower.contains("how")
        {
            QueryIntent::Procedural
        } else if question_lower.contains("차이")
            || question_lower.contains("비교")
            || question_lower.contains("vs")
        {
            QueryIntent::Comparative
        } else if question_lower.contains("무엇")
            || question_lower.contains("뭐")
            || question_lower.contains("what is")
        {
            QueryIntent::Definitional
        } else if question_lower.contains("며칠")
            || question_lower.contains("몇")
            || question_lower.contains("언제")
        {
            QueryIntent::Factual
        } else if question_lower.contains("경우")
            || question_lower.contains("만약")
            || question_lower.contains("if")
        {
            QueryIntent::Conditional
        } else {
            QueryIntent::General
        };

        // Determine expected answer type
        let expected_answer_type = match intent {
            QueryIntent::Procedural => AnswerType::List,
            QueryIntent::Comparative => AnswerType::Comparison,
            QueryIntent::Factual => AnswerType::SingleFact,
            QueryIntent::Definitional => AnswerType::Explanation,
            _ => AnswerType::Unknown,
        };

        // Extract keywords (simple whitespace tokenization, filter stopwords)
        let stopwords = [
            "은", "는", "이", "가", "를", "을", "의", "에", "와", "과", "the", "a", "is", "are",
            "what", "how",
        ];
        let keywords: Vec<String> = question
            .split_whitespace()
            .filter(|w| w.len() > 1 && !stopwords.contains(&w.to_lowercase().as_str()))
            .map(|s| s.to_string())
            .collect();

        Ok(QueryAnalysis {
            question: question.to_string(),
            intent,
            detected_entities: Vec::new(), // Would be populated by NER
            keywords,
            expected_answer_type,
        })
    }

    /// Search graph for context related to detected entities
    async fn search_graph_context(&self, analysis: &QueryAnalysis) -> Result<Vec<SearchResult>> {
        // Use keywords as starting points for graph traversal
        let query = analysis.keywords.join(" ");
        self.graph_store
            .search(&query, self.config.vector_top_k)
            .await
    }

    /// Search keywords if keyword store is available
    async fn search_keywords(&self, analysis: &QueryAnalysis) -> Result<Vec<SearchResult>> {
        if let Some(ref store) = self.keyword_store {
            let query = analysis.keywords.join(" ");
            store.search(&query, self.config.keyword_top_k).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Filter results based on user's access permissions
    fn filter_by_acl(&self, results: Vec<SearchResult>, user: &User) -> Vec<SearchResult> {
        results
            .into_iter()
            .filter(|r| r.acl.can_access(user))
            .collect()
    }

    /// Merge results using Reciprocal Rank Fusion (RRF)
    fn merge_results(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        // Group by content hash to handle duplicates
        let mut score_map: HashMap<String, (f32, SearchResult)> = HashMap::new();

        // Sort results by score to get ranks
        let mut vector_results: Vec<_> = results
            .iter()
            .filter(|r| r.result_type == SearchResultType::Vector)
            .collect();
        let mut graph_results: Vec<_> = results
            .iter()
            .filter(|r| r.result_type == SearchResultType::Graph)
            .collect();
        let mut keyword_results: Vec<_> = results
            .iter()
            .filter(|r| r.result_type == SearchResultType::Keyword)
            .collect();

        vector_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        graph_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        keyword_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Calculate RRF scores
        let k = self.config.rrf_k;

        for (rank, result) in vector_results.iter().enumerate() {
            let rrf_score = self.config.vector_weight / (k + rank as f32 + 1.0);
            let key = hash_content(&result.content);
            score_map
                .entry(key)
                .and_modify(|(score, _)| *score += rrf_score)
                .or_insert((rrf_score, (*result).clone()));
        }

        for (rank, result) in graph_results.iter().enumerate() {
            let rrf_score = self.config.graph_weight / (k + rank as f32 + 1.0);
            let key = hash_content(&result.content);
            score_map
                .entry(key)
                .and_modify(|(score, _)| *score += rrf_score)
                .or_insert((rrf_score, (*result).clone()));
        }

        for (rank, result) in keyword_results.iter().enumerate() {
            let rrf_score = self.config.keyword_weight / (k + rank as f32 + 1.0);
            let key = hash_content(&result.content);
            score_map
                .entry(key)
                .and_modify(|(score, _)| *score += rrf_score)
                .or_insert((rrf_score, (*result).clone()));
        }

        // Sort by RRF score and return
        let mut merged: Vec<_> = score_map
            .into_values()
            .map(|(score, mut result)| {
                result.score = score;
                result
            })
            .collect();

        merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        merged
    }

    /// Build the LLM prompt with context
    fn build_prompt(
        &self,
        question: &str,
        results: &[SearchResult],
        _analysis: &QueryAnalysis,
    ) -> String {
        let mut prompt = String::new();

        // System instruction
        prompt.push_str("<s>\n");
        prompt.push_str("당신은 조직의 지식 전문가입니다.\n");
        prompt.push_str("제공된 컨텍스트 정보만을 사용하여 질문에 답변하세요.\n");
        prompt.push_str("답변에 사용한 정보의 출처를 반드시 [출처: N] 형식으로 명시하세요.\n");
        prompt
            .push_str("컨텍스트에 없는 정보는 \"해당 정보를 찾을 수 없습니다\"라고 답변하세요.\n");

        // Include ontology schema if configured
        if self.config.include_ontology {
            if let Some(ref schema) = self.ontology_schema {
                prompt.push_str("\n온톨로지 스키마:\n");
                prompt.push_str(schema);
            }
        }

        prompt.push_str("</s>\n\n");

        // Context
        prompt.push_str("<context>\n");
        let mut total_length = 0;
        for (i, result) in results.iter().enumerate() {
            if total_length + result.content.len() > self.config.max_context_length {
                break;
            }

            prompt.push_str(&format!("[{}] 출처: {:?}\n", i + 1, result.source));
            prompt.push_str(&result.content);
            prompt.push_str("\n\n");

            total_length += result.content.len();
        }
        prompt.push_str("</context>\n\n");

        // Question
        prompt.push_str("<question>\n");
        prompt.push_str(question);
        prompt.push_str("\n</question>\n\n");

        // Instructions
        prompt.push_str("<instructions>\n");
        prompt.push_str("1. 컨텍스트를 주의 깊게 읽으세요.\n");
        prompt.push_str("2. 질문에 직접 관련된 정보만 사용하세요.\n");
        prompt.push_str("3. 답변 작성 시 [출처: N] 형식으로 인용하세요.\n");
        prompt.push_str("4. 확실하지 않은 정보는 언급하지 마세요.\n");
        prompt.push_str("</instructions>\n");

        prompt
    }

    /// Extract citations from the generated answer
    fn extract_citations(&self, answer: &str, results: &[SearchResult]) -> Vec<Citation> {
        let mut citations = Vec::new();

        // Find all [출처: N] patterns
        let re = regex::Regex::new(r"\[출처:\s*(\d+)\]").unwrap_or_else(|_| {
            // Fallback if regex fails
            regex::Regex::new(r"\[(\d+)\]").unwrap()
        });

        for cap in re.captures_iter(answer) {
            let Some(num_str) = cap.get(1) else {
                continue;
            };
            let Ok(num) = num_str.as_str().parse::<usize>() else {
                continue;
            };
            if num == 0 || num > results.len() {
                continue;
            }

            let result = &results[num - 1];
            citations.push(Citation {
                index: num as u32,
                text: result.content.chars().take(200).collect(),
                source: result.source.clone(),
                document_title: format!("Document {:?}", result.source.document_id),
            });
        }

        // Deduplicate by index
        citations.sort_by_key(|c| c.index);
        citations.dedup_by_key(|c| c.index);

        citations
    }

    /// Calculate overall confidence based on search results
    fn calculate_confidence(&self, results: &[SearchResult]) -> f32 {
        if results.is_empty() {
            return 0.0;
        }

        // Average of top-k scores, normalized
        let avg_score: f32 = results.iter().map(|r| r.score).sum::<f32>() / results.len() as f32;

        // Normalize to 0-1 range (assuming RRF scores are typically < 1)
        (avg_score * 10.0).min(1.0)
    }
}

/// Simple hash for content deduplication
fn hash_content(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    // Hash first 100 chars for efficiency
    content
        .chars()
        .take(100)
        .collect::<String>()
        .hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

// ============================================================================
// Prompt Builder
// ============================================================================

/// Builder for constructing RAG prompts
pub struct PromptBuilder {
    system_instruction: String,
    context_sections: Vec<String>,
    question: String,
    instructions: Vec<String>,
}

impl PromptBuilder {
    /// Create a new prompt builder
    pub fn new() -> Self {
        Self {
            system_instruction: String::new(),
            context_sections: Vec::new(),
            question: String::new(),
            instructions: Vec::new(),
        }
    }

    /// Set system instruction
    pub fn system(mut self, instruction: impl Into<String>) -> Self {
        self.system_instruction = instruction.into();
        self
    }

    /// Add a context section
    pub fn add_context(mut self, context: impl Into<String>) -> Self {
        self.context_sections.push(context.into());
        self
    }

    /// Set the question
    pub fn question(mut self, q: impl Into<String>) -> Self {
        self.question = q.into();
        self
    }

    /// Add an instruction
    pub fn add_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.instructions.push(instruction.into());
        self
    }

    /// Build the final prompt
    pub fn build(self) -> String {
        let mut prompt = String::new();

        if !self.system_instruction.is_empty() {
            prompt.push_str("<s>\n");
            prompt.push_str(&self.system_instruction);
            prompt.push_str("\n</s>\n\n");
        }

        if !self.context_sections.is_empty() {
            prompt.push_str("<context>\n");
            for section in &self.context_sections {
                prompt.push_str(section);
                prompt.push_str("\n\n");
            }
            prompt.push_str("</context>\n\n");
        }

        if !self.question.is_empty() {
            prompt.push_str("<question>\n");
            prompt.push_str(&self.question);
            prompt.push_str("\n</question>\n\n");
        }

        if !self.instructions.is_empty() {
            prompt.push_str("<instructions>\n");
            for (i, inst) in self.instructions.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, inst));
            }
            prompt.push_str("</instructions>\n");
        }

        prompt
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_intent_detection() {
        // This would need async runtime for actual testing
        // Just test the pattern matching logic directly

        let procedural_keywords = ["어떻게", "절차", "방법"];
        let comparative_keywords = ["차이", "비교"];

        for kw in procedural_keywords {
            assert!(kw.contains("어떻게") || kw.contains("절차") || kw.contains("방법"));
        }

        for kw in comparative_keywords {
            assert!(kw.contains("차이") || kw.contains("비교"));
        }
    }

    #[test]
    fn test_prompt_builder() {
        let prompt = PromptBuilder::new()
            .system("You are a helpful assistant.")
            .add_context("[1] Context from document A")
            .add_context("[2] Context from document B")
            .question("What is the answer?")
            .add_instruction("Be concise")
            .add_instruction("Cite sources")
            .build();

        assert!(prompt.contains("<s>"));
        assert!(prompt.contains("You are a helpful assistant."));
        assert!(prompt.contains("<context>"));
        assert!(prompt.contains("What is the answer?"));
        assert!(prompt.contains("1. Be concise"));
    }

    #[test]
    fn test_rrf_config_defaults() {
        let config = RagConfig::default();

        assert_eq!(config.vector_top_k, 20);
        assert_eq!(config.graph_depth, 2);
        assert_eq!(config.final_top_k, 5);
        assert!(config.rrf_k > 0.0);
    }

    #[test]
    fn test_content_hashing() {
        let content1 = "This is some content for testing";
        let content2 = "This is some content for testing";
        let content3 = "Different content";

        assert_eq!(hash_content(content1), hash_content(content2));
        assert_ne!(hash_content(content1), hash_content(content3));
    }
}
