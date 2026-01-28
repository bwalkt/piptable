use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // Column separator patterns
    static ref COLUMN_SEPARATOR: Regex = Regex::new(r"(\s{2,}|\t+|\|)+").unwrap();
    
    // Row patterns
    static ref HORIZONTAL_RULE: Regex = Regex::new(r"^[-=]{3,}$").unwrap();
    static ref NUMERIC_PATTERN: Regex = Regex::new(r"\d+(\.\d+)?").unwrap();
    static ref HEADER_PATTERN: Regex = Regex::new(r"^[A-Z][A-Z\s]+$").unwrap();
}

#[derive(Debug, Clone)]
pub struct TableRegion {
    pub rows: Vec<Vec<String>>,
    pub start_line: usize,
    pub end_line: usize,
}

pub struct TableDetector {
    min_rows: usize,
    min_cols: usize,
    column_threshold: usize,
}

impl Default for TableDetector {
    fn default() -> Self {
        Self {
            min_rows: 2,
            min_cols: 2,
            column_threshold: 2,
        }
    }
}

impl TableDetector {
    pub fn new(min_rows: usize, min_cols: usize) -> Self {
        Self {
            min_rows,
            min_cols,
            column_threshold: 2,
        }
    }
    
    pub fn detect_tables(&self, text: &str) -> Vec<TableRegion> {
        let lines: Vec<&str> = text.lines().collect();
        let mut tables = Vec::new();
        let mut i = 0;
        
        while i < lines.len() {
            if let Some(table) = self.extract_table_at(&lines, i) {
                i = table.end_line + 1;
                tables.push(table);
            } else {
                i += 1;
            }
        }
        
        tables
    }
    
    fn extract_table_at(&self, lines: &[&str], start: usize) -> Option<TableRegion> {
        let mut rows = Vec::new();
        let mut consistent_columns = None;
        let mut end_line = start;
        
        for (idx, line) in lines.iter().enumerate().skip(start) {
            let line = line.trim();
            
            // Skip empty lines between potential table rows
            if line.is_empty() && rows.is_empty() {
                continue;
            }
            
            // Check if it's a horizontal rule
            if HORIZONTAL_RULE.is_match(line) {
                continue;
            }
            
            // Try to parse as table row
            if let Some(columns) = self.parse_row(line) {
                let col_count = columns.len();
                
                // Check column consistency
                if let Some(expected) = consistent_columns {
                    if col_count != expected {
                        // Column count changed, might be end of table
                        break;
                    }
                } else {
                    // First row, set expected column count
                    if col_count >= self.min_cols {
                        consistent_columns = Some(col_count);
                    } else {
                        // Not enough columns for a table
                        break;
                    }
                }
                
                rows.push(columns);
                end_line = idx;
            } else if !rows.is_empty() {
                // Can't parse as row and we have existing rows, end table
                break;
            }
        }
        
        // Check if we have enough rows for a valid table
        if rows.len() >= self.min_rows {
            Some(TableRegion {
                rows,
                start_line: start,
                end_line,
            })
        } else {
            None
        }
    }
    
    fn parse_row(&self, line: &str) -> Option<Vec<String>> {
        // Split by column separators
        let parts: Vec<&str> = COLUMN_SEPARATOR.split(line)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        if parts.len() >= self.min_cols {
            Some(parts.iter().map(|s| s.to_string()).collect())
        } else {
            None
        }
    }
    
    pub fn is_likely_header(&self, row: &[String]) -> bool {
        // Check if row contains mostly uppercase text or common header keywords
        row.iter().any(|cell| {
            let upper = cell.to_uppercase();
            cell == &upper || HEADER_PATTERN.is_match(cell)
        })
    }
}