//! OTL Parser - Document parsing for various file formats
//!
//! Supports parsing of:
//! - PDF documents
//! - Microsoft Word (DOCX)
//! - Microsoft Excel (XLSX, XLS)
//! - Markdown files
//! - Plain text files
//!
//! Each parser implements the `DocumentParser` trait and produces
//! a `ParsedDocument` that can be further processed for chunking
//! and knowledge extraction.

use std::path::Path;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during document parsing
#[derive(Error, Debug)]
pub enum ParserError {
    /// File format is not supported
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    /// IO error while reading the file
    #[error("IO error reading file: {path}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// PDF parsing error
    #[error("PDF parsing error: {0}")]
    PdfError(String),

    /// DOCX parsing error
    #[error("DOCX parsing error: {0}")]
    DocxError(String),

    /// Excel parsing error
    #[error("Excel parsing error: {0}")]
    ExcelError(String),

    /// OCR processing error
    #[error("OCR engine failed: {0}")]
    OcrError(String),

    /// File is encrypted and cannot be parsed
    #[error("File is encrypted and requires a password: {0}")]
    EncryptedFile(String),

    /// File is corrupted or malformed
    #[error("File is corrupted or malformed: {0}")]
    CorruptedFile(String),

    /// Encoding error
    #[error("Text encoding error: {0}")]
    EncodingError(String),

    /// Timeout during parsing
    #[error("Parsing timeout exceeded: {0}ms")]
    Timeout(u64),
}

pub type Result<T> = std::result::Result<T, ParserError>;

// ============================================================================
// Parsed Document Types
// ============================================================================

/// A parsed document with extracted content
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    /// Original file path
    pub file_path: String,

    /// Detected file type
    pub file_type: FileType,

    /// Extracted text content
    pub content: String,

    /// Structured sections (if available)
    pub sections: Vec<DocumentSection>,

    /// Extracted tables
    pub tables: Vec<Table>,

    /// Metadata extracted from the document
    pub metadata: DocumentParseMetadata,
}

impl ParsedDocument {
    /// Create a new parsed document
    pub fn new(file_path: impl Into<String>, file_type: FileType) -> Self {
        Self {
            file_path: file_path.into(),
            file_type,
            content: String::new(),
            sections: Vec::new(),
            tables: Vec::new(),
            metadata: DocumentParseMetadata::default(),
        }
    }

    /// Set content
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Add a section
    pub fn add_section(&mut self, section: DocumentSection) {
        self.sections.push(section);
    }

    /// Add a table
    pub fn add_table(&mut self, table: Table) {
        self.tables.push(table);
    }

    /// Get total character count
    pub fn char_count(&self) -> usize {
        self.content.len()
    }

    /// Get total word count (approximate)
    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }
}

/// Supported file types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Pdf,
    Docx,
    Xlsx,
    Xls,
    Pptx,
    Markdown,
    PlainText,
    Html,
    Unknown,
}

impl FileType {
    /// Detect file type from extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "pdf" => Self::Pdf,
            "docx" => Self::Docx,
            "xlsx" => Self::Xlsx,
            "xls" => Self::Xls,
            "pptx" => Self::Pptx,
            "md" | "markdown" => Self::Markdown,
            "txt" => Self::PlainText,
            "html" | "htm" => Self::Html,
            _ => Self::Unknown,
        }
    }

    /// Detect file type from path
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(Self::Unknown)
    }

    /// Get MIME type
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::Xls => "application/vnd.ms-excel",
            Self::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::Markdown => "text/markdown",
            Self::PlainText => "text/plain",
            Self::Html => "text/html",
            Self::Unknown => "application/octet-stream",
        }
    }
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pdf => write!(f, "pdf"),
            Self::Docx => write!(f, "docx"),
            Self::Xlsx => write!(f, "xlsx"),
            Self::Xls => write!(f, "xls"),
            Self::Pptx => write!(f, "pptx"),
            Self::Markdown => write!(f, "markdown"),
            Self::PlainText => write!(f, "text"),
            Self::Html => write!(f, "html"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// A section within a document
#[derive(Debug, Clone)]
pub struct DocumentSection {
    /// Section heading/title
    pub title: Option<String>,

    /// Heading level (1 = top level)
    pub level: u8,

    /// Section content
    pub content: String,

    /// Page number where section starts
    pub start_page: Option<u32>,

    /// Child sections
    pub children: Vec<DocumentSection>,
}

impl DocumentSection {
    /// Create a new section
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            title: None,
            level: 1,
            content: content.into(),
            start_page: None,
            children: Vec::new(),
        }
    }

    /// Set title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set level
    pub fn with_level(mut self, level: u8) -> Self {
        self.level = level;
        self
    }

    /// Set start page
    pub fn with_start_page(mut self, page: u32) -> Self {
        self.start_page = Some(page);
        self
    }
}

/// A table extracted from a document
#[derive(Debug, Clone)]
pub struct Table {
    /// Table caption/title (if any)
    pub caption: Option<String>,

