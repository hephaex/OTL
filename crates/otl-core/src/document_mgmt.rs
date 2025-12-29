// ! Document Management Models
//!
//! Extended models for document versioning, collections, tags, and lineage tracking
//! Author: hephaex@gmail.com

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::AccessLevel;

// ============================================================================
// Document Versioning
// ============================================================================

/// A specific version of a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    /// Unique version ID
    pub id: Uuid,

    /// Parent document ID
    pub document_id: Uuid,

    /// Sequential version number (1, 2, 3, ...)
    pub version_number: i32,

    /// Document title at this version
    pub title: String,

    /// File path at this version
    pub file_path: String,

    /// File size in bytes
    pub file_size: i64,

    /// SHA-256 hash of content
    pub file_hash: Option<String>,

    /// Summary of changes in this version
    pub change_summary: Option<String>,

    /// User who created this version
    pub changed_by: Option<Uuid>,

    /// Parent version ID (for change tracking)
    pub parent_version_id: Option<Uuid>,

    /// Full content snapshot for diff
    pub content_snapshot: Option<String>,

    /// Metadata snapshot
    pub metadata_snapshot: HashMap<String, serde_json::Value>,

    /// When this version was created
    pub created_at: DateTime<Utc>,
}

impl DocumentVersion {
    /// Create a new document version
    pub fn new(
        document_id: Uuid,
        version_number: i32,
        title: impl Into<String>,
        file_path: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            document_id,
            version_number,
            title: title.into(),
            file_path: file_path.into(),
            file_size: 0,
            file_hash: None,
            change_summary: None,
            changed_by: None,
            parent_version_id: None,
            content_snapshot: None,
            metadata_snapshot: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Set content snapshot
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content_snapshot = Some(content.into());
        self
    }

    /// Set change summary
    pub fn with_change_summary(mut self, summary: impl Into<String>) -> Self {
        self.change_summary = Some(summary.into());
        self
    }

    /// Set who changed it
    pub fn with_changed_by(mut self, user_id: Uuid) -> Self {
        self.changed_by = Some(user_id);
        self
    }
}

// ============================================================================
// Document Collections/Folders
// ============================================================================

/// A collection/folder for organizing documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentCollection {
    /// Unique collection ID
    pub id: Uuid,

    /// Collection name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Parent collection (for hierarchical structure)
    pub parent_id: Option<Uuid>,

    /// Access control (inherited by documents unless overridden)
    pub access_level: AccessLevel,

    /// Collection owner
    pub owner_id: Option<Uuid>,

    /// Department
    pub department: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Full path (e.g., "/root/folder1/subfolder2")
    pub path: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Soft delete timestamp
    pub deleted_at: Option<DateTime<Utc>>,
}

impl DocumentCollection {
    /// Create a new collection
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            parent_id: None,
            access_level: AccessLevel::Internal,
            owner_id: None,
            department: None,
            metadata: HashMap::new(),
            path: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set parent collection
    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set access level
    pub fn with_access_level(mut self, level: AccessLevel) -> Self {
        self.access_level = level;
        self
    }
}

/// Membership of a document in a collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMembership {
    /// Unique membership ID
    pub id: Uuid,

    /// Document ID
    pub document_id: Uuid,

    /// Collection ID
    pub collection_id: Uuid,

    /// When the document was added
    pub added_at: DateTime<Utc>,

    /// Who added it
    pub added_by: Option<Uuid>,
}

// ============================================================================
// Document Tags
// ============================================================================

/// A tag for document classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTag {
    /// Unique tag ID
    pub id: Uuid,

    /// Tag name (unique)
    pub name: String,

    /// Tag category (e.g., 'language', 'department', 'topic', 'auto')
    pub category: Option<String>,

    /// Description
    pub description: Option<String>,

    /// Color for UI (hex format)
    pub color: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl DocumentTag {
    /// Create a new tag
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            category: None,
            description: None,
            color: None,
            created_at: Utc::now(),
        }
    }

    /// Set category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set color
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// Assignment of a tag to a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagAssignment {
    /// Unique assignment ID
    pub id: Uuid,

    /// Document ID
    pub document_id: Uuid,

    /// Tag ID
    pub tag_id: Uuid,

    /// Whether this was auto-assigned
    pub auto_assigned: bool,

    /// Confidence score for auto-assigned tags (0.0 to 1.0)
    pub confidence: Option<f32>,

    /// Who assigned the tag (for manual assignments)
    pub assigned_by: Option<Uuid>,

    /// When the tag was assigned
    pub assigned_at: DateTime<Utc>,
}

