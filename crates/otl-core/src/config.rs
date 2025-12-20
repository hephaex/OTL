//! OTL Configuration Management
//!
//! Handles configuration from environment variables, config files,
//! and command-line arguments with sensible defaults for development.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Database connections
    pub database: DatabaseConfig,

    /// LLM provider configuration
    pub llm: LlmConfig,

    /// RAG pipeline configuration
    pub rag: RagConfig,

    /// Logging configuration
    pub logging: LoggingConfig,
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();

        // Server
        if let Ok(host) = std::env::var("API_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("API_PORT") {
            config.server.port = port.parse().map_err(|_| ConfigError::InvalidValue {
                key: "API_PORT".to_string(),
                value: port,
            })?;
        }

        // PostgreSQL
        if let Ok(url) = std::env::var("DATABASE_URL") {
            config.database.postgres_url = url;
        }

        // SurrealDB
        if let Ok(url) = std::env::var("SURREALDB_URL") {
            config.database.surrealdb_url = url;
        }
        if let Ok(user) = std::env::var("SURREALDB_USER") {
            config.database.surrealdb_user = user;
        }
        if let Ok(pass) = std::env::var("SURREALDB_PASS") {
            config.database.surrealdb_pass = pass;
        }

        // Qdrant
        if let Ok(url) = std::env::var("QDRANT_URL") {
            config.database.qdrant_url = url;
        }

        // LLM
        if let Ok(provider) = std::env::var("LLM_PROVIDER") {
            config.llm.provider = provider.parse()?;
        }
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            config.llm.openai_api_key = Some(key);
        }
        if let Ok(url) = std::env::var("OLLAMA_URL") {
            config.llm.ollama_url = url;
        }
        if let Ok(model) = std::env::var("LLM_MODEL") {
            config.llm.model = model;
        }
        if let Ok(model) = std::env::var("EMBEDDING_MODEL") {
            config.llm.embedding_model = model;
        }

        // CORS origins from environment variable (comma-separated)
        if let Ok(origins) = std::env::var("CORS_ORIGINS") {
            config.server.cors_origins = origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // Logging
        if let Ok(level) = std::env::var("LOG_LEVEL") {
            config.logging.level = level;
        }

        Ok(config)
    }

    /// Load from a TOML file
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self, ConfigError> {
        let path = path.into();
        let content = std::fs::read_to_string(&path).map_err(|e| ConfigError::FileReadError {
            path: path.clone(),
            source: e,
        })?;

        toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            path,
            message: e.to_string(),
        })
    }

    /// Merge with environment variables (env takes precedence)
    pub fn with_env_override(mut self) -> Result<Self, ConfigError> {
        let env_config = Self::from_env()?;

        // Only override if env values differ from defaults
        if env_config.server.host != ServerConfig::default().host {
            self.server.host = env_config.server.host;
        }
        if env_config.server.port != ServerConfig::default().port {
            self.server.port = env_config.server.port;
        }

        // Always use env for sensitive values
        if env_config.llm.openai_api_key.is_some() {
            self.llm.openai_api_key = env_config.llm.openai_api_key;
        }

        Ok(self)
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,

    /// Port to listen on
    pub port: u16,

    /// Request timeout in seconds
    pub request_timeout_secs: u64,

    /// Maximum request body size in bytes
    pub max_body_size: usize,

    /// Enable CORS
    pub cors_enabled: bool,

    /// Allowed origins for CORS
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 300,
            max_body_size: 10 * 1024 * 1024, // 10MB
            cors_enabled: true,
            // Empty by default for security - set via CORS_ORIGINS env var
            cors_origins: vec![],
        }
    }
}