    /// Column headers
    pub headers: Vec<String>,

    /// Table rows
    pub rows: Vec<Vec<String>>,

    /// Page number where table appears
    pub page: Option<u32>,
}

impl Table {
    /// Create a new table
    pub fn new() -> Self {
        Self {
            caption: None,
            headers: Vec::new(),
            rows: Vec::new(),
            page: None,
        }
    }

    /// Add headers
    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.headers = headers;
        self
    }

    /// Add a row
    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    /// Get number of columns
    pub fn num_columns(&self) -> usize {
        self.headers
            .len()
            .max(self.rows.first().map(|r| r.len()).unwrap_or(0))
    }

    /// Get number of rows
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    /// Convert to markdown format
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Headers
        if !self.headers.is_empty() {
            md.push('|');
            for h in &self.headers {
                md.push_str(&format!(" {h} |"));
            }
            md.push('\n');

            // Separator
            md.push('|');
            for _ in &self.headers {
                md.push_str(" --- |");
            }
            md.push('\n');
        }

        // Rows
        for row in &self.rows {
            md.push('|');
            for cell in row {
                md.push_str(&format!(" {cell} |"));
            }
            md.push('\n');
        }

        md
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata extracted during parsing
#[derive(Debug, Clone, Default)]
pub struct DocumentParseMetadata {
    /// Document title
    pub title: Option<String>,

    /// Author
    pub author: Option<String>,

    /// Creation date
    pub created: Option<String>,

    /// Last modified date
    pub modified: Option<String>,

    /// Number of pages
    pub page_count: Option<u32>,

    /// Word count
    pub word_count: Option<u32>,

    /// Language detected
    pub language: Option<String>,

    /// Whether OCR was used
    pub ocr_applied: bool,

    /// Additional custom metadata
    pub custom: std::collections::HashMap<String, String>,
}

// ============================================================================
// Parser Trait
// ============================================================================

/// Trait for document parsers
pub trait DocumentParser: Send + Sync {
    /// Parse a document from a file path
    fn parse(&self, path: &Path) -> Result<ParsedDocument>;

    /// Get supported file types
    fn supported_types(&self) -> &[FileType];

    /// Check if this parser can handle a file type
    fn can_parse(&self, file_type: FileType) -> bool {
        self.supported_types().contains(&file_type)
    }
}

// ============================================================================
// Chunking
// ============================================================================

/// Configuration for document chunking
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Target chunk size in characters
    pub chunk_size: usize,

    /// Overlap between chunks in characters
    pub overlap: usize,

    /// Minimum chunk size (won't split below this)
    pub min_chunk_size: usize,

    /// Respect section boundaries when chunking
    pub respect_sections: bool,

    /// Respect paragraph boundaries
    pub respect_paragraphs: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            overlap: 200,
            min_chunk_size: 100,
            respect_sections: true,
            respect_paragraphs: true,
        }
    }
}

/// A chunk of text from a document
#[derive(Debug, Clone)]
pub struct TextChunk {
    /// Chunk content
    pub content: String,

    /// Chunk index within the document
    pub index: u32,

    /// Starting character offset in original document
    pub start_offset: usize,

    /// Ending character offset
    pub end_offset: usize,

    /// Page number (if applicable)
    pub page: Option<u32>,

    /// Section name (if applicable)
    pub section: Option<String>,
}

/// Chunk a parsed document into smaller pieces
pub fn chunk_document(doc: &ParsedDocument, config: &ChunkConfig) -> Vec<TextChunk> {
    let mut chunks = Vec::new();
    let mut index = 0u32;

    if config.respect_sections && !doc.sections.is_empty() {
        // Chunk by sections
        for section in &doc.sections {
            let section_chunks = chunk_text(
                &section.content,
                config,
                section.start_page,
                section.title.clone(),
            );

            for mut chunk in section_chunks {
                chunk.index = index;
                index += 1;
                chunks.push(chunk);
            }
        }
    } else {
        // Chunk the full content
        chunks = chunk_text(&doc.content, config, None, None);
        for (i, chunk) in chunks.iter_mut().enumerate() {
            chunk.index = i as u32;
        }
    }

    chunks
}

/// Chunk a text string
fn chunk_text(
    text: &str,
    config: &ChunkConfig,
    page: Option<u32>,
    section: Option<String>,
) -> Vec<TextChunk> {
    let mut chunks = Vec::new();

    if text.len() <= config.chunk_size {
        // Small enough to be a single chunk
        chunks.push(TextChunk {
            content: text.to_string(),
            index: 0,
            start_offset: 0,
            end_offset: text.len(),
            page,
            section,
        });
        return chunks;
    }

    let mut start = 0;

    while start < text.len() {
        let end = (start + config.chunk_size).min(text.len());

        // Find a good break point (end of sentence or paragraph)
        let actual_end = if config.respect_paragraphs {
            find_break_point(text, start, end)
        } else {
            end
        };

        let chunk_content = &text[start..actual_end];

        if chunk_content.len() >= config.min_chunk_size {
            chunks.push(TextChunk {
                content: chunk_content.to_string(),
                index: 0,
                start_offset: start,
                end_offset: actual_end,
                page,
                section: section.clone(),
            });
        }

        // Move start with overlap
        if actual_end >= text.len() {
            break;
        }

        start = if actual_end > config.overlap {
            actual_end - config.overlap
        } else {
            actual_end
        };
    }

    chunks
}

