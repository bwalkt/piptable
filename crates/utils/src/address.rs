//! Address and range helpers for spreadsheet-style A1 references.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellInterface {
    pub row_index: u32,
    pub column_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridRange {
    pub start_row_index: u32,
    pub start_column_index: u32,
    pub end_row_index: u32,
    pub end_column_index: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum AddressPosition {
    Start,
    End,
}

#[derive(Debug, Clone, Copy)]
pub struct AddressToCellOptions {
    pub position: AddressPosition,
    pub row_count: Option<u32>,
    pub column_count: Option<u32>,
}

impl Default for AddressToCellOptions {
    fn default() -> Self {
        Self {
            position: AddressPosition::Start,
            row_count: None,
            column_count: None,
        }
    }
}

pub const MAX_ROW_COUNT: u32 = 1_048_576;
pub const MAX_COLUMN_COUNT: u32 = 16_384;

/// Convert a cell to a range representing the single cell.
pub fn convert_cell_to_range(cell: CellInterface) -> GridRange {
    GridRange {
        start_row_index: cell.row_index,
        start_column_index: cell.column_index,
        end_row_index: cell.row_index,
        end_column_index: cell.column_index,
    }
}

/// Converts address string to CellInterface.
pub fn address_to_cell(address: &str, options: AddressToCellOptions) -> Option<CellInterface> {
    let max_row = options
        .row_count
        .map(|v| v.min(MAX_ROW_COUNT))
        .unwrap_or(MAX_ROW_COUNT);
    let max_col = options
        .column_count
        .map(|v| v.min(MAX_COLUMN_COUNT))
        .unwrap_or(MAX_COLUMN_COUNT);

    if let Some((col_letters, row_digits)) = find_a1_match(address) {
        let row_index = row_digits.parse::<u32>().ok()?;
        let column_index = alpha2number(&col_letters)?;
        return Some(CellInterface {
            row_index: row_index.min(max_row),
            column_index: column_index.min(max_col),
        });
    }

    if is_column_only(address) && options.row_count.is_some() {
        let column = get_column_name_from_address(address)?;
        let column_index = alpha2number(&column)?;
        let row_index = match options.position {
            AddressPosition::Start => 1,
            AddressPosition::End => max_row,
        };
        return Some(CellInterface {
            row_index,
            column_index: column_index.min(max_col),
        });
    }

    if is_row_only(address) && options.column_count.is_some() {
        let row_index = get_row_index_from_address(address)?.parse::<u32>().ok()?;
        let column_index = match options.position {
            AddressPosition::Start => 1,
            AddressPosition::End => max_col,
        };
        return Some(CellInterface {
            row_index: row_index.min(max_row),
            column_index,
        });
    }

    None
}

/// Gets column from address (e.g., "$G" => "G").
pub fn get_column_name_from_address(address: &str) -> Option<String> {
    let stripped = address.trim().trim_start_matches('$');
    if stripped.is_empty() {
        return None;
    }
    if stripped.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(stripped.to_ascii_uppercase());
    }
    None
}

/// Gets row index from address (e.g., "$1" => "1").
pub fn get_row_index_from_address(address: &str) -> Option<String> {
    let stripped = address.trim().trim_start_matches('$');
    if stripped.is_empty() {
        return None;
    }
    if stripped.chars().all(|c| c.is_ascii_digit()) {
        return Some(stripped.to_string());
    }
    None
}

/// Convert CellInterface to address string.
pub fn cell_to_address(
    cell: Option<CellInterface>,
    is_absolute_column: bool,
    is_absolute_row: bool,
    is_full_column: bool,
    is_full_row: bool,
) -> Option<String> {
    let cell = cell?;
    let column_alpha = number2alpha(cell.column_index.saturating_sub(1));
    let row_number = cell.row_index.max(1);

    if is_full_column {
        return Some(column_alpha);
    }
    if is_full_row {
        return Some(row_number.to_string());
    }

    Some(format!(
        "{}{}{}{}",
        if is_absolute_column { "$" } else { "" },
        column_alpha,
        if is_absolute_row { "$" } else { "" },
        row_number
    ))
}

/// Convert a cell range to address.
pub fn cell_range_to_address(sheet_name: Option<&str>, range: GridRange) -> String {
    let sheet_prefix = sheet_name
        .map(|name| format!("{}!", sanitize_sheet_name(Some(name)).unwrap_or_default()))
        .unwrap_or_default();
    let start = cell_to_address(
        Some(CellInterface {
            row_index: range.start_row_index,
            column_index: range.start_column_index,
        }),
        false,
        false,
        false,
        false,
    )
    .unwrap_or_default();
    let end = cell_to_address(
        Some(CellInterface {
            row_index: range.end_row_index,
            column_index: range.end_column_index,
        }),
        false,
        false,
        false,
        false,
    )
    .unwrap_or_default();
    format!("{}{}:{}", sheet_prefix, start, end)
}