/// Database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub postgres_url: String,

    /// PostgreSQL connection pool size
    pub postgres_pool_size: u32,

    /// SurrealDB WebSocket URL
    pub surrealdb_url: String,

    /// SurrealDB username
    pub surrealdb_user: String,

    /// SurrealDB password
    pub surrealdb_pass: String,

    /// SurrealDB namespace
    pub surrealdb_namespace: String,

    /// SurrealDB database name
    pub surrealdb_database: String,

    /// Qdrant gRPC URL
    pub qdrant_url: String,

    /// Qdrant collection name
    pub qdrant_collection: String,

    /// Vector dimension (must match embedding model)
    pub vector_dimension: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgres_url: "postgres://otl:otl_dev_password@localhost:5432/otl".to_string(),
            postgres_pool_size: 10,
            surrealdb_url: "ws://localhost:8000".to_string(),
            surrealdb_user: "root".to_string(),
            surrealdb_pass: "root".to_string(),
            surrealdb_namespace: "otl".to_string(),
            surrealdb_database: "knowledge".to_string(),
            qdrant_url: "http://localhost:6334".to_string(),
            qdrant_collection: "otl_chunks".to_string(),
            vector_dimension: 1536, // OpenAI text-embedding-3-small
        }
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// LLM provider to use
    pub provider: LlmProvider,

    /// OpenAI API key
    pub openai_api_key: Option<String>,

    /// OpenAI API base URL (for Azure or compatible APIs)
    pub openai_base_url: Option<String>,

    /// Ollama server URL
    pub ollama_url: String,

    /// Model name to use
    pub model: String,

    /// Embedding model name
    pub embedding_model: String,

    /// Maximum tokens for completion
    pub max_tokens: u32,

    /// Temperature for generation
    pub temperature: f32,

    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::OpenAI,
            openai_api_key: None,
            openai_base_url: None,
            ollama_url: "http://localhost:11434".to_string(),
            model: "gpt-4o-mini".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            max_tokens: 2048,
            temperature: 0.1,
            timeout_secs: 60,
        }
    }
}

/// Supported LLM providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    OpenAI,
    Ollama,
    Azure,
}

impl std::str::FromStr for LlmProvider {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Self::OpenAI),
            "ollama" => Ok(Self::Ollama),
            "azure" => Ok(Self::Azure),
            _ => Err(ConfigError::InvalidValue {
                key: "LLM_PROVIDER".to_string(),
                value: s.to_string(),
            }),
        }
    }
}

/// RAG pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Number of results from vector search
    pub vector_top_k: usize,

    /// Graph traversal depth
    pub graph_depth: u32,

    /// Final number of results after merging
    pub final_top_k: usize,

    /// RRF constant
    pub rrf_k: f32,

    /// Vector search weight
    pub vector_weight: f32,

    /// Graph search weight
    pub graph_weight: f32,

    /// Maximum context length (characters)
    pub max_context_length: usize,

    /// Include ontology schema in prompt
    pub include_ontology: bool,

    /// Chunk size for document processing
    pub chunk_size: usize,

    /// Chunk overlap
    pub chunk_overlap: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            vector_top_k: 20,
            graph_depth: 2,
            final_top_k: 5,
            rrf_k: 60.0,
            vector_weight: 1.0,
            graph_weight: 1.5,
            max_context_length: 8000,
            include_ontology: true,
            chunk_size: 1000,
            chunk_overlap: 200,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// JSON format for logs
    pub json_format: bool,

    /// Include file/line in logs
    pub include_location: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            include_location: false,
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file {path}: {source}")]
    FileReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse config file {path}: {message}")]
    ParseError { path: PathBuf, message: String },

    #[error("Invalid value for {key}: {value}")]
    InvalidValue { key: String, value: String },

    #[error("Missing required configuration: {0}")]
    MissingRequired(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.vector_dimension, 1536);
    }

    #[test]
    fn test_llm_provider_parse() {
        assert_eq!(
            "openai".parse::<LlmProvider>().unwrap(),
            LlmProvider::OpenAI
        );
        assert_eq!(
            "ollama".parse::<LlmProvider>().unwrap(),
            LlmProvider::Ollama
        );
        assert!("invalid".parse::<LlmProvider>().is_err());
    }
}