/// Find a good break point near the target position
fn find_break_point(text: &str, _start: usize, target: usize) -> usize {
    // Search window
    let search_start = if target > 100 { target - 100 } else { target };
    let search_end = (target + 100).min(text.len());

    let search_text = &text[search_start..search_end];

    // Look for paragraph break first
    if let Some(pos) = search_text.rfind("\n\n") {
        return search_start + pos + 2;
    }

    // Look for sentence end
    for pattern in [". ", "。", "! ", "? "] {
        if let Some(pos) = search_text.rfind(pattern) {
            return search_start + pos + pattern.len();
        }
    }

    // Look for line break
    if let Some(pos) = search_text.rfind('\n') {
        return search_start + pos + 1;
    }

    // Fall back to target
    target.min(text.len())
}

// ============================================================================
// Parser Registry
// ============================================================================

/// Registry of available parsers
pub struct ParserRegistry {
    parsers: Vec<Box<dyn DocumentParser>>,
}

impl ParserRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            parsers: Vec::new(),
        }
    }

    /// Register a parser
    pub fn register<P: DocumentParser + 'static>(&mut self, parser: P) {
        self.parsers.push(Box::new(parser));
    }

    /// Find a parser for a file type
    pub fn find_parser(&self, file_type: FileType) -> Option<&dyn DocumentParser> {
        self.parsers
            .iter()
            .find(|p| p.can_parse(file_type))
            .map(|p| p.as_ref())
    }

    /// Parse a file using the appropriate parser
    pub fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        let file_type = FileType::from_path(path);

        if file_type == FileType::Unknown {
            return Err(ParserError::UnsupportedFormat(
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("none")
                    .to_string(),
            ));
        }

        let parser = self
            .find_parser(file_type)
            .ok_or_else(|| ParserError::UnsupportedFormat(file_type.to_string()))?;

        parser.parse(path)
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Placeholder Parsers (to be implemented with actual libraries)
// ============================================================================

/// Plain text parser
pub struct PlainTextParser;

impl DocumentParser for PlainTextParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        let content = std::fs::read_to_string(path).map_err(|e| ParserError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        Ok(
            ParsedDocument::new(path.display().to_string(), FileType::PlainText)
                .with_content(content),
        )
    }

    fn supported_types(&self) -> &[FileType] {
        &[FileType::PlainText, FileType::Markdown]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(FileType::from_extension("pdf"), FileType::Pdf);
        assert_eq!(FileType::from_extension("PDF"), FileType::Pdf);
        assert_eq!(FileType::from_extension("docx"), FileType::Docx);
        assert_eq!(FileType::from_extension("xlsx"), FileType::Xlsx);
        assert_eq!(FileType::from_extension("md"), FileType::Markdown);
        assert_eq!(FileType::from_extension("unknown"), FileType::Unknown);
    }

    #[test]
    fn test_table_to_markdown() {
        let mut table = Table::new().with_headers(vec!["Name".to_string(), "Age".to_string()]);
        table.add_row(vec!["Alice".to_string(), "30".to_string()]);
        table.add_row(vec!["Bob".to_string(), "25".to_string()]);

        let md = table.to_markdown();
        assert!(md.contains("| Name | Age |"));
        assert!(md.contains("| Alice | 30 |"));
    }

    #[test]
    fn test_chunking() {
        let doc = ParsedDocument::new("test.txt", FileType::PlainText)
            .with_content("This is a test. ".repeat(100));

        let config = ChunkConfig {
            chunk_size: 200,
            overlap: 50,
            ..Default::default()
        };

        let chunks = chunk_document(&doc, &config);
        assert!(!chunks.is_empty());

        // Check overlap exists
        for window in chunks.windows(2) {
            let end_of_first = &window[0].content[window[0].content.len().saturating_sub(50)..];
            let start_of_second = &window[1].content[..50.min(window[1].content.len())];

            // There should be some overlap
            assert!(
                end_of_first.contains(&start_of_second[..10.min(start_of_second.len())])
                    || window[0].end_offset >= window[1].start_offset
            );
        }
    }

    #[test]
    fn test_document_section_builder() {
        let section = DocumentSection::new("Content here")
            .with_title("Introduction")
            .with_level(1)
            .with_start_page(5);

        assert_eq!(section.title, Some("Introduction".to_string()));
        assert_eq!(section.level, 1);
        assert_eq!(section.start_page, Some(5));
    }
}
