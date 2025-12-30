//! Document ingestion pipeline for batch processing
//!
//! This module provides a comprehensive CLI-based document ingestion system with:
//! - Directory scanning and file type detection
//! - Watch mode for continuous ingestion
//! - Progress reporting with progress bars
//! - Parallel batch processing with configurable workers
//! - Checkpoint/resume functionality for interrupted jobs
//! - Error recovery and retry logic
//! - Dry-run mode for validation
//!
//! Pipeline: scan → parse → chunk → extract → embed → index

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use otl_core::config::AppConfig;
use otl_extractor::ner::RuleBasedNer;
use otl_extractor::relation::RuleBasedRe;
use otl_extractor::{EntityExtractor, RelationExtractor};
use otl_parser::{chunk_document, ChunkConfig, FileType, ParserRegistry};
use otl_vector::{create_embedding_client, EmbeddingClient, EmbeddingVector};

#[cfg(test)]
mod tests;

// ============================================================================
// Configuration
// ============================================================================

/// Ingestion configuration
#[derive(Debug, Clone)]
pub struct IngestConfig {
    /// Number of parallel workers
    pub parallel_workers: usize,

    /// Batch size for processing (reserved for future batch API support)
    pub _batch_size: usize,

    /// Enable watch mode
    pub watch: bool,

    /// Enable progress bars
    pub show_progress: bool,

    /// Enable dry-run (no actual indexing)
    pub dry_run: bool,

    /// Enable resume from checkpoint
    pub resume: bool,

    /// Checkpoint file path
    pub checkpoint_path: PathBuf,

    /// Maximum retries per file
    pub max_retries: u32,

    /// Retry delay in seconds
    pub retry_delay_secs: u64,

    /// Chunk configuration
    pub chunk_config: ChunkConfig,

    /// Skip files already in checkpoint
    pub skip_processed: bool,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            parallel_workers: 4,
            _batch_size: 100,
            watch: false,
            show_progress: true,
            dry_run: false,
            resume: false,
            checkpoint_path: PathBuf::from(".otl_ingest_checkpoint.json"),
            max_retries: 3,
            retry_delay_secs: 2,
            chunk_config: ChunkConfig::default(),
            skip_processed: true,
        }
    }
}

// ============================================================================
// Checkpoint Management
// ============================================================================

/// Checkpoint for resuming ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestCheckpoint {
    /// When checkpoint was created
    pub created_at: DateTime<Utc>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Successfully processed files
    pub processed_files: HashSet<String>,

    /// Failed files with error messages
    pub failed_files: HashMap<String, String>,

    /// Files currently being processed
    pub in_progress_files: HashSet<String>,

    /// Total files discovered
    pub total_files: usize,

    /// Statistics
    pub stats: IngestStats,
}

impl IngestCheckpoint {
    /// Create a new checkpoint
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            processed_files: HashSet::new(),
            failed_files: HashMap::new(),
            in_progress_files: HashSet::new(),
            total_files: 0,
            stats: IngestStats::default(),
        }
    }

    /// Load from file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read checkpoint: {}", path.display()))?;
        let checkpoint: Self = serde_json::from_str(&content)
            .with_context(|| "Failed to parse checkpoint JSON")?;
        Ok(checkpoint)
    }

    /// Save to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write checkpoint: {}", path.display()))?;
        Ok(())
    }

    /// Mark file as processed
    pub fn mark_processed(&mut self, file_path: &str) {
        self.in_progress_files.remove(file_path);
        self.processed_files.insert(file_path.to_string());
        self.updated_at = Utc::now();
    }

    /// Mark file as failed
    pub fn mark_failed(&mut self, file_path: &str, error: &str) {
        self.in_progress_files.remove(file_path);
        self.failed_files.insert(file_path.to_string(), error.to_string());
        self.updated_at = Utc::now();
    }

    /// Mark file as in-progress (for future parallel checkpoint tracking)
    #[allow(dead_code)]
    pub fn mark_in_progress(&mut self, file_path: &str) {
        self.in_progress_files.insert(file_path.to_string());
        self.updated_at = Utc::now();
    }

    /// Check if file was already processed
    pub fn is_processed(&self, file_path: &str) -> bool {
        self.processed_files.contains(file_path)
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_files == 0 {
            return 0.0;
        }
        (self.processed_files.len() as f64 / self.total_files as f64) * 100.0
    }
}

