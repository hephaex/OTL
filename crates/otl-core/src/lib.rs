//! OTL Core - Domain models, traits, and shared types
//!
//! This crate defines the core abstractions used throughout the OTL system:
//! - Ontology models (classes, properties, entities, triples)
//! - Access control (ACL) structures
//! - Common error types
//! - Shared traits for search backends
//! - Configuration management
//! - Metadata storage (PostgreSQL)

pub mod config;
pub mod metadata;

pub use config::{AppConfig, ConfigError, DatabaseConfig, LlmConfig, LlmProvider, RagConfig};
pub use metadata::{MetadataRepository, MetadataStore};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

/// Core error types for OTL operations
#[derive(Error, Debug)]
pub enum OtlError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },

    #[error("Invalid ontology: {0}")]
    InvalidOntology(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Search error: {0}")]
    SearchError(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, OtlError>;

// ============================================================================
// Access Control (ACL)
// ============================================================================

/// Document access level classification
///
/// Defines the security classification for documents:
/// - `Public`: Anyone can access
/// - `Internal`: Organization members only
/// - `Confidential`: Specific departments/roles only
/// - `Restricted`: Named individuals only
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessLevel {
    Public,
    #[default]
    Internal,
    Confidential,
    Restricted,
}

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => write!(f, "public"),
            Self::Internal => write!(f, "internal"),
            Self::Confidential => write!(f, "confidential"),
            Self::Restricted => write!(f, "restricted"),
        }
    }
}

/// Access control metadata for a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAcl {
    /// Security classification level
    pub access_level: AccessLevel,

    /// Owner user ID
    pub owner_id: Option<String>,

    /// Owning department
    pub department: Option<String>,

    /// Roles required to access this document
    pub required_roles: Vec<String>,

    /// Specific users allowed access (for Restricted level)
    pub allowed_users: Vec<String>,
}

impl Default for DocumentAcl {
    fn default() -> Self {
        Self {
            access_level: AccessLevel::Internal,
            owner_id: None,
            department: None,
            required_roles: Vec::new(),
            allowed_users: Vec::new(),
        }
    }
}

impl DocumentAcl {
    /// Check if a user can access this document
    pub fn can_access(&self, user: &User) -> bool {
        match self.access_level {
            AccessLevel::Public => true,
            AccessLevel::Internal => user.is_internal,
            AccessLevel::Confidential => {
                // Check department match or role match
                let dept_match = self
                    .department
                    .as_ref()
                    .map(|d| user.departments.contains(d))
                    .unwrap_or(false);

                let role_match = self.required_roles.iter().any(|r| user.roles.contains(r));

                dept_match || role_match
            }
            AccessLevel::Restricted => {
                // Must be in allowed_users list
                self.allowed_users.contains(&user.user_id)
                    || self.owner_id.as_ref() == Some(&user.user_id)
            }
        }
    }
}

/// User identity and permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub departments: Vec<String>,
    pub is_internal: bool,
}

impl User {
    /// Create an anonymous (public) user
    pub fn anonymous() -> Self {
        Self {
            user_id: "anonymous".to_string(),
            email: None,
            roles: Vec::new(),
            departments: Vec::new(),
            is_internal: false,
        }
    }

    /// Create an internal user with specified roles
    pub fn internal(user_id: impl Into<String>, roles: Vec<String>) -> Self {
        Self {
            user_id: user_id.into(),
            email: None,
            roles,
            departments: Vec::new(),
            is_internal: true,
        }
    }
}

// ============================================================================
// Ontology Models
// ============================================================================

/// Ontology class definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyClass {
    /// Unique identifier (e.g., "hr:Employee")
    pub id: String,

    /// Human-readable label
    pub label: String,

    /// Optional description
    pub description: Option<String>,

    /// Parent class (for inheritance)
    pub parent: Option<String>,

    /// Properties defined on this class
    pub properties: Vec<PropertyDefinition>,
}

/// Property definition for an ontology class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDefinition {
    /// Property name
    pub name: String,

    /// Data type
    pub data_type: DataType,

    /// Cardinality constraint
    pub cardinality: Cardinality,

    /// For object properties: the target class
    pub range: Option<String>,
}

/// Data types supported in the ontology
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    String,
    Integer,
    Float,
    DateTime,
    Boolean,
    /// Reference to another entity
    ObjectReference(String),
}

/// Cardinality constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cardinality {
    One,
    ZeroOrOne,
    Many,
    OneOrMore,
}

