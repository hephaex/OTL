//! Excel document parser using calamine
//!
//! Extracts data from Excel files (XLSX, XLS) as tables.

use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};

use crate::{
    DocumentParseMetadata, DocumentParser, FileType, ParsedDocument, ParserError, Result, Table,
};

/// Excel document parser
pub struct ExcelParser {
    /// Sheets to parse (None = all sheets)
    pub sheet_filter: Option<Vec<String>>,
    /// Whether to treat first row as header
    pub first_row_header: bool,
}

impl ExcelParser {
    /// Create a new Excel parser with default settings
    pub fn new() -> Self {
        Self {
            sheet_filter: None,
            first_row_header: true,
        }
    }

    /// Filter specific sheets
    pub fn with_sheets(mut self, sheets: Vec<String>) -> Self {
        self.sheet_filter = Some(sheets);
        self
    }

    /// Set whether first row is treated as header
    pub fn with_first_row_header(mut self, enabled: bool) -> Self {
        self.first_row_header = enabled;
        self
    }

    /// Convert a Data cell to string
    fn cell_to_string(cell: &Data) -> String {
        match cell {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Float(f) => {
                // Format without unnecessary decimals
                if f.fract() == 0.0 {
                    format!("{}", *f as i64)
                } else {
                    format!("{f}")
                }
            }
            Data::Int(i) => format!("{i}"),
            Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Data::Error(e) => format!("#ERROR: {e:?}"),
            Data::DateTime(dt) => format!("{dt}"),
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
        }
    }

    /// Process a single sheet and return the table
    fn process_sheet(
        &self,
        sheet_name: &str,
        range: calamine::Range<Data>,
    ) -> (Table, String) {
        let mut table = Table::new();
        table.caption = Some(sheet_name.to_string());

        let mut rows_iter = range.rows();

        // Handle first row as header
        if self.first_row_header {
            if let Some(first_row) = rows_iter.next() {
                table.headers = first_row.iter().map(Self::cell_to_string).collect();
            }
        }

        // Process remaining rows, filtering out empty ones
        table.rows = rows_iter
            .map(|row| row.iter().map(Self::cell_to_string).collect::<Vec<_>>())
            .filter(|row_data: &Vec<String>| !row_data.iter().all(|s| s.is_empty()))
            .collect();

        // Generate content for this sheet
        let mut content = String::new();
        content.push_str(&format!("## {sheet_name}\n\n"));
        content.push_str(&table.to_markdown());
        content.push_str("\n\n");

        (table, content)
    }
}

impl Default for ExcelParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for ExcelParser {
    fn parse(&self, path: &Path) -> Result<ParsedDocument> {
        let mut workbook =
            open_workbook_auto(path).map_err(|e| ParserError::ExcelError(e.to_string()))?;

        let sheet_names = workbook.sheet_names().to_vec();

        let mut content = String::new();
        let mut tables = Vec::new();

        for sheet_name in &sheet_names {
            // Apply sheet filter if set
            if let Some(filter) = &self.sheet_filter {
                if !filter.contains(sheet_name) {
                    continue;
                }
            }

            let Ok(range) = workbook.worksheet_range(sheet_name) else {
                continue;
            };

            let (table, sheet_content) = self.process_sheet(sheet_name, range);
            content.push_str(&sheet_content);
            tables.push(table);
        }

        let mut metadata = DocumentParseMetadata::default();

        // Get title from first sheet name
        if let Some(first_sheet) = sheet_names.first() {
            metadata.title = Some(first_sheet.clone());
        }

        // Store sheet count in custom metadata
        metadata
            .custom
            .insert("sheet_count".to_string(), sheet_names.len().to_string());

        Ok(ParsedDocument {
            file_path: path.display().to_string(),
            file_type: FileType::Xlsx,
            content,
            sections: Vec::new(),
            tables,
            metadata,
        })
    }

    fn supported_types(&self) -> &[FileType] {
        &[FileType::Xlsx, FileType::Xls]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excel_parser_creation() {
        let parser = ExcelParser::new();
        assert!(parser.first_row_header);
        assert!(parser.sheet_filter.is_none());

        let parser = parser.with_first_row_header(false);
        assert!(!parser.first_row_header);
    }

    #[test]
    fn test_sheet_filter() {
        let parser = ExcelParser::new().with_sheets(vec!["Sheet1".to_string()]);
        assert_eq!(parser.sheet_filter, Some(vec!["Sheet1".to_string()]));
    }

    #[test]
    fn test_cell_to_string() {
        use calamine::Data;
        assert_eq!(ExcelParser::cell_to_string(&Data::Empty), "");
        assert_eq!(
            ExcelParser::cell_to_string(&Data::String("test".to_string())),
            "test"
        );
        assert_eq!(ExcelParser::cell_to_string(&Data::Int(42)), "42");
        assert_eq!(ExcelParser::cell_to_string(&Data::Float(3.5)), "3.5");
        assert_eq!(ExcelParser::cell_to_string(&Data::Float(10.0)), "10");
        assert_eq!(ExcelParser::cell_to_string(&Data::Bool(true)), "TRUE");
    }

    #[test]
    fn test_supported_types() {
        let parser = ExcelParser::new();
        assert!(parser.can_parse(FileType::Xlsx));
        assert!(parser.can_parse(FileType::Xls));
        assert!(!parser.can_parse(FileType::Pdf));
    }
}