impl TagAssignment {
    /// Create a new manual tag assignment
    pub fn new_manual(document_id: Uuid, tag_id: Uuid, assigned_by: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            document_id,
            tag_id,
            auto_assigned: false,
            confidence: None,
            assigned_by: Some(assigned_by),
            assigned_at: Utc::now(),
        }
    }

    /// Create a new auto-assigned tag
    pub fn new_auto(document_id: Uuid, tag_id: Uuid, confidence: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            document_id,
            tag_id,
            auto_assigned: true,
            confidence: Some(confidence),
            assigned_by: None,
            assigned_at: Utc::now(),
        }
    }
}

// ============================================================================
// Document Lineage/Processing History
// ============================================================================

/// Processing step status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Type of processing step
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingStepType {
    Ingestion,
    Processing,
    Extraction,
    Indexing,
    Analysis,
}

/// A processing step in document lineage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLineage {
    /// Unique lineage record ID
    pub id: Uuid,

    /// Document ID
    pub document_id: Uuid,

    /// Version ID (if applicable)
    pub version_id: Option<Uuid>,

    /// Step name (e.g., 'upload', 'parse', 'chunk', 'embed')
    pub step_name: String,

    /// Step type category
    pub step_type: ProcessingStepType,

    /// Current status
    pub status: ProcessingStatus,

    /// When processing started
    pub started_at: DateTime<Utc>,

    /// When processing completed
    pub completed_at: Option<DateTime<Utc>>,

    /// Duration in milliseconds
    pub duration_ms: Option<i32>,

    /// Input data for this step
    pub input_data: HashMap<String, serde_json::Value>,

    /// Output data from this step
    pub output_data: HashMap<String, serde_json::Value>,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Processing metrics (chunk_count, entity_count, etc.)
    pub metrics: HashMap<String, serde_json::Value>,

    /// Who triggered this step
    pub triggered_by: Option<Uuid>,

    /// Parent step ID (for step dependencies)
    pub parent_step_id: Option<Uuid>,
}

impl DocumentLineage {
    /// Create a new lineage record
    pub fn new(
        document_id: Uuid,
        step_name: impl Into<String>,
        step_type: ProcessingStepType,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            document_id,
            version_id: None,
            step_name: step_name.into(),
            step_type,
            status: ProcessingStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            input_data: HashMap::new(),
            output_data: HashMap::new(),
            error_message: None,
            metrics: HashMap::new(),
            triggered_by: None,
            parent_step_id: None,
        }
    }

    /// Mark as running
    pub fn start(&mut self) {
        self.status = ProcessingStatus::Running;
        self.started_at = Utc::now();
    }

    /// Mark as completed
    pub fn complete(&mut self) {
        self.status = ProcessingStatus::Completed;
        self.completed_at = Some(Utc::now());
        if let Some(completed) = self.completed_at {
            self.duration_ms = Some((completed - self.started_at).num_milliseconds() as i32);
        }
    }

    /// Mark as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = ProcessingStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error.into());
        if let Some(completed) = self.completed_at {
            self.duration_ms = Some((completed - self.started_at).num_milliseconds() as i32);
        }
    }

    /// Add a metric
    pub fn add_metric(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.metrics.insert(key.into(), value.into());
    }
}

// ============================================================================
// Retention Policies
// ============================================================================

/// Document retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Unique policy ID
    pub id: Uuid,

    /// Policy name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Retention period in days
    pub retention_days: i32,

    /// Automatically purge after retention period
    pub auto_purge: bool,

    /// Applies to these access levels
    pub applies_to_access_level: Vec<AccessLevel>,

    /// Applies to these departments
    pub applies_to_departments: Vec<String>,

    /// Applies to these collections
    pub applies_to_collections: Vec<Uuid>,

    /// Priority (higher = higher priority when multiple policies match)
    pub priority: i32,

    /// Whether this policy is active
    pub is_active: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl RetentionPolicy {
    /// Create a new retention policy
    pub fn new(name: impl Into<String>, retention_days: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            retention_days,
            auto_purge: false,
            applies_to_access_level: Vec::new(),
            applies_to_departments: Vec::new(),
            applies_to_collections: Vec::new(),
            priority: 0,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set auto-purge
    pub fn with_auto_purge(mut self, enabled: bool) -> Self {
        self.auto_purge = enabled;
        self
    }

    /// Add access level applicability
    pub fn for_access_level(mut self, level: AccessLevel) -> Self {
        self.applies_to_access_level.push(level);
        self
    }
}

// ============================================================================
// Content Diff
// ============================================================================

/// Type of diff change
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffChangeType {
    /// Content was added
    Add,
    /// Content was removed
    Delete,
    /// Content was modified
    Modify,
    /// Content unchanged
    Equal,
}

/// A single diff change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffChange {
    /// Type of change
    pub change_type: DiffChangeType,

    /// Old content (for delete/modify)
    pub old_content: Option<String>,

    /// New content (for add/modify)
    pub new_content: Option<String>,

    /// Line number in old version
    pub old_line: Option<usize>,

    /// Line number in new version
    pub new_line: Option<usize>,
}

