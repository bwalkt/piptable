//! HTML table import functionality

use crate::{CellValue, Result, Sheet, SheetError};
use scraper::{Html, Selector};
use std::fs;

impl Sheet {
    /// Load a sheet from an HTML file containing tables.
    ///
    /// # Arguments
    /// * `path` - Path to the HTML file
    ///
    /// # Returns
    /// A `Sheet` containing data from the first table found in the HTML file.
    ///
    /// # Example
    /// ```no_run
    /// use piptable_sheet::Sheet;
    ///
    /// let sheet = Sheet::from_html("data.html").unwrap();
    /// ```
    pub fn from_html(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|e| SheetError::Io(e))?;

        Self::from_html_string(&contents)
    }

    /// Load a sheet from an HTML string containing tables.
    ///
    /// # Arguments
    /// * `html_content` - HTML content as a string
    ///
    /// # Returns
    /// A `Sheet` containing data from the first table found in the HTML.
    pub fn from_html_string(html_content: &str) -> Result<Self> {
        let document = Html::parse_document(html_content);

        // Select the first table
        let table_selector = Selector::parse("table").unwrap();
        let table = document
            .select(&table_selector)
            .next()
            .ok_or_else(|| SheetError::Parse("No table found in HTML".to_string()))?;

        let mut sheet = Sheet::new();

        // Parse rows
        let row_selector = Selector::parse("tr").unwrap();
        let cell_selector_th = Selector::parse("th").unwrap();
        let cell_selector_td = Selector::parse("td").unwrap();

        for row in table.select(&row_selector) {
            let mut row_data = Vec::new();

            // First check for header cells (th)
            let th_cells: Vec<scraper::element_ref::ElementRef> =
                row.select(&cell_selector_th).collect();
            if th_cells.is_empty() {
                // Check for regular cells (td)
                for cell in row.select(&cell_selector_td) {
                    let text: String = cell.text().collect::<String>().trim().to_string();
                    row_data.push(parse_cell_value(&text));
                }
            } else {
                // Process header cells (th)
                for cell in th_cells {
                    let text: String = cell.text().collect::<String>().trim().to_string();
                    row_data.push(CellValue::String(text));
                }
            }

            if !row_data.is_empty() {
                sheet.row_append(row_data)?;
            }
        }

        Ok(sheet)
    }

    /// Load a specific table from an HTML file by index.
    ///
    /// # Arguments
    /// * `path` - Path to the HTML file
    /// * `table_index` - Zero-based index of the table to extract
    ///
    /// # Returns
    /// A `Sheet` containing data from the specified table.
    pub fn from_html_table(path: &str, table_index: usize) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|e| SheetError::Io(e))?;

        Self::from_html_table_string(&contents, table_index)
    }

    /// Load a specific table from an HTML string by index.
    ///
    /// # Arguments
    /// * `html_content` - HTML content as a string
    /// * `table_index` - Zero-based index of the table to extract
    ///
    /// # Returns
    /// A `Sheet` containing data from the specified table.
    pub fn from_html_table_string(html_content: &str, table_index: usize) -> Result<Self> {
        let document = Html::parse_document(html_content);

        // Select all tables
        let table_selector = Selector::parse("table").unwrap();
        let table = document
            .select(&table_selector)
            .nth(table_index)
            .ok_or_else(|| {
                SheetError::Parse(format!("Table index {} not found in HTML", table_index))
            })?;

        let mut sheet = Sheet::new();

        // Parse rows
        let row_selector = Selector::parse("tr").unwrap();
        let cell_selector_th = Selector::parse("th").unwrap();
        let cell_selector_td = Selector::parse("td").unwrap();

        for row in table.select(&row_selector) {
            let mut row_data = Vec::new();

            // First check for header cells (th)
            let th_cells: Vec<scraper::element_ref::ElementRef> =
                row.select(&cell_selector_th).collect();
            if th_cells.is_empty() {
                // Check for regular cells (td)
                for cell in row.select(&cell_selector_td) {
                    let text: String = cell.text().collect::<String>().trim().to_string();
                    row_data.push(parse_cell_value(&text));
                }
            } else {
                // Process header cells (th)
                for cell in th_cells {
                    let text: String = cell.text().collect::<String>().trim().to_string();
                    row_data.push(CellValue::String(text));
                }
            }

            if !row_data.is_empty() {
                sheet.row_append(row_data)?;
            }
        }

        Ok(sheet)
    }

    /// Load all tables from an HTML file.
    ///
    /// # Arguments
    /// * `path` - Path to the HTML file
    ///
    /// # Returns
    /// A vector of `Sheet`s, one for each table found in the HTML.
    pub fn from_html_all_tables(path: &str) -> Result<Vec<Self>> {
        let contents = fs::read_to_string(path).map_err(|e| SheetError::Io(e))?;

        Self::from_html_all_tables_string(&contents)
    }

    /// Load all tables from an HTML string.
    ///
    /// # Arguments
    /// * `html_content` - HTML content as a string
    ///
    /// # Returns
    /// A vector of `Sheet`s, one for each table found in the HTML.
    pub fn from_html_all_tables_string(html_content: &str) -> Result<Vec<Self>> {
        let document = Html::parse_document(html_content);
        let table_selector = Selector::parse("table").unwrap();

        let mut sheets = Vec::new();

        for table in document.select(&table_selector) {
            let mut sheet = Sheet::new();

            // Parse rows
            let row_selector = Selector::parse("tr").unwrap();
            let cell_selector_th = Selector::parse("th").unwrap();
            let cell_selector_td = Selector::parse("td").unwrap();

            for row in table.select(&row_selector) {
                let mut row_data = Vec::new();

                // First check for header cells (th)
                let th_cells: Vec<scraper::element_ref::ElementRef> =
                    row.select(&cell_selector_th).collect();
                if th_cells.is_empty() {
                    // Check for regular cells (td)
                    for cell in row.select(&cell_selector_td) {
                        let text: String = cell.text().collect::<String>().trim().to_string();
                        row_data.push(parse_cell_value(&text));
                    }
                } else {
                    // Process header cells (th)
                    for cell in th_cells {
                        let text: String = cell.text().collect::<String>().trim().to_string();
                        row_data.push(CellValue::String(text));
                    }
                }

                if !row_data.is_empty() {
                    sheet.row_append(row_data)?;
                }
            }

            if sheet.row_count() > 0 {
                sheets.push(sheet);
            }
        }

        if sheets.is_empty() {
            return Err(SheetError::Parse("No tables found in HTML".to_string()));
        }

        Ok(sheets)
    }
}

