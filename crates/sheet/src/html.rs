//! HTML table import functionality
//!
//! ## Features
//!
//! - **Colspan and Rowspan**: Both `colspan` and `rowspan` attributes are properly handled,
//!   duplicating cell values across the spanned columns and rows while maintaining proper
//!   table structure alignment.
//! - **Mixed cell types**: Supports tables with mixed `<th>` and `<td>` elements, treating
//!   `<th>` elements as string headers regardless of content.
//! - **Type inference**: Automatically detects and converts cell values to appropriate types
//!   (integers, floats, booleans, strings) while preserving leading zeros in strings.
//!
//! ## Limitations
//!
//! - **Text extraction**: Nested HTML elements in cells are concatenated without whitespace
//!   normalization (e.g., `foo<b>bar</b>` becomes `"foobar"`, not `"foo bar"`).

use crate::{CellValue, Result, Sheet, SheetError};
use scraper::{Html, Selector};
use std::fs;

/// Parse a single HTML table element into a Sheet
fn parse_table_element(table: scraper::ElementRef<'_>) -> Result<Sheet> {
    parse_table_element_with_options(table, false)
}

/// Parse a single HTML table element into a Sheet with options
fn parse_table_element_with_options(
    table: scraper::ElementRef<'_>,
    force_first_row_as_strings: bool,
) -> Result<Sheet> {
    let mut sheet = Sheet::new();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("th, td").unwrap();

    // Track occupied cells due to rowspan from previous rows
    // Key: (row_index, col_index), Value: cell_value
    let mut occupied_cells: std::collections::HashMap<(usize, usize), CellValue> =
        std::collections::HashMap::new();

    // First pass: process all rows and handle colspan/rowspan
    let mut all_rows = Vec::new();
    let mut max_columns = 0;

    for (row_index, row) in table.select(&row_selector).enumerate() {
        let mut row_data = Vec::new();
        let mut col_index = 0;
        let is_first_row = row_index == 0;

        // Process all cells (th and td) in DOM order
        for cell in row.select(&cell_selector) {
            // Skip columns occupied by previous rows' rowspan
            while let Some(occupied_value) = occupied_cells.remove(&(row_index, col_index)) {
                row_data.push(occupied_value);
                col_index += 1;
            }

            let text: String = cell.text().collect::<String>().trim().to_string();

            // Handle colspan attribute - repeat the cell value horizontally
            let colspan = cell
                .value()
                .attr("colspan")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);

            // Handle rowspan attribute - repeat the cell value vertically
            let rowspan = cell
                .value()
                .attr("rowspan")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);

            // Determine cell value type based on element type and options
            let cell_value = if cell.value().name() == "th" {
                // Always treat th elements as strings
                CellValue::String(text.clone())
            } else if force_first_row_as_strings && is_first_row {
                // Force first row to be strings if headers are expected
                CellValue::String(text.clone())
            } else {
                // Normal type inference for td elements
                parse_cell_value(&text)
            };

            // Handle both colspan and rowspan
            for row_offset in 0..rowspan {
                for col_offset in 0..colspan {
                    let target_row = row_index + row_offset;
                    let target_col = col_index + col_offset;

                    let value_to_insert = if row_offset == 0 && col_offset == 0 {
                        // First cell gets the original value
                        cell_value.clone()
                    } else if row_offset == 0
                        && col_offset > 0
                        && (cell.value().name() == "th" || force_first_row_as_strings)
                        && is_first_row
                    {
                        // For header row cells or th elements in first row with colspan, append a suffix to make them unique
                        match &cell_value {
                            CellValue::String(s) => {
                                CellValue::String(format!("{}_{}", s, col_offset + 1))
                            }
                            _ => CellValue::String(format!("{}_{}", &text, col_offset + 1)),
                        }
                    } else {
                        // For other cells (rowspan duplicates, data cell colspan duplicates), just duplicate the value
                        cell_value.clone()
                    };

                    if row_offset == 0 {
                        // Current row - add to row_data
                        row_data.push(value_to_insert);
                    } else {
                        // Future row - mark as occupied
                        occupied_cells.insert((target_row, target_col), value_to_insert);
                    }
                }
            }

            col_index += colspan;
        }

        // Add any remaining occupied cells for this row (at the end)
        while let Some(value) = occupied_cells.remove(&(row_index, col_index)) {
            row_data.push(value);
            col_index += 1;
        }

        // Even if row had no explicit cells, we need to include it if it has occupied cells from rowspan
        // This handles cases where rowspan cells occupy columns but the row itself is empty or has no cells
        if row_data.is_empty() {
            // Check if this row has any occupied cells at all (scanning all possible columns)
            let mut temp_col = 0;
            while let Some(value) = occupied_cells.remove(&(row_index, temp_col)) {
                row_data.push(value);
                temp_col += 1;
            }
            
            // Continue scanning in case there are gaps
            while occupied_cells.keys().any(|(r, _)| *r == row_index) {
                if let Some(value) = occupied_cells.remove(&(row_index, temp_col)) {
                    row_data.push(value);
                } else {
                    // Fill gap with Null
                    row_data.push(CellValue::Null);
                }
                temp_col += 1;
            }
        }

        // Include row if it has any data (explicit cells or occupied cells from rowspan)
        if !row_data.is_empty() {
            max_columns = max_columns.max(row_data.len());
            all_rows.push(row_data);
        }
    }

    // Second pass: normalize all rows to have the same number of columns
    for (row_index, mut row_data) in all_rows.into_iter().enumerate() {
        let is_first_row = row_index == 0;

        // Pad rows that are shorter than max_columns
        while row_data.len() < max_columns {
            if force_first_row_as_strings && is_first_row {
                // For header rows, generate column names instead of Null values
                let col_index = row_data.len();
                let generated_name = format!("Column_{}", col_index + 1);
                row_data.push(CellValue::String(generated_name));
            } else {
                // For data rows, pad with Null values
                row_data.push(CellValue::Null);
            }
        }
        sheet.row_append(row_data)?;
    }

    Ok(sheet)
}

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

    /// Load a sheet from an HTML file with header handling options.
    ///
    /// # Arguments
    /// * `path` - Path to the HTML file
    /// * `has_headers` - Whether the first row should be treated as headers
    ///
    /// # Returns
    /// A `Sheet` containing data from the first table found in the HTML file.
    pub fn from_html_with_headers(path: &str, has_headers: bool) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|e| SheetError::Io(e))?;

        Self::from_html_string_with_headers(&contents, has_headers)
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

        parse_table_element(table)
    }

    /// Load a sheet from an HTML string with header handling options.
    ///
    /// # Arguments
    /// * `html_content` - HTML content as a string
    /// * `has_headers` - Whether the first row should be treated as headers
    ///
    /// # Returns
    /// A `Sheet` containing data from the first table found in the HTML.
    pub fn from_html_string_with_headers(html_content: &str, has_headers: bool) -> Result<Self> {
        let document = Html::parse_document(html_content);

        // Select the first table
        let table_selector = Selector::parse("table").unwrap();
        let table = document
            .select(&table_selector)
            .next()
            .ok_or_else(|| SheetError::Parse("No table found in HTML".to_string()))?;

        let mut sheet = parse_table_element_with_options(table, has_headers)?;

        // If headers are expected, name the columns automatically
        if has_headers && sheet.row_count() > 0 {
            sheet.name_columns_by_row(0)?;
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

        parse_table_element(table)
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
        let mut table_count = 0;

        for table in document.select(&table_selector) {
            table_count += 1;
            let sheet = parse_table_element(table)?;
            // Include empty tables as valid sheets
            sheets.push(sheet);
        }

        if table_count == 0 {
            return Err(SheetError::Parse(
                "No table elements found in HTML".to_string(),
            ));
        }

        Ok(sheets)
    }
}