/// Diff between two document versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    /// Old version ID
    pub old_version_id: Uuid,

    /// New version ID
    pub new_version_id: Uuid,

    /// List of changes
    pub changes: Vec<DiffChange>,

    /// Summary statistics
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
}

impl VersionDiff {
    /// Create a new version diff
    pub fn new(old_version_id: Uuid, new_version_id: Uuid) -> Self {
        Self {
            old_version_id,
            new_version_id,
            changes: Vec::new(),
            additions: 0,
            deletions: 0,
            modifications: 0,
        }
    }

    /// Add a change
    pub fn add_change(&mut self, change: DiffChange) {
        match change.change_type {
            DiffChangeType::Add => self.additions += 1,
            DiffChangeType::Delete => self.deletions += 1,
            DiffChangeType::Modify => self.modifications += 1,
            DiffChangeType::Equal => {}
        }
        self.changes.push(change);
    }
}

// ============================================================================
// Language Detection
// ============================================================================

/// Detected language with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDetection {
    /// ISO 639-1 language code (e.g., 'en', 'ko')
    pub language: String,

    /// Full language name (e.g., 'English', 'Korean')
    pub language_name: String,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Alternative languages with their confidence scores
    pub alternatives: Vec<(String, f32)>,
}

impl LanguageDetection {
    /// Create a new language detection result
    pub fn new(language: impl Into<String>, confidence: f32) -> Self {
        let lang = language.into();
        let name = language_code_to_name(&lang);

        Self {
            language: lang,
            language_name: name,
            confidence,
            alternatives: Vec::new(),
        }
    }

    /// Add an alternative language
    pub fn add_alternative(&mut self, language: impl Into<String>, confidence: f32) {
        self.alternatives.push((language.into(), confidence));
    }
}

/// Convert ISO 639-1 language code to full name
fn language_code_to_name(code: &str) -> String {
    match code.to_lowercase().as_str() {
        "ko" => "Korean",
        "en" => "English",
        "ja" => "Japanese",
        "zh" => "Chinese",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "it" => "Italian",
        "pt" => "Portuguese",
        "ru" => "Russian",
        _ => "Unknown",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_version_builder() {
        let doc_id = Uuid::new_v4();
        let version = DocumentVersion::new(doc_id, 1, "Test", "/path/test.pdf")
            .with_content("Test content")
            .with_change_summary("Initial version");

        assert_eq!(version.version_number, 1);
        assert_eq!(version.title, "Test");
        assert!(version.content_snapshot.is_some());
        assert!(version.change_summary.is_some());
    }

    #[test]
    fn test_collection_hierarchy() {
        let parent = DocumentCollection::new("Parent Folder");
        let child = DocumentCollection::new("Child Folder").with_parent(parent.id);

        assert_eq!(parent.parent_id, None);
        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[test]
    fn test_tag_assignment() {
        let doc_id = Uuid::new_v4();
        let tag_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let manual = TagAssignment::new_manual(doc_id, tag_id, user_id);
        assert!(!manual.auto_assigned);
        assert!(manual.assigned_by.is_some());

        let auto = TagAssignment::new_auto(doc_id, tag_id, 0.95);
        assert!(auto.auto_assigned);
        assert_eq!(auto.confidence, Some(0.95));
    }

    #[test]
    fn test_lineage_lifecycle() {
        let doc_id = Uuid::new_v4();
        let mut lineage =
            DocumentLineage::new(doc_id, "parse", ProcessingStepType::Processing);

        assert_eq!(lineage.status, ProcessingStatus::Pending);

        lineage.start();
        assert_eq!(lineage.status, ProcessingStatus::Running);

        lineage.add_metric("chunks", 42);
        lineage.complete();
        assert_eq!(lineage.status, ProcessingStatus::Completed);
        assert!(lineage.completed_at.is_some());
        assert!(lineage.duration_ms.is_some());
    }

    #[test]
    fn test_language_detection() {
        let mut detection = LanguageDetection::new("en", 0.95);
        detection.add_alternative("es", 0.03);
        detection.add_alternative("fr", 0.02);

        assert_eq!(detection.language, "en");
        assert_eq!(detection.language_name, "English");
        assert_eq!(detection.alternatives.len(), 2);
    }

    #[test]
    fn test_version_diff() {
        let old_id = Uuid::new_v4();
        let new_id = Uuid::new_v4();
        let mut diff = VersionDiff::new(old_id, new_id);

        diff.add_change(DiffChange {
            change_type: DiffChangeType::Add,
            old_content: None,
            new_content: Some("new line".to_string()),
            old_line: None,
            new_line: Some(1),
        });

        diff.add_change(DiffChange {
            change_type: DiffChangeType::Delete,
            old_content: Some("old line".to_string()),
            new_content: None,
            old_line: Some(5),
            new_line: None,
        });

        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
        assert_eq!(diff.modifications, 0);
        assert_eq!(diff.changes.len(), 2);
    }
}
