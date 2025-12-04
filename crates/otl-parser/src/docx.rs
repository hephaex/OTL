//! DOCX document parser using docx-rs
//!
//! Extracts text content and structure from Microsoft Word documents.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use docx_rs::read_docx;

use crate::{
    DocumentParseMetadata, DocumentParser, DocumentSection, FileType, ParsedDocument, ParserError,
    Result, Table,
};

/// DOCX document parser
pub struct DocxParser {
    /// Whether to preserve formatting hints
    pub preserve_formatting: bool,
}

impl DocxParser {
    /// Create a new DOCX parser with default settings
    pub fn new() -> Self {
        Self {
            preserve_formatting: false,
        }
    }

    /// Enable formatting preservation
    pub fn with_formatting(mut self, enabled: bool) -> Self {
        self.preserve_formatting = enabled;
        self
    }
}

impl Default for DocxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for DocxParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        let mut file = File::open(path).map_err(|e| ParserError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| ParserError::IoError {
                path: path.display().to_string(),
                source: e,
            })?;

        let docx = read_docx(&buf).map_err(|e| ParserError::DocxError(e.to_string()))?;

        let mut content = String::new();
        let mut sections = Vec::new();
        let mut tables = Vec::new();
        let mut current_section_content = String::new();
        let mut current_section_title: Option<String> = None;
        let mut current_section_level = 1u8;

        // Process document body
        for child in docx.document.children {
            match child {
                docx_rs::DocumentChild::Paragraph(para) => {
                    let mut para_text = String::new();
                    let mut is_heading = false;
                    let mut heading_level = 0u8;

                    // Check paragraph style for heading
                    if let Some(style) = &para.property.style {
                        let style_id = &style.val;
                        if style_id.starts_with("Heading") || style_id.starts_with("heading") {
                            is_heading = true;
                            heading_level = style_id
                                .chars()
                                .last()
                                .and_then(|c| c.to_digit(10))
                                .unwrap_or(1) as u8;
                        }
                    }

                    // Extract text from paragraph
                    for child in &para.children {
                        if let docx_rs::ParagraphChild::Run(run) = child {
                            for run_child in &run.children {
                                if let docx_rs::RunChild::Text(text) = run_child {
                                    para_text.push_str(&text.text);
                                }
                            }
                        }
                    }

                    if is_heading && !para_text.trim().is_empty() {
                        // Save previous section
                        if !current_section_content.trim().is_empty()
                            || current_section_title.is_some()
                        {
                            sections.push(DocumentSection {
                                title: current_section_title.take(),
                                level: current_section_level,
                                content: current_section_content.trim().to_string(),
                                start_page: None,
                                children: Vec::new(),
                            });
                            current_section_content.clear();
                        }

                        current_section_title = Some(para_text.trim().to_string());
                        current_section_level = heading_level;
                    } else {
                        current_section_content.push_str(&para_text);
                        current_section_content.push('\n');
                    }

                    content.push_str(&para_text);
                    content.push('\n');
                }
                docx_rs::DocumentChild::Table(tbl) => {
                    let mut table = Table::new();
                    let mut is_first_row = true;

                    for row in &tbl.rows {
                        let docx_rs::TableChild::TableRow(tr) = row;
                        let mut row_cells = Vec::new();

                        for cell in &tr.cells {
                            let docx_rs::TableRowChild::TableCell(tc) = cell;
                            let mut cell_text = String::new();

                            for child in &tc.children {
                                if let docx_rs::TableCellContent::Paragraph(para) = child {
                                    for para_child in &para.children {
                                        if let docx_rs::ParagraphChild::Run(run) = para_child {
                                            for run_child in &run.children {
                                                if let docx_rs::RunChild::Text(text) = run_child {
                                                    cell_text.push_str(&text.text);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            row_cells.push(cell_text.trim().to_string());
                        }

                        if is_first_row {
                            table.headers = row_cells;
                            is_first_row = false;
                        } else {
                            table.rows.push(row_cells);
                        }
                    }

                    // Add table to content as markdown
                    content.push_str(&table.to_markdown());
                    content.push('\n');
                    tables.push(table);
                }
                _ => {}
            }
        }

        // Add final section
        if !current_section_content.trim().is_empty() || current_section_title.is_some() {
            sections.push(DocumentSection {
                title: current_section_title,
                level: current_section_level,
                content: current_section_content.trim().to_string(),
                start_page: None,
                children: Vec::new(),
            });
        }

        let mut metadata = DocumentParseMetadata::default();
        metadata.word_count = Some(content.split_whitespace().count() as u32);

        // Try to get title from first heading
        if let Some(first_section) = sections.first() {
            if let Some(title) = &first_section.title {
                metadata.title = Some(title.clone());
            }
        }

        Ok(ParsedDocument {
            file_path: path.display().to_string(),
            file_type: FileType::Docx,
            content,
            sections,
            tables,
            metadata,
        })
    }

    fn supported_types(&self) -> &[FileType] {
        &[FileType::Docx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docx_parser_creation() {
        let parser = DocxParser::new();
        assert!(!parser.preserve_formatting);

        let parser = parser.with_formatting(true);
        assert!(parser.preserve_formatting);
    }

    #[test]
    fn test_supported_types() {
        let parser = DocxParser::new();
        assert!(parser.can_parse(FileType::Docx));
        assert!(!parser.can_parse(FileType::Pdf));
    }
}
