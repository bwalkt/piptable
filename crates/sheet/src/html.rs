//! HTML table import functionality

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
    let mut max_columns = 0;

    // First pass: collect all rows and track maximum column count
    let mut all_rows = Vec::new();

    for (row_index, row) in table.select(&row_selector).enumerate() {
        let mut row_data = Vec::new();
        let is_first_row = row_index == 0;

        // Process all cells (th and td) in DOM order
        let cell_selector = Selector::parse("th, td").unwrap();
        for cell in row.select(&cell_selector) {
            let text: String = cell.text().collect::<String>().trim().to_string();

            // Handle colspan attribute - repeat the cell value
            let colspan = cell
                .value()
                .attr("colspan")
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

            // Add the cell value 'colspan' times
            // For the first occurrence, use the original value
            // For subsequent ones, append a suffix to avoid duplicate column names
            for i in 0..colspan {
                if i == 0 {
                    row_data.push(cell_value.clone());
                } else if cell.value().name() == "th"
                    || (force_first_row_as_strings && is_first_row)
                {
                    // For header cells, append a suffix to make them unique
                    let suffix_value = match &cell_value {
                        CellValue::String(s) => CellValue::String(format!("{}_{}", s, i + 1)),
                        _ => CellValue::String(format!("{}_{}", &text, i + 1)),
                    };
                    row_data.push(suffix_value);
                } else {
                    // For data cells, just duplicate the value
                    row_data.push(cell_value.clone());
                }
            }
        }

        if !row_data.is_empty() {
            max_columns = max_columns.max(row_data.len());
            all_rows.push(row_data);
        }
    }

    // Second pass: normalize all rows to have the same number of columns
    for mut row_data in all_rows {
        // Pad rows that are shorter than max_columns with Null values
        while row_data.len() < max_columns {
            row_data.push(CellValue::Null);
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
}