impl Default for IngestCheckpoint {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Ingestion statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestStats {
    pub files_processed: usize,
    pub files_failed: usize,
    pub documents_created: usize,
    pub chunks_created: usize,
    pub entities_extracted: usize,
    pub relations_extracted: usize,
    pub embeddings_created: usize,
    pub total_bytes_processed: u64,
}

impl IngestStats {
    /// Update progress bar with stats
    pub fn to_status_string(&self) -> String {
        format!(
            "Files: {}/{} | Docs: {} | Chunks: {} | Entities: {} | Relations: {} | Embeddings: {}",
            self.files_processed,
            self.files_processed + self.files_failed,
            self.documents_created,
            self.chunks_created,
            self.entities_extracted,
            self.relations_extracted,
            self.embeddings_created
        )
    }
}

// ============================================================================
// File Discovery
// ============================================================================

/// Discover files to ingest
pub fn discover_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(files);
    }

    if !path.is_dir() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    info!("Scanning directory: {}", path.display());

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let file_path = entry.path();
            let file_type = FileType::from_path(file_path);

            // Only include supported file types
            if matches!(
                file_type,
                FileType::Pdf
                    | FileType::Docx
                    | FileType::Xlsx
                    | FileType::Xls
                    | FileType::PlainText
                    | FileType::Markdown
            ) {
                files.push(file_path.to_path_buf());
            }
        }
    }

    info!("Discovered {} files", files.len());
    Ok(files)
}

// ============================================================================
// Ingestion Pipeline
// ============================================================================

/// Main ingestion engine
pub struct IngestEngine {
    config: IngestConfig,
    checkpoint: IngestCheckpoint,
    parser_registry: Arc<ParserRegistry>,
    embedding_client: Arc<dyn EmbeddingClient>,
    ner: Arc<RuleBasedNer>,
    re: Arc<RuleBasedRe>,
}

impl IngestEngine {
    /// Create a new ingestion engine
    pub async fn new(config: IngestConfig, app_config: &AppConfig) -> Result<Self> {
        // Load or create checkpoint
        let checkpoint = if config.resume && config.checkpoint_path.exists() {
            info!("Loading checkpoint from: {}", config.checkpoint_path.display());
            IngestCheckpoint::load(&config.checkpoint_path)?
        } else {
            IngestCheckpoint::new()
        };

        // Initialize parser registry
        let parser_registry = Arc::new(ParserRegistry::with_defaults());

        // Initialize embedding client
        let embedding_client: Arc<dyn EmbeddingClient> = create_embedding_client(&app_config.llm)?
            .into();

        // Initialize extractors
        let ner = Arc::new(RuleBasedNer::new());
        let re = Arc::new(RuleBasedRe::new());

        Ok(Self {
            config,
            checkpoint,
            parser_registry,
            embedding_client,
            ner,
            re,
        })
    }

    /// Run ingestion on a path
    pub async fn ingest(&mut self, path: &Path) -> Result<()> {
        if self.config.watch {
            self.ingest_watch_mode(path).await
        } else {
            self.ingest_batch_mode(path).await
        }
    }

    /// Batch ingestion mode
    async fn ingest_batch_mode(&mut self, path: &Path) -> Result<()> {
        // Discover files
        let files = discover_files(path)?;
        self.checkpoint.total_files = files.len();

        if files.is_empty() {
            info!("No files to ingest");
            return Ok(());
        }

        // Filter out already processed files if resume is enabled
        let files_to_process: Vec<PathBuf> = if self.config.skip_processed {
            files
                .into_iter()
                .filter(|f| {
                    let path_str = f.display().to_string();
                    !self.checkpoint.is_processed(&path_str)
                })
                .collect()
        } else {
            files
        };

        if files_to_process.is_empty() {
            info!("All files already processed (use --no-skip-processed to reprocess)");
            self.print_final_stats();
            return Ok(());
        }

        info!(
            "Processing {} files ({} already completed)",
            files_to_process.len(),
            self.checkpoint.processed_files.len()
        );

        // Setup progress bars
        let multi_progress = MultiProgress::new();
        let main_progress = if self.config.show_progress {
            let pb = multi_progress.add(ProgressBar::new(files_to_process.len() as u64));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")?
                    .progress_chars("#>-"),
            );
            Some(pb)
        } else {
            None
        };

        // Process files in parallel
        let semaphore = Arc::new(Semaphore::new(self.config.parallel_workers));
        let mut tasks = Vec::new();

        for file_path in files_to_process {
            let permit = semaphore.clone().acquire_owned().await?;
            let parser_registry = self.parser_registry.clone();
            let embedding_client = self.embedding_client.clone();
            let ner = self.ner.clone();
            let re = self.re.clone();
            let chunk_config = self.config.chunk_config.clone();
            let dry_run = self.config.dry_run;
            let max_retries = self.config.max_retries;
            let retry_delay = Duration::from_secs(self.config.retry_delay_secs);

            let task = tokio::spawn(async move {
                let result = process_file_with_retry(
                    &file_path,
                    parser_registry,
                    embedding_client,
                    ner,
                    re,
                    &chunk_config,
                    dry_run,
                    max_retries,
                    retry_delay,
                )
                .await;

                drop(permit); // Release semaphore
                (file_path, result)
            });

            tasks.push(task);
        }