// ============================================================================
// Knowledge Graph Entities
// ============================================================================

/// An entity in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier
    pub id: Uuid,

    /// Ontology class this entity belongs to
    pub class: String,

    /// Property values
    pub properties: HashMap<String, serde_json::Value>,

    /// Source reference (where this entity was extracted from)
    pub source: SourceReference,

    /// When this entity was created
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Entity {
    /// Create a new entity
    pub fn new(class: impl Into<String>, source: SourceReference) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            class: class.into(),
            properties: HashMap::new(),
            source,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a property value
    pub fn with_property(
        mut self,
        name: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.properties.insert(name.into(), value.into());
        self
    }
}

/// A relationship triple (Subject, Predicate, Object)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Triple {
    /// Unique identifier
    pub id: Uuid,

    /// Subject entity ID
    pub subject: Uuid,

    /// Predicate (relationship type)
    pub predicate: String,

    /// Object entity ID
    pub object: Uuid,

    /// Source reference
    pub source: SourceReference,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,

    /// Extraction timestamp
    pub created_at: DateTime<Utc>,
}

impl Triple {
    /// Create a new triple
    pub fn new(
        subject: Uuid,
        predicate: impl Into<String>,
        object: Uuid,
        source: SourceReference,
        confidence: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject,
            predicate: predicate.into(),
            object,
            source,
            confidence,
            created_at: Utc::now(),
        }
    }
}

/// Reference to the source of extracted knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    /// Document ID
    pub document_id: Uuid,

    /// Page number (if applicable)
    pub page: Option<u32>,

    /// Section name or heading
    pub section: Option<String>,

    /// Character offset in the document
    pub offset: Option<usize>,

    /// Extraction confidence score
    pub confidence: f32,
}

impl SourceReference {
    /// Create a new source reference
    pub fn new(document_id: Uuid) -> Self {
        Self {
            document_id,
            page: None,
            section: None,
            offset: None,
            confidence: 1.0,
        }
    }

    /// Set page number
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Set section name
    pub fn with_section(mut self, section: impl Into<String>) -> Self {
        self.section = Some(section.into());
        self
    }

    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}

// ============================================================================
// Document Models
// ============================================================================

/// Metadata for a document in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Unique identifier
    pub id: Uuid,

    /// Document title
    pub title: String,

    /// Original file path
    pub file_path: String,

    /// File type (pdf, docx, xlsx, etc.)
    pub file_type: String,

    /// File size in bytes
    pub file_size: u64,

    /// Access control settings
    pub acl: DocumentAcl,

    /// Upload timestamp
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,

    /// Additional metadata (custom fields)
    pub extra: HashMap<String, serde_json::Value>,
}

impl DocumentMetadata {
    /// Create new document metadata
    pub fn new(
        title: impl Into<String>,
        file_path: impl Into<String>,
        file_type: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            file_path: file_path.into(),
            file_type: file_type.into(),
            file_size: 0,
            acl: DocumentAcl::default(),
            created_at: now,
            updated_at: now,
            extra: HashMap::new(),
        }
    }

    /// Set ACL
    pub fn with_acl(mut self, acl: DocumentAcl) -> Self {
        self.acl = acl;
        self
    }
}

/// A chunk of document content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// Unique identifier
    pub id: Uuid,

    /// Parent document ID
    pub document_id: Uuid,

    /// Chunk index within the document
    pub chunk_index: u32,

    /// Text content
    pub content: String,

    /// Page number (if applicable)
    pub page_number: Option<u32>,

    /// Section name
    pub section_name: Option<String>,

    /// Vector ID in the vector store
    pub vector_id: Option<String>,
}

impl DocumentChunk {
    /// Create a new chunk
    pub fn new(document_id: Uuid, chunk_index: u32, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            document_id,
            chunk_index,
            content: content.into(),
            page_number: None,
            section_name: None,
            vector_id: None,
        }
    }
}

// ============================================================================
// Search and RAG Types
// ============================================================================

/// Search result from any backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Content snippet
    pub content: String,

    /// Relevance score (higher is better)
    pub score: f32,

    /// Source reference
    pub source: SourceReference,

    /// Access control metadata
    pub acl: DocumentAcl,

    /// Search result type
    pub result_type: SearchResultType,
}

/// Type of search result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchResultType {
    /// Vector similarity search result
    Vector,
    /// Graph traversal result
    Graph,
    /// Keyword search result
    Keyword,
}

