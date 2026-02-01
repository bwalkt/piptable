use crate::error::{Result, SheetError};
use piptable_primitives::R1C1Ref;

/// Parse A1-style cell notation (e.g., "A1", "Z99", "AA1")
/// Returns (row, column) as 0-based indices
pub fn parse_a1(notation: &str) -> Result<(usize, usize)> {
    if notation.is_empty() {
        return Err(SheetError::InvalidCellNotation(notation.to_string()));
    }

    let notation = notation.to_uppercase();
    let bytes = notation.as_bytes();

    // Find where letters end and numbers begin
    let mut split_pos = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_digit() {
            split_pos = i;
            break;
        }
    }

    if split_pos == 0 {
        return Err(SheetError::InvalidCellNotation(notation));
    }

    // Parse column letters (A=0, B=1, ... Z=25, AA=26, AB=27, ...)
    let col_part = &notation[..split_pos];
    let row_part = &notation[split_pos..];

    if col_part.is_empty() || row_part.is_empty() {
        return Err(SheetError::InvalidCellNotation(notation));
    }

    let col = parse_column_letters(col_part)?;
    let row = row_part
        .parse::<usize>()
        .map_err(|_| SheetError::InvalidCellNotation(notation.clone()))?;

    // Convert to 0-based indexing (A1 = 0,0)
    if row == 0 {
        return Err(SheetError::InvalidCellNotation(notation));
    }

    Ok((row - 1, col))
}

/// Parse A1-style range notation (e.g., "A1:C3")
/// Returns ((start_row, start_col), (end_row, end_col)) as 0-based indices
pub fn parse_a1_range(notation: &str) -> Result<((usize, usize), (usize, usize))> {
    let parts: Vec<&str> = notation.split(':').collect();

    if parts.len() != 2 {
        // If no colon, treat as single cell
        let cell = parse_a1(notation)?;
        return Ok((cell, cell));
    }

    let start = parse_a1(parts[0])?;
    let end = parse_a1(parts[1])?;

    // Ensure start <= end
    let (start_row, start_col) = start;
    let (end_row, end_col) = end;

    let actual_start = (start_row.min(end_row), start_col.min(end_col));
    let actual_end = (start_row.max(end_row), start_col.max(end_col));

    Ok((actual_start, actual_end))
}

/// Parse cell notation supporting A1 or absolute R1C1 (e.g., "A1" or "R1C1").
pub fn parse_cell_notation(notation: &str) -> Result<(usize, usize)> {
    if let Ok(cell) = parse_a1(notation) {
        return Ok(cell);
    }

    let r1c1 = R1C1Ref::from_r1c1(notation)
        .map_err(|_| SheetError::InvalidCellNotation(notation.to_string()))?;
    if !r1c1.is_absolute() {
        return Err(SheetError::InvalidCellNotation(notation.to_string()));
    }
    let addr = r1c1
        .resolve(None)
        .map_err(|_| SheetError::InvalidCellNotation(notation.to_string()))?;
    Ok((addr.row as usize, addr.col as usize))
}

/// Parse range notation supporting A1 or absolute R1C1 (e.g., "A1:C3" or "R1C1:R3C3").
pub fn parse_range_notation(notation: &str) -> Result<((usize, usize), (usize, usize))> {
    let parts: Vec<&str> = notation.split(':').collect();
    if parts.len() != 2 {
        let cell = parse_cell_notation(notation)?;
        return Ok((cell, cell));
    }

    let start = parse_cell_notation(parts[0])?;
    let end = parse_cell_notation(parts[1])?;

    let (start_row, start_col) = start;
    let (end_row, end_col) = end;

    let actual_start = (start_row.min(end_row), start_col.min(end_col));
    let actual_end = (start_row.max(end_row), start_col.max(end_col));

    Ok((actual_start, actual_end))
}

/// Convert column letters to 0-based column index
/// A=0, B=1, ... Z=25, AA=26, AB=27, ...
fn parse_column_letters(col_str: &str) -> Result<usize> {
    if col_str.is_empty() {
        return Err(SheetError::InvalidCellNotation(col_str.to_string()));
    }

    let mut col = 0;
    let bytes = col_str.as_bytes();

    for &b in bytes {
        if !b.is_ascii_uppercase() {
            return Err(SheetError::InvalidCellNotation(col_str.to_string()));
        }
        col = col * 26 + (b - b'A') as usize + 1;
    }

    Ok(col - 1) // Convert to 0-based
}