/// Sanitize sheet names with special characters.
pub fn sanitize_sheet_name(name: Option<&str>) -> Option<String> {
    let name = name?;
    if name.chars().any(|c| c.is_whitespace() || !c.is_ascii_alphanumeric()) {
        return Some(format!("'{}'", name));
    }
    Some(name.to_string())
}

/// Remove single quotes from sheet name.
pub fn desanitize_sheet_name(name: Option<&str>) -> Option<String> {
    let name = name?;
    Some(name.replace('\'', ""))
}

/// Escape special regex characters in a string.
pub fn escape_characters(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '\\' | '^' | '$' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|')
        {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Get selection bounds from start and end cells.
pub fn selection_bounds_from_cells(start: CellInterface, end: Option<CellInterface>) -> GridRange {
    let end = end.unwrap_or(start);
    GridRange {
        start_row_index: start.row_index.min(end.row_index),
        start_column_index: start.column_index.min(end.column_index),
        end_row_index: start.row_index.max(end.row_index),
        end_column_index: start.column_index.max(end.column_index),
    }
}

/// Replace {key} placeholders in a string.
pub fn supplant(input: &str, values: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut key = String::new();
            while let Some(next) = chars.peek().copied() {
                chars.next();
                if next == '}' {
                    break;
                }
                key.push(next);
            }
            if let Some(replacement) = values.get(&key) {
                out.push_str(replacement);
            } else {
                out.push('{');
                out.push_str(&key);
                out.push('}');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn find_a1_match(address: &str) -> Option<(String, String)> {
    let bytes = address.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let mut j = i;
        if bytes[j] == b'$' {
            j += 1;
            if j >= bytes.len() {
                break;
            }
        }
        if !bytes[j].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let letters_start = j;
        while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
            j += 1;
        }
        let letters = &address[letters_start..j];
        if j < bytes.len() && bytes[j] == b'$' {
            j += 1;
        }
        let digits_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if digits_start == j {
            i += 1;
            continue;
        }
        let digits = &address[digits_start..j];
        return Some((letters.to_ascii_uppercase(), digits.to_string()));
    }
    None
}

fn is_column_only(address: &str) -> bool {
    let stripped = address.trim().trim_start_matches('$');
    !stripped.is_empty() && stripped.chars().all(|c| c.is_ascii_alphabetic())
}

fn is_row_only(address: &str) -> bool {
    let stripped = address.trim().trim_start_matches('$');
    !stripped.is_empty() && stripped.chars().all(|c| c.is_ascii_digit())
}

fn number2alpha(index: u32) -> String {
    let mut n = index;
    let mut out = String::new();
    loop {
        let rem = (n % 26) as u8;
        out.push((b'A' + rem) as char);
        n = n / 26;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    out.chars().rev().collect()
}

fn alpha2number(letters: &str) -> Option<u32> {
    let mut result: u32 = 0;
    for ch in letters.chars() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        let value = (ch.to_ascii_uppercase() as u8 - b'A' + 1) as u32;
        result = result * 26 + value;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_to_cell_basic() {
        let cell = address_to_cell("$B$2", AddressToCellOptions::default()).unwrap();
        assert_eq!(cell.row_index, 2);
        assert_eq!(cell.column_index, 2);
    }

    #[test]
    fn test_address_to_cell_column_only() {
        let opts = AddressToCellOptions {
            position: AddressPosition::End,
            row_count: Some(10),
            column_count: Some(5),
        };
        let cell = address_to_cell("$C", opts).unwrap();
        assert_eq!(cell.row_index, 10);
        assert_eq!(cell.column_index, 3);
    }

    #[test]
    fn test_address_to_cell_row_only() {
        let opts = AddressToCellOptions {
            position: AddressPosition::Start,
            row_count: Some(10),
            column_count: Some(5),
        };
        let cell = address_to_cell("$4", opts).unwrap();
        assert_eq!(cell.row_index, 4);
        assert_eq!(cell.column_index, 1);
    }

    #[test]
    fn test_cell_to_address() {
        let cell = CellInterface {
            row_index: 5,
            column_index: 28,
        };
        let addr = cell_to_address(Some(cell), true, false, false, false).unwrap();
        assert_eq!(addr, "$AB5");
    }

    #[test]
    fn test_sanitize_sheet_name() {
        assert_eq!(sanitize_sheet_name(Some("Sheet1")), Some("Sheet1".to_string()));
        assert_eq!(
            sanitize_sheet_name(Some("Sheet 1")),
            Some("'Sheet 1'".to_string())
        );
    }
}
