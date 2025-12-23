//! LLM Client implementations
//!
//! Provides abstraction for OpenAI and Ollama LLM APIs
//! with support for both synchronous and streaming responses.
//!
//! Author: hephaex@gmail.com

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use otl_core::{LlmClient, LlmConfig, LlmProvider, OtlError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ============================================================================
// OpenAI Client
// ============================================================================

/// OpenAI API client
pub struct OpenAiClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Choice {
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

impl OpenAiClient {
    /// Create a new OpenAI client
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        max_tokens: u32,
        temperature: f32,
    ) -> Self {
        // Configure reqwest client with appropriate timeouts for LLM operations
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minutes total timeout
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: model.into(),
            max_tokens,
            temperature,
        }
    }

    /// Create from config
    pub fn from_config(config: &LlmConfig) -> Result<Self> {
        let api_key = config
            .openai_api_key
            .as_ref()
            .ok_or_else(|| OtlError::ConfigError("OpenAI API key required".to_string()))?;

        let base_url = config
            .openai_base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        // Configure reqwest client with appropriate timeouts for LLM operations
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minutes total timeout
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| Client::new());

        Ok(Self {
            client,
            api_key: api_key.clone(),
            base_url,
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        })
    }

    /// Set custom base URL (for Azure or compatible APIs)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn generate(&self, prompt: &str) -> Result<String> {
        let request = OpenAiRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            stream: None,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| OtlError::LlmError(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OtlError::LlmError(format!("OpenAI error: {error_text}")));
        }

        let result: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| OtlError::LlmError(format!("Failed to parse response: {e}")))?;

        result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| OtlError::LlmError("No response generated".to_string()))
    }

    async fn generate_stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        let request = OpenAiRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            stream: Some(true),
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| OtlError::LlmError(format!("Stream request failed: {e}")))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OtlError::LlmError(format!(
                "OpenAI stream error: {error_text}"
            )));
        }

        let stream = response.bytes_stream();

        let mapped_stream = stream.filter_map(|result| async move {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    // Parse SSE format: data: {...}
                    let mut content = String::new();
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                continue;
                            }
                            if let Ok(parsed) = serde_json::from_str::<StreamResponse>(data) {
                                if let Some(choice) = parsed.choices.first() {
                                    if let Some(c) = &choice.delta.content {
                                        content.push_str(c);
                                    }
                                }
                            }
                        }
                    }
                    if content.is_empty() {
                        None
                    } else {
                        Some(Ok(content))
                    }
                }
                Err(e) => Some(Err(OtlError::LlmError(format!("Stream error: {e}")))),
            }
        });

        Ok(Box::pin(mapped_stream))
    }
}

// ============================================================================
// Ollama Client
// ============================================================================

/// Ollama API client
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaResponse {
    response: String,
    done: bool,
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        // Configure reqwest client with appropriate timeouts
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minutes total timeout
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.into(),
            model: model.into(),
        }
    }

    /// Create from config
    pub fn from_config(config: &LlmConfig) -> Self {
        Self::new(config.ollama_url.clone(), config.model.clone())
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    async fn generate(&self, prompt: &str) -> Result<String> {
        tracing::info!("Ollama generate: sending request to {}", self.base_url);

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: Some(false),
        };

        tracing::debug!("Ollama generate: request prepared");

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| OtlError::LlmError(format!("Ollama request failed: {e}")))?;

        tracing::info!(
            "Ollama generate: received response with status {}",
            response.status()
        );

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OtlError::LlmError(format!("Ollama error: {error_text}")));
        }

        tracing::debug!("Ollama generate: parsing JSON response");

        let result: OllamaResponse = response
            .json()
            .await
            .map_err(|e| OtlError::LlmError(format!("Failed to parse Ollama response: {e}")))?;

        tracing::info!("Ollama generate: received {} chars", result.response.len());

        Ok(result.response)
    }

    async fn generate_stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        use futures::stream::StreamExt;
        use tokio_util::codec::{FramedRead, LinesCodec};

        tracing::info!("Starting Ollama stream request to {}", self.base_url);

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: Some(true),
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| OtlError::LlmError(format!("Ollama stream request failed: {e}")))?;

        tracing::info!("Received response with status: {}", response.status());

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OtlError::LlmError(format!(
                "Ollama stream error: {error_text}"
            )));
        }

        // Convert bytes stream into async reader
        let stream_reader = tokio_util::io::StreamReader::new(
            response
                .bytes_stream()
                .map(|result| result.map_err(std::io::Error::other)),
        );

        // Use LinesCodec to properly frame the stream by lines
        // Limit max line length to 64KB to prevent DoS attacks from malicious servers
        let lines_stream =
            FramedRead::new(stream_reader, LinesCodec::new_with_max_length(64 * 1024));

        // Process each line and extract the response field
        let mapped_stream = lines_stream.filter_map(|result| async move {
            match result {
                Ok(line) => {
                    if line.trim().is_empty() {
                        return None;
                    }

                    tracing::debug!(
                        "Received line from Ollama: {}",
                        &line[..line.len().min(100)]
                    );

                    match serde_json::from_str::<OllamaResponse>(&line) {
                        Ok(parsed) => {
                            if !parsed.response.is_empty() {
                                tracing::debug!("Parsed response chunk: {}", &parsed.response);
                                Some(Ok(parsed.response))
                            } else {
                                None
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse Ollama response line: {} - {}",
                                &line[..line.len().min(100)],
                                e
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                    Some(Err(OtlError::LlmError(format!("Stream error: {e}"))))
                }
            }
        });

        Ok(Box::pin(mapped_stream))
    }
}

// ============================================================================
// Factory function
// ============================================================================

/// Create an LLM client from config
pub fn create_llm_client(config: &LlmConfig) -> Result<Box<dyn LlmClient>> {
    match config.provider {
        LlmProvider::OpenAI | LlmProvider::Azure => {
            Ok(Box::new(OpenAiClient::from_config(config)?))
        }
        LlmProvider::Ollama => Ok(Box::new(OllamaClient::from_config(config))),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_client_creation() {
        let client = OpenAiClient::new("test-key", "gpt-4o-mini", 2048, 0.1);
        assert_eq!(client.model, "gpt-4o-mini");
    }

    #[test]
    fn test_ollama_client_creation() {
        let client = OllamaClient::new("http://localhost:11434", "llama2");
        assert_eq!(client.model, "llama2");
    }
}