/// Convert 0-based column index to column letters
/// 0=A, 1=B, ... 25=Z, 26=AA, 27=AB, ...
#[allow(dead_code)]
pub fn column_index_to_letters(mut col: usize) -> String {
    let mut result = String::new();
    col += 1; // Convert to 1-based for calculation

    while col > 0 {
        col -= 1;
        result.insert(0, ((col % 26) as u8 + b'A') as char);
        col /= 26;
    }

    result
}

/// Convert (row, col) to A1 notation
/// (0, 0) = "A1", (0, 1) = "B1", etc.
#[allow(dead_code)]
pub fn to_a1_notation(row: usize, col: usize) -> String {
    format!("{}{}", column_index_to_letters(col), row + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_a1() {
        assert_eq!(parse_a1("A1").unwrap(), (0, 0));
        assert_eq!(parse_a1("B1").unwrap(), (0, 1));
        assert_eq!(parse_a1("A2").unwrap(), (1, 0));
        assert_eq!(parse_a1("Z1").unwrap(), (0, 25));
        assert_eq!(parse_a1("AA1").unwrap(), (0, 26));
        assert_eq!(parse_a1("AB1").unwrap(), (0, 27));
        assert_eq!(parse_a1("AZ1").unwrap(), (0, 51));
        assert_eq!(parse_a1("BA1").unwrap(), (0, 52));
        assert_eq!(parse_a1("ZZ1").unwrap(), (0, 701));

        // Test case insensitive
        assert_eq!(parse_a1("a1").unwrap(), (0, 0));
        assert_eq!(parse_a1("aA1").unwrap(), (0, 26));
    }

    #[test]
    fn test_parse_a1_errors() {
        assert!(parse_a1("").is_err());
        assert!(parse_a1("A").is_err());
        assert!(parse_a1("1").is_err());
        assert!(parse_a1("A0").is_err()); // Row must be >= 1
        assert!(parse_a1("123").is_err());
        assert!(parse_a1("ABC").is_err());
    }

    #[test]
    fn test_parse_a1_range() {
        let ((sr, sc), (er, ec)) = parse_a1_range("A1:C3").unwrap();
        assert_eq!((sr, sc), (0, 0));
        assert_eq!((er, ec), (2, 2));

        // Test reversed range (should auto-correct)
        let ((sr, sc), (er, ec)) = parse_a1_range("C3:A1").unwrap();
        assert_eq!((sr, sc), (0, 0));
        assert_eq!((er, ec), (2, 2));

        // Single cell (no colon)
        let ((sr, sc), (er, ec)) = parse_a1_range("B2").unwrap();
        assert_eq!((sr, sc), (1, 1));
        assert_eq!((er, ec), (1, 1));
    }

    #[test]
    fn test_parse_cell_notation_r1c1() {
        assert_eq!(parse_cell_notation("R1C1").unwrap(), (0, 0));
        assert_eq!(parse_cell_notation("R5C3").unwrap(), (4, 2));
        assert!(parse_cell_notation("R[1]C1").is_err());
    }

    #[test]
    fn test_parse_range_notation_r1c1() {
        let ((sr, sc), (er, ec)) = parse_range_notation("R1C1:R2C3").unwrap();
        assert_eq!((sr, sc), (0, 0));
        assert_eq!((er, ec), (1, 2));
    }

    #[test]
    fn test_column_index_to_letters() {
        assert_eq!(column_index_to_letters(0), "A");
        assert_eq!(column_index_to_letters(1), "B");
        assert_eq!(column_index_to_letters(25), "Z");
        assert_eq!(column_index_to_letters(26), "AA");
        assert_eq!(column_index_to_letters(27), "AB");
        assert_eq!(column_index_to_letters(51), "AZ");
        assert_eq!(column_index_to_letters(52), "BA");
        assert_eq!(column_index_to_letters(701), "ZZ");
        assert_eq!(column_index_to_letters(702), "AAA");
    }

    #[test]
    fn test_to_a1_notation() {
        assert_eq!(to_a1_notation(0, 0), "A1");
        assert_eq!(to_a1_notation(0, 1), "B1");
        assert_eq!(to_a1_notation(1, 0), "A2");
        assert_eq!(to_a1_notation(99, 25), "Z100");
        assert_eq!(to_a1_notation(0, 26), "AA1");
    }

    #[test]
    fn test_roundtrip() {
        // Test that parsing and converting back gives same result
        for row in 0..10 {
            for col in 0..100 {
                let notation = to_a1_notation(row, col);
                let (parsed_row, parsed_col) = parse_a1(&notation).unwrap();
                assert_eq!((row, col), (parsed_row, parsed_col));
            }
        }
    }
}
