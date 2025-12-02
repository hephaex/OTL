//! PDF document parser using pdf-extract
//!
//! Extracts text content from PDF files, handling multi-page documents
//! and basic structure detection.

use std::path::Path;

use crate::{
    DocumentParseMetadata, DocumentParser, DocumentSection, FileType, ParsedDocument, ParserError,
    Result,
};

/// PDF document parser
pub struct PdfParser {
    /// Whether to extract tables (experimental)
    pub extract_tables: bool,
}

impl PdfParser {
    /// Create a new PDF parser with default settings
    pub fn new() -> Self {
        Self {
            extract_tables: false,
        }
    }

    /// Enable table extraction (experimental)
    pub fn with_table_extraction(mut self, enabled: bool) -> Self {
        self.extract_tables = enabled;
        self
    }

    /// Extract text from a PDF file
    fn extract_text(&self, path: &Path) -> Result<(String, Option<u32>)> {
        let bytes = std::fs::read(path).map_err(|e| ParserError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        let text = pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|e| ParserError::PdfError(e.to_string()))?;

        // Try to estimate page count from the extracted content
        // This is a rough estimate based on form feed characters
        let page_count = text.matches('\x0C').count() as u32;
        let page_count = if page_count > 0 {
            Some(page_count + 1)
        } else {
            None
        };

        Ok((text, page_count))
    }

    /// Parse sections from extracted text
    fn parse_sections(&self, text: &str) -> Vec<DocumentSection> {
        let mut sections = Vec::new();
        let mut current_content = String::new();
        let mut current_page = 1u32;

        for line in text.lines() {
            // Check for form feed (page break)
            if line.contains('\x0C') {
                current_page += 1;
            }

            // Detect potential section headers (simple heuristics)
            // - All caps lines
            // - Lines ending with specific patterns
            // - Numbered sections
            let is_header = self.is_potential_header(line);

            if is_header && !current_content.trim().is_empty() {
                // Save previous section
                sections.push(DocumentSection {
                    title: None,
                    level: 1,
                    content: current_content.trim().to_string(),
                    start_page: Some(current_page),
                    children: Vec::new(),
                });
                current_content.clear();
            }

            if is_header {
                // Start new section with this as title
                sections.push(DocumentSection {
                    title: Some(line.trim().to_string()),
                    level: self.detect_heading_level(line),
                    content: String::new(),
                    start_page: Some(current_page),
                    children: Vec::new(),
                });
            } else {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Add remaining content
        if !current_content.trim().is_empty() {
            if let Some(last) = sections.last_mut() {
                last.content = current_content.trim().to_string();
            } else {
                sections.push(DocumentSection {
                    title: None,
                    level: 1,
                    content: current_content.trim().to_string(),
                    start_page: Some(1),
                    children: Vec::new(),
                });
            }
        }

        sections
    }

    /// Check if a line might be a section header
    fn is_potential_header(&self, line: &str) -> bool {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.len() > 100 {
            return false;
        }

        // Check for numbered sections (e.g., "1. Introduction", "제1조")
        let numbered_pattern = trimmed.starts_with(|c: char| c.is_numeric())
            || trimmed.starts_with("제")
            || trimmed.starts_with("Chapter")
            || trimmed.starts_with("Section");

        // Check for all caps (common in headers)
        let all_caps = trimmed.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())
            && trimmed.chars().any(|c| c.is_alphabetic());

        // Check for Korean section markers
        let korean_section = trimmed.contains("조") && trimmed.len() < 50;

        numbered_pattern || all_caps || korean_section
    }

    /// Detect heading level from line content
    fn detect_heading_level(&self, line: &str) -> u8 {
        let trimmed = line.trim();

        // Check for chapter-level markers
        if trimmed.starts_with("Chapter")
            || trimmed.starts_with("제")
            || trimmed.starts_with("CHAPTER")
        {
            return 1;
        }

        // Check for section markers
        if trimmed.starts_with("Section") || trimmed.starts_with("절") {
            return 2;
        }

        // Check for numbered depth (1.1.1 = level 3)
        let dots = trimmed
            .chars()
            .take_while(|c| c.is_numeric() || *c == '.')
            .filter(|c| *c == '.')
            .count();

        if dots > 0 {
            return (dots + 1).min(6) as u8;
        }

        1
    }
}

impl Default for PdfParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for PdfParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        let (text, page_count) = self.extract_text(path)?;

        let sections = self.parse_sections(&text);

        let mut metadata = DocumentParseMetadata::default();
        metadata.page_count = page_count;

        let mut doc = ParsedDocument {
            file_path: path.display().to_string(),
            file_type: FileType::Pdf,
            content: text,
            sections,
            tables: Vec::new(),
            metadata,
        };

        // Try to extract title from first section or first line
        if let Some(first_section) = doc.sections.first() {
            if let Some(title) = &first_section.title {
                doc.metadata.title = Some(title.clone());
            }
        } else if let Some(first_line) = doc.content.lines().next() {
            let trimmed = first_line.trim();
            if !trimmed.is_empty() && trimmed.len() < 200 {
                doc.metadata.title = Some(trimmed.to_string());
            }
        }

        Ok(doc)
    }

    fn supported_types(&self) -> &[FileType] {
        &[FileType::Pdf]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_parser_creation() {
        let parser = PdfParser::new();
        assert!(!parser.extract_tables);

        let parser = parser.with_table_extraction(true);
        assert!(parser.extract_tables);
    }

    #[test]
    fn test_header_detection() {
        let parser = PdfParser::new();

        assert!(parser.is_potential_header("1. Introduction"));
        assert!(parser.is_potential_header("CHAPTER ONE"));
        assert!(parser.is_potential_header("제1조 총칙"));
        assert!(!parser.is_potential_header("This is a normal paragraph."));
        assert!(!parser.is_potential_header(""));
    }

    #[test]
    fn test_heading_level_detection() {
        let parser = PdfParser::new();

        assert_eq!(parser.detect_heading_level("Chapter 1"), 1);
        assert_eq!(parser.detect_heading_level("Section 1.1"), 2);
        assert_eq!(parser.detect_heading_level("1.1.1 Details"), 3);
    }

    #[test]
    fn test_supported_types() {
        let parser = PdfParser::new();
        assert!(parser.can_parse(FileType::Pdf));
        assert!(!parser.can_parse(FileType::Docx));
    }
}