        // Collect results
        for task in tasks {
            match task.await {
                Ok((file_path, result)) => {
                    let path_str = file_path.display().to_string();

                    match result {
                        Ok(file_stats) => {
                            self.checkpoint.mark_processed(&path_str);
                            self.update_stats(&file_stats);

                            if let Some(pb) = &main_progress {
                                pb.inc(1);
                                pb.set_message(self.checkpoint.stats.to_status_string());
                            }

                            debug!("Processed: {}", path_str);
                        }
                        Err(e) => {
                            let error_msg = format!("{:#}", e);
                            self.checkpoint.mark_failed(&path_str, &error_msg);
                            self.checkpoint.stats.files_failed += 1;

                            error!("Failed to process {}: {}", path_str, error_msg);

                            if let Some(pb) = &main_progress {
                                pb.inc(1);
                            }
                        }
                    }

                    // Save checkpoint periodically
                    if self.checkpoint.processed_files.len() % 10 == 0 {
                        if let Err(e) = self.checkpoint.save(&self.config.checkpoint_path) {
                            warn!("Failed to save checkpoint: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Task panicked: {}", e);
                }
            }
        }

        if let Some(pb) = main_progress {
            pb.finish_with_message("Completed");
        }

        // Save final checkpoint
        self.checkpoint.save(&self.config.checkpoint_path)?;
        self.print_final_stats();

        Ok(())
    }

    /// Watch mode for continuous ingestion
    async fn ingest_watch_mode(&mut self, path: &Path) -> Result<()> {
        info!("Starting watch mode on: {}", path.display());

        // First, ingest existing files
        self.ingest_batch_mode(path).await?;

        // Setup file watcher
        let (tx, mut rx) = mpsc::channel(100);

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(path, RecursiveMode::Recursive)?;

        info!("Watching for changes... (Ctrl+C to stop)");

        // Process file events
        while let Some(event) = rx.recv().await {
            if let notify::EventKind::Create(_) | notify::EventKind::Modify(_) = event.kind {
                for event_path in event.paths {
                    if event_path.is_file() {
                        let file_type = FileType::from_path(&event_path);

                        // Only process supported file types
                        if matches!(
                            file_type,
                            FileType::Pdf
                                | FileType::Docx
                                | FileType::Xlsx
                                | FileType::Xls
                                | FileType::PlainText
                                | FileType::Markdown
                        ) {
                            info!("Detected new/modified file: {}", event_path.display());

                            // Process the file
                            let result = process_file_with_retry(
                                &event_path,
                                self.parser_registry.clone(),
                                self.embedding_client.clone(),
                                self.ner.clone(),
                                self.re.clone(),
                                &self.config.chunk_config,
                                self.config.dry_run,
                                self.config.max_retries,
                                Duration::from_secs(self.config.retry_delay_secs),
                            )
                            .await;

                            let path_str = event_path.display().to_string();

                            match result {
                                Ok(file_stats) => {
                                    self.checkpoint.mark_processed(&path_str);
                                    self.update_stats(&file_stats);
                                    info!("Successfully processed: {}", path_str);
                                }
                                Err(e) => {
                                    let error_msg = format!("{:#}", e);
                                    self.checkpoint.mark_failed(&path_str, &error_msg);
                                    error!("Failed to process {}: {}", path_str, error_msg);
                                }
                            }

                            // Save checkpoint
                            if let Err(e) = self.checkpoint.save(&self.config.checkpoint_path) {
                                warn!("Failed to save checkpoint: {}", e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Update statistics
    fn update_stats(&mut self, file_stats: &FileStats) {
        self.checkpoint.stats.files_processed += 1;
        self.checkpoint.stats.documents_created += file_stats.documents;
        self.checkpoint.stats.chunks_created += file_stats.chunks;
        self.checkpoint.stats.entities_extracted += file_stats.entities;
        self.checkpoint.stats.relations_extracted += file_stats.relations;
        self.checkpoint.stats.embeddings_created += file_stats.embeddings;
        self.checkpoint.stats.total_bytes_processed += file_stats.bytes_processed;
    }

    /// Print final statistics
    fn print_final_stats(&self) {
        println!("\n=== Ingestion Complete ===");
        println!(
            "Completion: {:.1}%",
            self.checkpoint.completion_percentage()
        );
        println!("Files processed: {}", self.checkpoint.stats.files_processed);
        println!("Files failed: {}", self.checkpoint.stats.files_failed);
        println!("Documents created: {}", self.checkpoint.stats.documents_created);
        println!("Chunks created: {}", self.checkpoint.stats.chunks_created);
        println!("Entities extracted: {}", self.checkpoint.stats.entities_extracted);
        println!("Relations extracted: {}", self.checkpoint.stats.relations_extracted);
        println!("Embeddings created: {}", self.checkpoint.stats.embeddings_created);
        println!(
            "Total bytes processed: {} MB",
            self.checkpoint.stats.total_bytes_processed / 1_024 / 1_024
        );

        if !self.checkpoint.failed_files.is_empty() {
            println!("\nFailed files:");
            for (file, error) in &self.checkpoint.failed_files {
                println!("  - {}: {}", file, error);
            }
        }
    }
}

// ============================================================================
// File Processing
// ============================================================================

/// Statistics for a single file
#[derive(Debug, Default)]
struct FileStats {
    documents: usize,
    chunks: usize,
    entities: usize,
    relations: usize,
    embeddings: usize,
    bytes_processed: u64,
}

/// Process a single file with retry logic
async fn process_file_with_retry(
    file_path: &Path,
    parser_registry: Arc<ParserRegistry>,
    embedding_client: Arc<dyn EmbeddingClient>,
    ner: Arc<RuleBasedNer>,
    re: Arc<RuleBasedRe>,
    chunk_config: &ChunkConfig,
    dry_run: bool,
    max_retries: u32,
    retry_delay: Duration,
) -> Result<FileStats> {
    let mut attempts = 0;
    let mut last_error = None;

    while attempts < max_retries {
        match process_file(
            file_path,
            parser_registry.clone(),
            embedding_client.clone(),
            ner.clone(),
            re.clone(),
            chunk_config,
            dry_run,
        )
        .await
        {
            Ok(stats) => return Ok(stats),
            Err(e) => {
                attempts += 1;
                last_error = Some(e);

                if attempts < max_retries {
                    warn!(
                        "Attempt {}/{} failed for {}: {}. Retrying...",
                        attempts,
                        max_retries,
                        file_path.display(),
                        last_error.as_ref().unwrap()
                    );
                    tokio::time::sleep(retry_delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
}

/// Process a single file through the complete pipeline
async fn process_file(
    file_path: &Path,
    parser_registry: Arc<ParserRegistry>,
    embedding_client: Arc<dyn EmbeddingClient>,
    ner: Arc<RuleBasedNer>,
    re: Arc<RuleBasedRe>,
    chunk_config: &ChunkConfig,
    dry_run: bool,
) -> Result<FileStats> {
    let mut stats = FileStats::default();

    // Get file size
    let metadata = fs::metadata(file_path)?;
    stats.bytes_processed = metadata.len();

    // Step 1: Parse document
    debug!("Parsing: {}", file_path.display());
    let parsed_doc = parser_registry
        .parse(file_path)
        .with_context(|| format!("Failed to parse: {}", file_path.display()))?;

    stats.documents = 1;

    // Step 2: Chunk document
    debug!("Chunking: {}", file_path.display());
    let chunks = chunk_document(&parsed_doc, chunk_config);
    stats.chunks = chunks.len();

    debug!("Created {} chunks", chunks.len());

    if dry_run {
        info!(
            "DRY RUN: Would process {} with {} chunks",
            file_path.display(),
            chunks.len()
        );
        return Ok(stats);
    }

    // Generate document ID
    let document_id = Uuid::new_v4();

    // Process each chunk
    for chunk in chunks {
        // Step 3: Extract entities and relations
        let entities = ner
            .extract(&chunk.content)
            .with_context(|| "Entity extraction failed")?;

        let relations = re
            .extract(&chunk.content, &entities)
            .with_context(|| "Relation extraction failed")?;

        stats.entities += entities.len();
        stats.relations += relations.len();

        // Step 4: Create embedding
        let embedding_vector = embedding_client
            .embed(&chunk.content)
            .await
            .with_context(|| "Embedding creation failed")?;

        stats.embeddings += 1;

        // Step 5: Store in vector database (in a real implementation)
        // For now, just create the vector structure
        let _vector = EmbeddingVector {
            id: Uuid::new_v4(),
            vector: embedding_vector,
            document_id,
            chunk_index: chunk.index,
            content: chunk.content.clone(),
        };

        // In a complete implementation, we would:
        // - Store vector in Qdrant
        // - Store entities/relations in SurrealDB
        // - Create graph connections
        //
        // For the CLI, we log what would be done
        debug!(
            "Chunk {}: {} entities, {} relations, embedding dim: {}",
            chunk.index,
            entities.len(),
            relations.len(),
            _vector.vector.len()
        );
    }

    info!(
        "Processed {}: {} chunks, {} entities, {} relations",
        file_path.display(),
        stats.chunks,
        stats.entities,
        stats.relations
    );

    Ok(stats)
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Validate path exists and is accessible
pub fn validate_path(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    if path.is_file() {
        // Check if file type is supported
        let file_type = FileType::from_path(path);
        if file_type == FileType::Unknown {
            anyhow::bail!("Unsupported file type: {}", path.display());
        }
    }

    Ok(())
}
