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

impl DocxParser {
    /// Extract text from a paragraph's runs
    fn extract_paragraph_text(para: &docx_rs::Paragraph) -> String {
        para.children
            .iter()
            .filter_map(|child| {
                if let docx_rs::ParagraphChild::Run(run) = child {
                    Some(run)
                } else {
                    None
                }
            })
            .flat_map(|run| &run.children)
            .filter_map(|run_child| {
                if let docx_rs::RunChild::Text(text) = run_child {
                    Some(text.text.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if paragraph is a heading and extract level
    fn check_heading_style(para: &docx_rs::Paragraph) -> Option<u8> {
        let style = para.property.style.as_ref()?;
        let style_id = &style.val;

        if !style_id.starts_with("Heading") && !style_id.starts_with("heading") {
            return None;
        }

        Some(
            style_id
                .chars()
                .last()
                .and_then(|c| c.to_digit(10))
                .unwrap_or(1) as u8,
        )
    }

    /// Extract text from a table cell
    fn extract_cell_text(cell: &docx_rs::TableCell) -> String {
        cell.children
            .iter()
            .filter_map(|child| {
                if let docx_rs::TableCellContent::Paragraph(para) = child {
                    Some(para)
                } else {
                    None
                }
            })
            .map(Self::extract_paragraph_text)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Save current section if it has content
    fn save_section_if_needed(
        sections: &mut Vec<DocumentSection>,
        title: &mut Option<String>,
        content: &mut String,
        level: u8,
    ) {
        if content.trim().is_empty() && title.is_none() {
            return;
        }

        sections.push(DocumentSection {
            title: title.take(),
            level,
            content: content.trim().to_string(),
            start_page: None,
            children: Vec::new(),
        });
        content.clear();
    }

    /// Process a table and extract its data
    fn process_table(tbl: &docx_rs::Table) -> Table {
        let mut table = Table::new();
        let mut rows_iter = tbl.rows.iter();

        // First row as headers
        if let Some(docx_rs::TableChild::TableRow(first_row)) = rows_iter.next() {
            table.headers = first_row
                .cells
                .iter()
                .map(|cell| {
                    let docx_rs::TableRowChild::TableCell(tc) = cell;
                    Self::extract_cell_text(tc).trim().to_string()
                })
                .collect();
        }

        // Remaining rows as data
        for row in rows_iter {
            let docx_rs::TableChild::TableRow(tr) = row;
            let row_cells: Vec<String> = tr
                .cells
                .iter()
                .map(|cell| {
                    let docx_rs::TableRowChild::TableCell(tc) = cell;
                    Self::extract_cell_text(tc).trim().to_string()
                })
                .collect();
            table.rows.push(row_cells);
        }

        table
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
                    let para_text = Self::extract_paragraph_text(&para);
                    let heading_level = Self::check_heading_style(&para);

                    // Handle heading paragraphs
                    let Some(level) = heading_level else {
                        // Regular paragraph - add to current section
                        current_section_content.push_str(&para_text);
                        current_section_content.push('\n');
                        content.push_str(&para_text);
                        content.push('\n');
                        continue;
                    };

                    // Skip empty headings
                    if para_text.trim().is_empty() {
                        content.push_str(&para_text);
                        content.push('\n');
                        continue;
                    }

                    // Save previous section and start new one
                    Self::save_section_if_needed(
                        &mut sections,
                        &mut current_section_title,
                        &mut current_section_content,
                        current_section_level,
                    );

                    current_section_title = Some(para_text.trim().to_string());
                    current_section_level = level;

                    content.push_str(&para_text);
                    content.push('\n');
                }
                docx_rs::DocumentChild::Table(tbl) => {
                    let table = Self::process_table(&tbl);

                    // Add table to content as markdown
                    content.push_str(&table.to_markdown());
                    content.push('\n');
                    tables.push(table);
                }
                _ => {}
            }
        }

        // Add final section
        Self::save_section_if_needed(
            &mut sections,
            &mut current_section_title,
            &mut current_section_content,
            current_section_level,
        );

        // Try to get title from first heading
        let title = sections.first().and_then(|s| s.title.clone());

        let metadata = DocumentParseMetadata {
            word_count: Some(content.split_whitespace().count() as u32),
            title,
            ..Default::default()
        };

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
