//! Embedding client for generating vector representations
//!
//! Supports OpenAI and Ollama embedding APIs.
//!
//! Author: hephaex@gmail.com

use async_trait::async_trait;
use otl_core::{LlmConfig, LlmProvider, OtlError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ============================================================================
// Embedding Trait
// ============================================================================

/// Trait for embedding generation
#[async_trait]
pub trait EmbeddingClient: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimension
    fn dimension(&self) -> usize;
}

// ============================================================================
// OpenAI Embedding Client
// ============================================================================

/// OpenAI embedding API client
pub struct OpenAiEmbedding {
    client: Client,
    api_key: String,
    model: String,
    dimension: usize,
}

#[derive(Debug, Serialize)]
struct OpenAiEmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

impl OpenAiEmbedding {
    /// Create a new OpenAI embedding client
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        let model = model.into();
        let dimension = match model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536, // Default
        };

        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model,
            dimension,
        }
    }

    /// Create from config
    pub fn from_config(config: &LlmConfig) -> Result<Self> {
        let api_key = config
            .openai_api_key
            .as_ref()
            .ok_or_else(|| OtlError::ConfigError("OpenAI API key required".to_string()))?;

        Ok(Self::new(api_key.clone(), config.embedding_model.clone()))
    }
}

#[async_trait]
impl EmbeddingClient for OpenAiEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| OtlError::LlmError("No embedding returned".to_string()))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let request = OpenAiEmbeddingRequest {
            input: texts.to_vec(),
            model: self.model.clone(),
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| OtlError::LlmError(format!("Embedding request failed: {e}")))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OtlError::LlmError(format!(
                "OpenAI embedding error: {error_text}"
            )));
        }

        let result: OpenAiEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| OtlError::LlmError(format!("Failed to parse embedding response: {e}")))?;

        // Sort by index and extract embeddings
        let mut embeddings: Vec<_> = result.data.into_iter().collect();
        embeddings.sort_by_key(|e| e.index);

        Ok(embeddings.into_iter().map(|e| e.embedding).collect())
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

// ============================================================================
// Ollama Embedding Client
// ============================================================================

/// Ollama embedding API client
pub struct OllamaEmbedding {
    client: Client,
    base_url: String,
    model: String,
    dimension: usize,
}

#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

impl OllamaEmbedding {
    /// Create a new Ollama embedding client
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        let model = model.into();
        let dimension = match model.as_str() {
            "nomic-embed-text" => 768,
            "mxbai-embed-large" => 1024,
            "all-minilm" => 384,
            _ => 768, // Default for most models
        };

        Self {
            client: Client::new(),
            base_url: base_url.into(),
            model,
            dimension,
        }
    }

    /// Create from config
    pub fn from_config(config: &LlmConfig) -> Self {
        Self::new(config.ollama_url.clone(), config.embedding_model.clone())
    }
}

#[async_trait]
impl EmbeddingClient for OllamaEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = OllamaEmbeddingRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| OtlError::LlmError(format!("Ollama embedding request failed: {e}")))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OtlError::LlmError(format!(
                "Ollama embedding error: {error_text}"
            )));
        }

        let result: OllamaEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| OtlError::LlmError(format!("Failed to parse embedding response: {e}")))?;

        Ok(result.embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Ollama doesn't have native batch embedding, so we process sequentially
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

// ============================================================================
// Factory function
// ============================================================================

/// Create an embedding client from config
pub fn create_embedding_client(config: &LlmConfig) -> Result<Box<dyn EmbeddingClient>> {
    match config.provider {
        LlmProvider::OpenAI | LlmProvider::Azure => {
            Ok(Box::new(OpenAiEmbedding::from_config(config)?))
        }
        LlmProvider::Ollama => Ok(Box::new(OllamaEmbedding::from_config(config))),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_dimension() {
        let client = OpenAiEmbedding::new("test-key", "text-embedding-3-small");
        assert_eq!(client.dimension(), 1536);

        let client = OpenAiEmbedding::new("test-key", "text-embedding-3-large");
        assert_eq!(client.dimension(), 3072);
    }

    #[test]
    fn test_ollama_dimension() {
        let client = OllamaEmbedding::new("http://localhost:11434", "nomic-embed-text");
        assert_eq!(client.dimension(), 768);

        let client = OllamaEmbedding::new("http://localhost:11434", "mxbai-embed-large");
        assert_eq!(client.dimension(), 1024);
    }
}
