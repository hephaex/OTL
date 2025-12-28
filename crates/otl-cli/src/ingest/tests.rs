//! Tests for the ingestion pipeline

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = IngestCheckpoint::new();
        assert_eq!(checkpoint.processed_files.len(), 0);
        assert_eq!(checkpoint.failed_files.len(), 0);
        assert_eq!(checkpoint.total_files, 0);
    }

    #[test]
    fn test_checkpoint_mark_processed() {
        let mut checkpoint = IngestCheckpoint::new();
        checkpoint.total_files = 10;

        checkpoint.mark_processed("file1.pdf");
        checkpoint.mark_processed("file2.pdf");

        assert_eq!(checkpoint.processed_files.len(), 2);
        assert!(checkpoint.is_processed("file1.pdf"));
        assert!(checkpoint.is_processed("file2.pdf"));
        assert!(!checkpoint.is_processed("file3.pdf"));
    }

    #[test]
    fn test_checkpoint_mark_failed() {
        let mut checkpoint = IngestCheckpoint::new();

        checkpoint.mark_failed("file1.pdf", "Parse error");
        assert_eq!(checkpoint.failed_files.len(), 1);
        assert_eq!(
            checkpoint.failed_files.get("file1.pdf"),
            Some(&"Parse error".to_string())
        );
    }

    #[test]
    fn test_checkpoint_save_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let checkpoint_path = temp_dir.path().join("checkpoint.json");

        let mut checkpoint = IngestCheckpoint::new();
        checkpoint.mark_processed("file1.pdf");
        checkpoint.mark_failed("file2.pdf", "Error");
        checkpoint.total_files = 10;

        checkpoint.save(&checkpoint_path)?;
        assert!(checkpoint_path.exists());

        let loaded = IngestCheckpoint::load(&checkpoint_path)?;
        assert_eq!(loaded.processed_files.len(), 1);
        assert_eq!(loaded.failed_files.len(), 1);
        assert_eq!(loaded.total_files, 10);

        Ok(())
    }

    #[test]
    fn test_checkpoint_completion_percentage() {
        let mut checkpoint = IngestCheckpoint::new();
        checkpoint.total_files = 100;

        assert_eq!(checkpoint.completion_percentage(), 0.0);

        for i in 0..50 {
            checkpoint.mark_processed(&format!("file{}.pdf", i));
        }

        assert_eq!(checkpoint.completion_percentage(), 50.0);
    }

    #[test]
    fn test_discover_files_single_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content")?;

        let files = discover_files(&file_path)?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], file_path);

        Ok(())
    }

    #[test]
    fn test_discover_files_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // Create test files
        fs::write(temp_dir.path().join("file1.txt"), "content")?;
        fs::write(temp_dir.path().join("file2.pdf"), "content")?;
        fs::write(temp_dir.path().join("file3.docx"), "content")?;
        fs::write(temp_dir.path().join("file4.xyz"), "content")?; // Unsupported

        let files = discover_files(temp_dir.path())?;

        // Should only include supported file types
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.file_name().unwrap() == "file1.txt"));
        assert!(files.iter().any(|f| f.file_name().unwrap() == "file2.pdf"));
        assert!(files.iter().any(|f| f.file_name().unwrap() == "file3.docx"));
        assert!(!files.iter().any(|f| f.file_name().unwrap() == "file4.xyz"));

        Ok(())
    }

    #[test]
    fn test_discover_files_nested_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // Create nested structure
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir)?;

        fs::write(temp_dir.path().join("root.txt"), "content")?;
        fs::write(subdir.join("nested.pdf"), "content")?;

        let files = discover_files(temp_dir.path())?;
        assert_eq!(files.len(), 2);

        Ok(())
    }

    #[test]
    fn test_validate_path_existing_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test")?;

        validate_path(&file_path)?;
        Ok(())
    }

    #[test]
    fn test_validate_path_nonexistent() {
        let result = validate_path(Path::new("/nonexistent/path/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_unsupported_type() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.xyz");
        fs::write(&file_path, "test")?;

        let result = validate_path(&file_path);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_ingest_stats_to_string() {
        let stats = IngestStats {
            files_processed: 10,
            files_failed: 2,
            documents_created: 10,
            chunks_created: 100,
            entities_extracted: 50,
            relations_extracted: 25,
            embeddings_created: 100,
            total_bytes_processed: 1024 * 1024,
        };

        let status = stats.to_status_string();
        assert!(status.contains("Files: 10/12"));
        assert!(status.contains("Docs: 10"));
        assert!(status.contains("Chunks: 100"));
        assert!(status.contains("Entities: 50"));
        assert!(status.contains("Relations: 25"));
        assert!(status.contains("Embeddings: 100"));
    }

    #[test]
    fn test_ingest_config_default() {
        let config = IngestConfig::default();
        assert_eq!(config.parallel_workers, 4);
        assert_eq!(config.batch_size, 100);
        assert!(!config.watch);
        assert!(config.show_progress);
        assert!(!config.dry_run);
        assert_eq!(config.max_retries, 3);
    }
}