/// Parse a cell value from text, attempting to convert to appropriate type
fn parse_cell_value(text: &str) -> CellValue {
    // Try to parse as integer
    if let Ok(i) = text.parse::<i64>() {
        return CellValue::Int(i);
    }

    // Try to parse as float
    if let Ok(f) = text.parse::<f64>() {
        return CellValue::Float(f);
    }

    // Try to parse as boolean
    if let Ok(b) = text.parse::<bool>() {
        return CellValue::Bool(b);
    }

    // Check for common boolean representations
    let lower = text.to_lowercase();
    if lower == "yes" || lower == "y" || lower == "true" || lower == "t" {
        return CellValue::Bool(true);
    }
    if lower == "no" || lower == "n" || lower == "false" || lower == "f" {
        return CellValue::Bool(false);
    }

    // Check for null/empty
    if text.is_empty() || lower == "null" || lower == "none" {
        return CellValue::Null;
    }

    // Default to string
    CellValue::String(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_html_table() {
        let html = r#"
            <table>
                <tr>
                    <th>Name</th>
                    <th>Age</th>
                    <th>City</th>
                </tr>
                <tr>
                    <td>Alice</td>
                    <td>30</td>
                    <td>New York</td>
                </tr>
                <tr>
                    <td>Bob</td>
                    <td>25</td>
                    <td>Los Angeles</td>
                </tr>
            </table>
        "#;

        let sheet = Sheet::from_html_string(html).unwrap();

        assert_eq!(sheet.row_count(), 3);
        assert_eq!(sheet.col_count(), 3);

        // Check header row
        assert_eq!(
            sheet.cell(0, 0).unwrap(),
            &CellValue::String("Name".to_string())
        );
        assert_eq!(
            sheet.cell(0, 1).unwrap(),
            &CellValue::String("Age".to_string())
        );
        assert_eq!(
            sheet.cell(0, 2).unwrap(),
            &CellValue::String("City".to_string())
        );

        // Check data rows
        assert_eq!(
            sheet.cell(1, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(sheet.cell(1, 1).unwrap(), &CellValue::Int(30));
        assert_eq!(
            sheet.cell(1, 2).unwrap(),
            &CellValue::String("New York".to_string())
        );
    }

    #[test]
    fn test_parse_multiple_tables() {
        let html = r#"
            <table>
                <tr><th>A</th><th>B</th></tr>
                <tr><td>1</td><td>2</td></tr>
            </table>
            <table>
                <tr><th>X</th><th>Y</th></tr>
                <tr><td>3</td><td>4</td></tr>
            </table>
        "#;

        let sheets = Sheet::from_html_all_tables_string(html).unwrap();

        assert_eq!(sheets.len(), 2);
        assert_eq!(sheets[0].row_count(), 2);
        assert_eq!(sheets[1].row_count(), 2);

        // Check first table
        assert_eq!(
            sheets[0].cell(0, 0).unwrap(),
            &CellValue::String("A".to_string())
        );
        assert_eq!(sheets[0].cell(1, 0).unwrap(), &CellValue::Int(1));

        // Check second table
        assert_eq!(
            sheets[1].cell(0, 0).unwrap(),
            &CellValue::String("X".to_string())
        );
        assert_eq!(sheets[1].cell(1, 0).unwrap(), &CellValue::Int(3));
    }

    #[test]
    fn test_parse_table_with_mixed_types() {
        let html = r#"
            <table>
                <tr><td>Alice</td><td>30</td><td>true</td><td>3.14</td></tr>
                <tr><td>Bob</td><td>25</td><td>false</td><td>2.71</td></tr>
            </table>
        "#;

        let sheet = Sheet::from_html_string(html).unwrap();

        assert_eq!(sheet.row_count(), 2);
        assert_eq!(sheet.col_count(), 4);

        // Check data types
        assert_eq!(
            sheet.cell(0, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(sheet.cell(0, 1).unwrap(), &CellValue::Int(30));
        assert_eq!(sheet.cell(0, 2).unwrap(), &CellValue::Bool(true));
        assert_eq!(sheet.cell(0, 3).unwrap(), &CellValue::Float(3.14));
    }

    #[test]
    fn test_no_table_in_html() {
        let html = r#"<div>No table here</div>"#;

        let result = Sheet::from_html_string(html);
        assert!(result.is_err());
    }
}