/// Parse a cell value from text, attempting to convert to appropriate type
fn parse_cell_value(text: &str) -> CellValue {
    // Try to parse as integer (but preserve leading zeros as strings)
    if let Ok(i) = text.parse::<i64>() {
        // Check if the string has leading zeros (except for "0" itself)
        if text.len() > 1 && text.starts_with('0') && text != "0" {
            // Keep as string to preserve leading zeros
            return CellValue::String(text.to_string());
        }
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
            sheet.get(0, 0).unwrap(),
            &CellValue::String("Name".to_string())
        );
        assert_eq!(
            sheet.get(0, 1).unwrap(),
            &CellValue::String("Age".to_string())
        );
        assert_eq!(
            sheet.get(0, 2).unwrap(),
            &CellValue::String("City".to_string())
        );

        // Check data rows
        assert_eq!(
            sheet.get(1, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(sheet.get(1, 1).unwrap(), &CellValue::Int(30));
        assert_eq!(
            sheet.get(1, 2).unwrap(),
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
            sheets[0].get(0, 0).unwrap(),
            &CellValue::String("A".to_string())
        );
        assert_eq!(sheets[0].get(1, 0).unwrap(), &CellValue::Int(1));

        // Check second table
        assert_eq!(
            sheets[1].get(0, 0).unwrap(),
            &CellValue::String("X".to_string())
        );
        assert_eq!(sheets[1].get(1, 0).unwrap(), &CellValue::Int(3));
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
            sheet.get(0, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(sheet.get(0, 1).unwrap(), &CellValue::Int(30));
        assert_eq!(sheet.get(0, 2).unwrap(), &CellValue::Bool(true));
        assert_eq!(sheet.get(0, 3).unwrap(), &CellValue::Float(3.14));
    }

    #[test]
    fn test_no_table_in_html() {
        let html = r#"<div>No table here</div>"#;

        let result = Sheet::from_html_string(html);
        assert!(result.is_err());
    }

    #[test]
    fn test_mixed_th_td_rows() {
        // Test that rows with both th and td cells work correctly
        let html = r#"
            <table>
                <tr>
                    <th>Row Header</th>
                    <td>Value 1</td>
                    <td>100</td>
                </tr>
                <tr>
                    <th>Another Header</th>
                    <td>Value 2</td>
                    <td>200</td>
                </tr>
            </table>
        "#;

        let sheet = Sheet::from_html_string(html).unwrap();

        assert_eq!(sheet.row_count(), 2);
        assert_eq!(sheet.col_count(), 3);

        // First cell of each row should be a string (th)
        assert_eq!(
            sheet.get(0, 0).unwrap(),
            &CellValue::String("Row Header".to_string())
        );
        assert_eq!(
            sheet.get(1, 0).unwrap(),
            &CellValue::String("Another Header".to_string())
        );

        // Other cells should be parsed by type
        assert_eq!(
            sheet.get(0, 1).unwrap(),
            &CellValue::String("Value 1".to_string())
        );
        assert_eq!(sheet.get(0, 2).unwrap(), &CellValue::Int(100));
        assert_eq!(sheet.get(1, 2).unwrap(), &CellValue::Int(200));
    }

    #[test]
    fn test_simple_rowspan() {
        // Test basic rowspan functionality
        let html = r#"
            <table>
                <tr>
                    <th>Name</th>
                    <th>Details</th>
                </tr>
                <tr>
                    <td rowspan="2">Alice</td>
                    <td>Engineer</td>
                </tr>
                <tr>
                    <td>Senior</td>
                </tr>
            </table>
        "#;

        let sheet = Sheet::from_html_string(html).unwrap();

        assert_eq!(sheet.row_count(), 3);
        assert_eq!(sheet.col_count(), 2);

        // Check header row
        assert_eq!(
            sheet.get(0, 0).unwrap(),
            &CellValue::String("Name".to_string())
        );
        assert_eq!(
            sheet.get(0, 1).unwrap(),
            &CellValue::String("Details".to_string())
        );

        // Check first data row
        assert_eq!(
            sheet.get(1, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(
            sheet.get(1, 1).unwrap(),
            &CellValue::String("Engineer".to_string())
        );

        // Check second data row - Alice should be duplicated, Senior should be in second column
        assert_eq!(
            sheet.get(2, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(
            sheet.get(2, 1).unwrap(),
            &CellValue::String("Senior".to_string())
        );
    }

    #[test]
    fn test_combined_colspan_rowspan() {
        // Test table with both colspan and rowspan
        let html = r#"
            <table>
                <tr>
                    <th colspan="2">Personal Info</th>
                    <th>Status</th>
                </tr>
                <tr>
                    <td rowspan="2">Alice</td>
                    <td>Age: 30</td>
                    <td>Active</td>
                </tr>
                <tr>
                    <td>City: NYC</td>
                    <td>Inactive</td>
                </tr>
            </table>
        "#;

        let sheet = Sheet::from_html_string(html).unwrap();

        assert_eq!(sheet.row_count(), 3);
        assert_eq!(sheet.col_count(), 3);

        // Check header row with colspan
        assert_eq!(
            sheet.get(0, 0).unwrap(),
            &CellValue::String("Personal Info".to_string())
        );
        assert_eq!(
            sheet.get(0, 1).unwrap(),
            &CellValue::String("Personal Info_2".to_string())
        );
        assert_eq!(
            sheet.get(0, 2).unwrap(),
            &CellValue::String("Status".to_string())
        );

        // Check first data row
        assert_eq!(
            sheet.get(1, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(
            sheet.get(1, 1).unwrap(),
            &CellValue::String("Age: 30".to_string())
        );
        assert_eq!(
            sheet.get(1, 2).unwrap(),
            &CellValue::String("Active".to_string())
        );

        // Check second data row - Alice duplicated due to rowspan
        assert_eq!(
            sheet.get(2, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(
            sheet.get(2, 1).unwrap(),
            &CellValue::String("City: NYC".to_string())
        );
        assert_eq!(
            sheet.get(2, 2).unwrap(),
            &CellValue::String("Inactive".to_string())
        );
    }

    #[test]
    fn test_complex_rowspan_layout() {
        // Test more complex rowspan with multiple spanning cells
        let html = r#"
            <table>
                <tr>
                    <th>Category</th>
                    <th>Item</th>
                    <th>Value</th>
                </tr>
                <tr>
                    <td rowspan="3">Food</td>
                    <td>Apple</td>
                    <td>5</td>
                </tr>
                <tr>
                    <td>Banana</td>
                    <td>3</td>
                </tr>
                <tr>
                    <td>Orange</td>
                    <td>7</td>
                </tr>
                <tr>
                    <td rowspan="2">Drinks</td>
                    <td>Water</td>
                    <td>10</td>
                </tr>
                <tr>
                    <td>Juice</td>
                    <td>2</td>
                </tr>
            </table>
        "#;

        let sheet = Sheet::from_html_string(html).unwrap();

        assert_eq!(sheet.row_count(), 6);
        assert_eq!(sheet.col_count(), 3);

        // Check Food category spanning 3 rows
        assert_eq!(
            sheet.get(1, 0).unwrap(),
            &CellValue::String("Food".to_string())
        );
        assert_eq!(
            sheet.get(2, 0).unwrap(),
            &CellValue::String("Food".to_string())
        );
        assert_eq!(
            sheet.get(3, 0).unwrap(),
            &CellValue::String("Food".to_string())
        );

        // Check items under Food
        assert_eq!(
            sheet.get(1, 1).unwrap(),
            &CellValue::String("Apple".to_string())
        );
        assert_eq!(
            sheet.get(2, 1).unwrap(),
            &CellValue::String("Banana".to_string())
        );
        assert_eq!(
            sheet.get(3, 1).unwrap(),
            &CellValue::String("Orange".to_string())
        );

        // Check Drinks category spanning 2 rows
        assert_eq!(
            sheet.get(4, 0).unwrap(),
            &CellValue::String("Drinks".to_string())
        );
        assert_eq!(
            sheet.get(5, 0).unwrap(),
            &CellValue::String("Drinks".to_string())
        );

        // Check items under Drinks
        assert_eq!(
            sheet.get(4, 1).unwrap(),
            &CellValue::String("Water".to_string())
        );
        assert_eq!(
            sheet.get(5, 1).unwrap(),
            &CellValue::String("Juice".to_string())
        );

        // Check values
        assert_eq!(sheet.get(1, 2).unwrap(), &CellValue::Int(5));
        assert_eq!(sheet.get(4, 2).unwrap(), &CellValue::Int(10));
        assert_eq!(sheet.get(5, 2).unwrap(), &CellValue::Int(2));
    }

    #[test]
    fn test_rowspan_only_rows() {
        // Test for CodeRabbit finding: rows with only occupied cells from rowspan shouldn't be dropped
        let html = r#"
            <table>
                <tr>
                    <th>Name</th>
                    <th>Info</th>
                </tr>
                <tr>
                    <td rowspan="3">Alice</td>
                    <td>Engineer</td>
                </tr>
                <tr>
                    <!-- This row has no explicit cells, only occupied from rowspan -->
                </tr>
                <tr>
                    <td>Senior Level</td>
                </tr>
            </table>
        "#;

        let sheet = Sheet::from_html_string(html).unwrap();

        // Should have 4 rows: header + 3 data rows (including the empty one)
        assert_eq!(sheet.row_count(), 4);
        assert_eq!(sheet.col_count(), 2);

        // Check that Alice appears in all three data rows
        assert_eq!(
            sheet.get(1, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(
            sheet.get(2, 0).unwrap(), // The "empty" row should still have Alice
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(
            sheet.get(3, 0).unwrap(),
            &CellValue::String("Alice".to_string())
        );

        // Check the explicit cells
        assert_eq!(
            sheet.get(1, 1).unwrap(),
            &CellValue::String("Engineer".to_string())
        );
        assert_eq!(
            sheet.get(2, 1).unwrap(), // Should be Null since no explicit cell
            &CellValue::Null
        );
        assert_eq!(
            sheet.get(3, 1).unwrap(),
            &CellValue::String("Senior Level".to_string())
        );
    }

    #[test]
    fn test_empty_tr_with_rowspan_offset() {
        // Test for CodeRabbit finding: empty rows where rowspan occupies column > 0
        let html = r#"
            <table>
                <tr>
                    <th>A</th>
                    <th>B</th>
                    <th>C</th>
                </tr>
                <tr>
                    <td>X</td>
                    <td rowspan="2">Data</td>
                    <td>Z</td>
                </tr>
                <tr>
                    <td>X2</td>
                    <!-- Column 1 occupied by rowspan, column 2 would be empty -->
                    <td>Z2</td>
                </tr>
            </table>
        "#;

        let sheet = Sheet::from_html_string(html).unwrap();

        assert_eq!(sheet.row_count(), 3);
        assert_eq!(sheet.col_count(), 3);

        // Check that Data appears in both rows where it spans
        assert_eq!(
            sheet.get(1, 1).unwrap(),
            &CellValue::String("Data".to_string())
        );
        assert_eq!(
            sheet.get(2, 1).unwrap(),
            &CellValue::String("Data".to_string())
        );

        // Check other cells
        assert_eq!(
            sheet.get(2, 0).unwrap(),
            &CellValue::String("X2".to_string())
        );
        assert_eq!(
            sheet.get(2, 2).unwrap(),
            &CellValue::String("Z2".to_string())
        );
    }
}