/// RAG query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagQuery {
    /// User's question
    pub question: String,

    /// Maximum number of results to return
    pub top_k: usize,

    /// Minimum relevance score threshold
    pub min_score: Option<f32>,

    /// Filter by document IDs
    pub document_filter: Option<Vec<Uuid>>,
}

impl RagQuery {
    /// Create a new RAG query
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            top_k: 10,
            min_score: None,
            document_filter: None,
        }
    }

    /// Set top-k
    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = k;
        self
    }
}

/// RAG response with answer and citations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    /// Generated answer
    pub answer: String,

    /// Citations used in the answer
    pub citations: Vec<Citation>,

    /// Confidence score
    pub confidence: f32,

    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Citation for a claim in the answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Citation index (e.g., [1], [2])
    pub index: u32,

    /// Cited text snippet
    pub text: String,

    /// Source reference
    pub source: SourceReference,

    /// Document title
    pub document_title: String,
}

// ============================================================================
// HITL Verification Types
// ============================================================================

/// Status of an extraction for HITL verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationStatus {
    /// Awaiting review
    Pending,
    /// Under review
    InReview,
    /// Approved and loaded to graph
    Approved,
    /// Rejected
    Rejected,
}

/// Extraction awaiting verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionForVerification {
    /// Unique identifier
    pub id: Uuid,

    /// Source document ID
    pub document_id: Uuid,

    /// Extracted triples
    pub triples: Vec<Triple>,

    /// Original text snippet
    pub source_text: String,

    /// Confidence score
    pub confidence: f32,

    /// Current status
    pub status: VerificationStatus,

    /// Reviewer ID (if reviewed)
    pub reviewer_id: Option<String>,

    /// Review notes
    pub review_notes: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Review timestamp
    pub reviewed_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Traits
// ============================================================================

/// Trait for search backends (vector, graph, keyword)
#[async_trait::async_trait]
pub trait SearchBackend: Send + Sync {
    /// Search for relevant content
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;

    /// Get backend name for logging
    fn name(&self) -> &str;
}

/// Trait for LLM clients
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate a response
    async fn generate(&self, prompt: &str) -> Result<String>;

    /// Generate a streaming response
    async fn generate_stream(
        &self,
        prompt: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<String>>>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acl_public_access() {
        let acl = DocumentAcl {
            access_level: AccessLevel::Public,
            ..Default::default()
        };
        let user = User::anonymous();
        assert!(acl.can_access(&user));
    }

    #[test]
    fn test_acl_internal_access() {
        let acl = DocumentAcl {
            access_level: AccessLevel::Internal,
            ..Default::default()
        };

        let internal_user = User::internal("user1", vec![]);
        let anonymous = User::anonymous();

        assert!(acl.can_access(&internal_user));
        assert!(!acl.can_access(&anonymous));
    }

    #[test]
    fn test_acl_confidential_role_match() {
        let acl = DocumentAcl {
            access_level: AccessLevel::Confidential,
            required_roles: vec!["HR_ADMIN".to_string()],
            ..Default::default()
        };

        let hr_admin = User::internal("user1", vec!["HR_ADMIN".to_string()]);
        let regular_user = User::internal("user2", vec!["EMPLOYEE".to_string()]);

        assert!(acl.can_access(&hr_admin));
        assert!(!acl.can_access(&regular_user));
    }

    #[test]
    fn test_acl_restricted_allowed_users() {
        let acl = DocumentAcl {
            access_level: AccessLevel::Restricted,
            allowed_users: vec!["ceo".to_string(), "cfo".to_string()],
            ..Default::default()
        };

        let ceo = User::internal("ceo", vec![]);
        let random = User::internal("random", vec![]);

        assert!(acl.can_access(&ceo));
        assert!(!acl.can_access(&random));
    }

    #[test]
    fn test_entity_builder() {
        let source = SourceReference::new(Uuid::new_v4())
            .with_page(5)
            .with_section("Chapter 1");

        let entity = Entity::new("hr:Employee", source)
            .with_property("name", "John Doe")
            .with_property("employeeId", "E001");

        assert_eq!(entity.class, "hr:Employee");
        assert_eq!(
            entity.properties.get("name"),
            Some(&serde_json::json!("John Doe"))
        );
    }

    #[test]
    fn test_access_level_ordering() {
        assert!(AccessLevel::Public < AccessLevel::Internal);
        assert!(AccessLevel::Internal < AccessLevel::Confidential);
        assert!(AccessLevel::Confidential < AccessLevel::Restricted);
    }
}
