//! Tests for document management enhancements
//!
//! Author: hephaex@gmail.com

#[cfg(test)]
mod tests {
    use otl_core::{
        DiffChange, DiffChangeType, DocumentCollection, DocumentLineage, DocumentTag,
        DocumentVersion, LanguageDetection, ProcessingStatus, ProcessingStepType,
        RetentionPolicy, TagAssignment, VersionDiff,
    };
    use uuid::Uuid;

    #[test]
    fn test_document_version_creation() {
        let doc_id = Uuid::new_v4();
        let version = DocumentVersion::new(doc_id, 1, "Test Document", "/path/test.pdf")
            .with_content("This is version 1 content")
            .with_change_summary("Initial version");

        assert_eq!(version.document_id, doc_id);
        assert_eq!(version.version_number, 1);
        assert_eq!(version.title, "Test Document");
        assert!(version.content_snapshot.is_some());
        assert_eq!(
            version.change_summary,
            Some("Initial version".to_string())
        );
    }

    #[test]
    fn test_version_with_changed_by() {
        let doc_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let version = DocumentVersion::new(doc_id, 2, "Test", "/path/test.pdf")
            .with_changed_by(user_id);

        assert_eq!(version.changed_by, Some(user_id));
    }

    #[test]
    fn test_collection_hierarchy() {
        let parent = DocumentCollection::new("HR Documents")
            .with_description("All HR related documents");
        let child = DocumentCollection::new("Policies").with_parent(parent.id);

        assert_eq!(parent.name, "HR Documents");
        assert_eq!(parent.parent_id, None);
        assert_eq!(child.parent_id, Some(parent.id));
    }

    #[test]
    fn test_collection_access_level() {
        let collection = DocumentCollection::new("IT Docs")
            .with_access_level(otl_core::AccessLevel::Confidential);

        assert_eq!(
            collection.access_level,
            otl_core::AccessLevel::Confidential
        );
    }

    #[test]
    fn test_tag_creation() {
        let tag = DocumentTag::new("korean")
            .with_category("language")
            .with_color("#FF6B6B");

        assert_eq!(tag.name, "korean");
        assert_eq!(tag.category, Some("language".to_string()));
        assert_eq!(tag.color, Some("#FF6B6B".to_string()));
    }

    #[test]
    fn test_manual_tag_assignment() {
        let doc_id = Uuid::new_v4();
        let tag_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let assignment = TagAssignment::new_manual(doc_id, tag_id, user_id);

        assert_eq!(assignment.document_id, doc_id);
        assert_eq!(assignment.tag_id, tag_id);
        assert!(!assignment.auto_assigned);
        assert_eq!(assignment.assigned_by, Some(user_id));
        assert!(assignment.confidence.is_none());
    }

    #[test]
    fn test_auto_tag_assignment() {
        let doc_id = Uuid::new_v4();
        let tag_id = Uuid::new_v4();

        let assignment = TagAssignment::new_auto(doc_id, tag_id, 0.92);

        assert!(assignment.auto_assigned);
        assert_eq!(assignment.confidence, Some(0.92));
        assert!(assignment.assigned_by.is_none());
    }

    #[test]
    fn test_lineage_lifecycle() {
        let doc_id = Uuid::new_v4();
        let mut lineage =
            DocumentLineage::new(doc_id, "parse", ProcessingStepType::Processing);

        // Initial state
        assert_eq!(lineage.document_id, doc_id);
        assert_eq!(lineage.step_name, "parse");
        assert_eq!(lineage.status, ProcessingStatus::Pending);

        // Start processing
        lineage.start();
        assert_eq!(lineage.status, ProcessingStatus::Running);

        // Add metrics
        lineage.add_metric("chunk_count", 42);
        lineage.add_metric("avg_chunk_size", 850);

        // Complete successfully
        lineage.complete();
        assert_eq!(lineage.status, ProcessingStatus::Completed);
        assert!(lineage.completed_at.is_some());
        assert!(lineage.duration_ms.is_some());
    }

    #[test]
    fn test_lineage_failure() {
        let doc_id = Uuid::new_v4();
        let mut lineage = DocumentLineage::new(doc_id, "embed", ProcessingStepType::Indexing);

        lineage.start();
        lineage.fail("Vector store connection timeout");

        assert_eq!(lineage.status, ProcessingStatus::Failed);
        assert_eq!(
            lineage.error_message,
            Some("Vector store connection timeout".to_string())
        );
        assert!(lineage.completed_at.is_some());
        assert!(lineage.duration_ms.is_some());
    }

    #[test]
    fn test_retention_policy() {
        let policy = RetentionPolicy::new("Standard Policy", 90)
            .with_auto_purge(true)
            .for_access_level(otl_core::AccessLevel::Internal);

        assert_eq!(policy.retention_days, 90);
        assert!(policy.auto_purge);
        assert_eq!(policy.applies_to_access_level.len(), 1);
        assert_eq!(
            policy.applies_to_access_level[0],
            otl_core::AccessLevel::Internal
        );
    }

    #[test]
    fn test_version_diff() {
        let old_id = Uuid::new_v4();
        let new_id = Uuid::new_v4();
        let mut diff = VersionDiff::new(old_id, new_id);

        // Add some changes
        diff.add_change(DiffChange {
            change_type: DiffChangeType::Add,
            old_content: None,
            new_content: Some("New paragraph added".to_string()),
            old_line: None,
            new_line: Some(5),
        });

        diff.add_change(DiffChange {
            change_type: DiffChangeType::Delete,
            old_content: Some("Old content removed".to_string()),
            new_content: None,
            old_line: Some(10),
            new_line: None,
        });

        diff.add_change(DiffChange {
            change_type: DiffChangeType::Modify,
            old_content: Some("Original text".to_string()),
            new_content: Some("Modified text".to_string()),
            old_line: Some(15),
            new_line: Some(14),
        });

        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
        assert_eq!(diff.modifications, 1);
        assert_eq!(diff.changes.len(), 3);
    }

    #[test]
    fn test_language_detection() {
        let mut detection = LanguageDetection::new("ko", 0.95);

        assert_eq!(detection.language, "ko");
        assert_eq!(detection.language_name, "Korean");
        assert_eq!(detection.confidence, 0.95);

        detection.add_alternative("en", 0.03);
        detection.add_alternative("ja", 0.02);

        assert_eq!(detection.alternatives.len(), 2);
        assert_eq!(detection.alternatives[0].0, "en");
        assert_eq!(detection.alternatives[1].0, "ja");
    }

    #[test]
    fn test_language_detection_english() {
        let detection = LanguageDetection::new("en", 0.98);
        assert_eq!(detection.language_name, "English");
    }

    #[test]
    fn test_diff_statistics() {
        let old_id = Uuid::new_v4();
        let new_id = Uuid::new_v4();
        let mut diff = VersionDiff::new(old_id, new_id);

        // Add multiple changes
        for i in 0..5 {
            diff.add_change(DiffChange {
                change_type: DiffChangeType::Add,
                old_content: None,
                new_content: Some(format!("Added line {}", i)),
                old_line: None,
                new_line: Some(i + 1),
            });
        }

        for i in 0..3 {
            diff.add_change(DiffChange {
                change_type: DiffChangeType::Delete,
                old_content: Some(format!("Deleted line {}", i)),
                new_content: None,
                old_line: Some(i + 10),
                new_line: None,
            });
        }

        assert_eq!(diff.additions, 5);
        assert_eq!(diff.deletions, 3);
        assert_eq!(diff.changes.len(), 8);
    }
}
