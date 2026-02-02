//! # Piptable Primitives
//!
//! Core primitives for spreadsheet operations including cell addresses,
//! ranges, references, and value types.

use serde::{Deserialize, Serialize};
use std::fmt;

pub mod toon;

/// A cell address in the spreadsheet (e.g., A1, B2, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellAddress {
    pub row: u32,
    pub col: u32,
}

impl CellAddress {
    /// Create a new cell address
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    /// Parse from A1 notation (e.g., "A1", "B2")
    pub fn from_a1(s: &str) -> Result<Self, AddressError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(AddressError::InvalidRange("Empty A1 reference".to_string()));
        }

        let mut chars = trimmed.chars().peekable();

        // Optional $ for absolute column
        if matches!(chars.peek(), Some('$')) {
            chars.next();
        }

        // Parse column letters
        let mut col_letters = String::new();
        while let Some(ch) = chars.peek().copied() {
            if ch.is_ascii_alphabetic() {
                col_letters.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        if col_letters.is_empty() {
            return Err(AddressError::InvalidColumn(trimmed.to_string()));
        }

        // Optional $ for absolute row
        if matches!(chars.peek(), Some('$')) {
            chars.next();
        }

        // Parse row digits
        let mut row_digits = String::new();
        while let Some(ch) = chars.peek().copied() {
            if ch.is_ascii_digit() {
                row_digits.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        if row_digits.is_empty() || chars.peek().is_some() {
            return Err(AddressError::InvalidRow(trimmed.to_string()));
        }

        let row_num: u32 = row_digits
            .parse()
            .map_err(|_| AddressError::InvalidRow(row_digits.clone()))?;

        if row_num == 0 {
            return Err(AddressError::InvalidRow(row_digits));
        }

        let col_index = column_letters_to_index(&col_letters)?;
        Ok(Self {
            row: row_num - 1,
            col: col_index,
        })
    }

    /// Convert to A1 notation
    pub fn to_a1(&self) -> String {
        let col_letters = column_index_to_letters(self.col);
        format!("{}{}", col_letters, self.row + 1)
    }

    /// Convert to absolute R1C1 notation
    pub fn to_r1c1(&self) -> String {
        format!("R{}C{}", self.row + 1, self.col + 1)
    }
}

/// A range of cells (e.g., A1:B10)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellRange {
    pub start: CellAddress,
    pub end: CellAddress,
}

impl CellRange {
    /// Create a new cell range
    pub fn new(start: CellAddress, end: CellAddress) -> Self {
        Self { start, end }
    }

    /// Return a normalized range where start <= end
    pub fn normalized(&self) -> Self {
        let start_row = self.start.row.min(self.end.row);
        let end_row = self.start.row.max(self.end.row);
        let start_col = self.start.col.min(self.end.col);
        let end_col = self.start.col.max(self.end.col);
        Self {
            start: CellAddress::new(start_row, start_col),
            end: CellAddress::new(end_row, end_col),
        }
    }

    /// Number of rows in the range
    pub fn rows(&self) -> u32 {
        let range = self.normalized();
        range.end.row - range.start.row + 1
    }

    /// Number of columns in the range
    pub fn cols(&self) -> u32 {
        let range = self.normalized();
        range.end.col - range.start.col + 1
    }

    /// Check if a cell is within this range
    pub fn contains(&self, addr: &CellAddress) -> bool {
        let range = self.normalized();
        addr.row >= range.start.row
            && addr.row <= range.end.row
            && addr.col >= range.start.col
            && addr.col <= range.end.col
    }

    /// Get total number of cells in range
    pub fn size(&self) -> usize {
        let range = self.normalized();
        let rows = (range.end.row - range.start.row + 1) as usize;
        let cols = (range.end.col - range.start.col + 1) as usize;
        rows * cols
    }

    /// Iterate over all addresses in row-major order
    pub fn iter(&self) -> CellRangeIter {
        let range = self.normalized();
        CellRangeIter {
            current: range.start,
            start: range.start,
            end: range.end,
            done: false,
        }
    }
}

/// Cell reference type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellRef {
    /// Absolute reference (e.g., $A$1)
    Absolute(CellAddress),
    /// Relative reference (e.g., A1)
    Relative(CellAddress),
    /// Mixed reference (e.g., $A1 or A$1)
    Mixed {
        row_abs: bool,
        col_abs: bool,
        addr: CellAddress,
    },
}

/// R1C1 axis reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum R1C1Axis {
    /// Absolute index (1-based in notation).
    Absolute(u32),
    /// Relative offset from the current cell.
    Relative(i32),
    /// Same row/column as the current cell.
    Current,
}

/// R1C1-style cell reference (e.g., R1C1, R[-1]C[2], RC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1C1Ref {
    pub row: R1C1Axis,
    pub col: R1C1Axis,
}

impl R1C1Ref {
    /// Parse R1C1 notation (e.g., "R1C1", "R[-1]C[2]", "RC").
    pub fn from_r1c1(s: &str) -> Result<Self, AddressError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(AddressError::InvalidRange(
                "Empty R1C1 reference".to_string(),
            ));
        }

        let mut chars = trimmed.chars().peekable();
        let r = chars
            .next()
            .ok_or_else(|| AddressError::InvalidRange(trimmed.to_string()))?;
        if r != 'R' && r != 'r' {
            return Err(AddressError::InvalidRange(trimmed.to_string()));
        }

        let row = parse_r1c1_axis(&mut chars)?;

        let c = chars
            .next()
            .ok_or_else(|| AddressError::InvalidRange(trimmed.to_string()))?;
        if c != 'C' && c != 'c' {
            return Err(AddressError::InvalidRange(trimmed.to_string()));
        }

        let col = parse_r1c1_axis(&mut chars)?;

        if chars.peek().is_some() {
            return Err(AddressError::InvalidRange(trimmed.to_string()));
        }

        Ok(Self { row, col })
    }

    /// Returns true if both row and column are absolute.
    pub fn is_absolute(&self) -> bool {
        matches!(self.row, R1C1Axis::Absolute(_)) && matches!(self.col, R1C1Axis::Absolute(_))
    }

    /// Resolve to an absolute cell address using an optional base cell.
    pub fn resolve(&self, base: Option<CellAddress>) -> Result<CellAddress, AddressError> {
        let base = match base {
            Some(base) => base,
            None => {
                if self.is_absolute() {
                    CellAddress::new(0, 0)
                } else {
                    return Err(AddressError::InvalidRange(
                        "Relative R1C1 reference requires a base cell".to_string(),
                    ));
                }
            }
        };

        let row = resolve_axis(self.row, base.row)?;
        let col = resolve_axis(self.col, base.col)?;

        Ok(CellAddress { row, col })
    }

    /// Convert to A1 notation using an optional base cell for relative references.
    pub fn to_a1(&self, base: Option<CellAddress>) -> Result<String, AddressError> {
        let addr = self.resolve(base)?;
        Ok(addr.to_a1())
    }

    /// Convert back to R1C1 string notation.
    pub fn to_r1c1(&self) -> String {
        format!(
            "{}{}",
            axis_to_r1c1('R', self.row),
            axis_to_r1c1('C', self.col)
        )
    }
}

fn parse_r1c1_axis<I>(chars: &mut std::iter::Peekable<I>) -> Result<R1C1Axis, AddressError>
where
    I: Iterator<Item = char>,
{
    match chars.peek().copied() {
        Some('[') => {
            chars.next();
            let mut digits = String::new();
            if matches!(chars.peek(), Some('+') | Some('-')) {
                digits.push(chars.next().unwrap());
            }
            while let Some(ch) = chars.peek().copied() {
                if ch.is_ascii_digit() {
                    digits.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            if digits.is_empty() || digits == "+" || digits == "-" {
                return Err(AddressError::InvalidRange(
                    "Invalid R1C1 offset".to_string(),
                ));
            }
            if chars.next() != Some(']') {
                return Err(AddressError::InvalidRange(
                    "Invalid R1C1 offset".to_string(),
                ));
            }
            let offset = digits
                .parse::<i32>()
                .map_err(|_| AddressError::InvalidRange("Invalid R1C1 offset".to_string()))?;
            Ok(R1C1Axis::Relative(offset))
        }
        Some(ch) if ch.is_ascii_digit() => {
            let mut digits = String::new();
            while let Some(ch) = chars.peek().copied() {
                if ch.is_ascii_digit() {
                    digits.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            let value = digits
                .parse::<u32>()
                .map_err(|_| AddressError::InvalidRange(digits.clone()))?;
            if value == 0 {
                return Err(AddressError::InvalidRange(digits));
            }
            Ok(R1C1Axis::Absolute(value))
        }
        _ => Ok(R1C1Axis::Current),
    }
}

fn resolve_axis(axis: R1C1Axis, base: u32) -> Result<u32, AddressError> {
    match axis {
        R1C1Axis::Absolute(value) => Ok(value - 1),
        R1C1Axis::Current => Ok(base),
        R1C1Axis::Relative(offset) => {
            let base_i = i64::from(base);
            let offset_i = i64::from(offset);
            let resolved = base_i + offset_i;
            if resolved < 0 {
                return Err(AddressError::InvalidRange(
                    "R1C1 reference resolved out of bounds".to_string(),
                ));
            }
            u32::try_from(resolved).map_err(|_| {
                AddressError::InvalidRange("R1C1 reference resolved out of bounds".to_string())
            })
        }
    }
}

fn axis_to_r1c1(prefix: char, axis: R1C1Axis) -> String {
    match axis {
        R1C1Axis::Absolute(value) => format!("{}{}", prefix, value),
        R1C1Axis::Relative(offset) => format!("{}[{}]", prefix, offset),
        R1C1Axis::Current => prefix.to_string(),
    }
}

impl CellRef {
    /// Parse A1 notation preserving absolute/mixed references (e.g., $A$1, A$1, $A1)
    pub fn from_a1(s: &str) -> Result<Self, AddressError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(AddressError::InvalidRange("Empty A1 reference".to_string()));
        }

        let mut chars = trimmed.chars().peekable();

        // Optional $ for absolute column
        let col_abs = if matches!(chars.peek(), Some('$')) {
            chars.next();
            true
        } else {
            false
        };

        // Parse column letters
        let mut col_letters = String::new();
        while let Some(ch) = chars.peek().copied() {
            if ch.is_ascii_alphabetic() {
                col_letters.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        if col_letters.is_empty() {
            return Err(AddressError::InvalidColumn(trimmed.to_string()));
        }

        // Optional $ for absolute row
        let row_abs = if matches!(chars.peek(), Some('$')) {
            chars.next();
            true
        } else {
            false
        };

        // Parse row digits
        let mut row_digits = String::new();
        while let Some(ch) = chars.peek().copied() {
            if ch.is_ascii_digit() {
                row_digits.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        if row_digits.is_empty() || chars.peek().is_some() {
            return Err(AddressError::InvalidRow(trimmed.to_string()));
        }

        let row_num: u32 = row_digits
            .parse()
            .map_err(|_| AddressError::InvalidRow(row_digits.clone()))?;

        if row_num == 0 {
            return Err(AddressError::InvalidRow(row_digits));
        }

        let col_index = column_letters_to_index(&col_letters)?;
        let addr = CellAddress {
            row: row_num - 1,
            col: col_index,
        };

        match (row_abs, col_abs) {
            (true, true) => Ok(CellRef::Absolute(addr)),
            (false, false) => Ok(CellRef::Relative(addr)),
            _ => Ok(CellRef::Mixed {
                row_abs,
                col_abs,
                addr,
            }),
        }
    }
}

/// Value types that can be stored in cells
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Empty,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Error(ErrorValue),
    Array(Vec<Value>),
}

/// Error types for cell values
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorValue {
    Div0,  // #DIV/0!
    Name,  // #NAME?
    Value, // #VALUE!
    Ref,   // #REF!
    Null,  // #NULL!
    Num,   // #NUM!
    NA,    // #N/A
}

impl ErrorValue {
    /// Excel-style error label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Div0 => "#DIV/0!",
            Self::Name => "#NAME?",
            Self::Value => "#VALUE!",
            Self::Ref => "#REF!",
            Self::Null => "#NULL!",
            Self::Num => "#NUM!",
            Self::NA => "#N/A",
        }
    }
}

/// Errors that can occur when parsing addresses
#[derive(Debug, thiserror::Error)]
pub enum AddressError {
    #[error("Invalid column: {0}")]
    InvalidColumn(String),
    #[error("Invalid row: {0}")]
    InvalidRow(String),
    #[error("Invalid range: {0}")]
    InvalidRange(String),
}

fn column_letters_to_index(col: &str) -> Result<u32, AddressError> {
    let mut result: u32 = 0;
    for ch in col.chars() {
        let upper = ch.to_ascii_uppercase();
        if !upper.is_ascii_alphabetic() {
            return Err(AddressError::InvalidColumn(col.to_string()));
        }
        let value = (upper as u8 - b'A' + 1) as u32;
        result = result
            .checked_mul(26)
            .and_then(|v| v.checked_add(value))
            .ok_or_else(|| AddressError::InvalidColumn(col.to_string()))?;
    }
    // Convert to zero-based index
    Ok(result - 1)
}

fn column_index_to_letters(mut index: u32) -> String {
    let mut letters = Vec::new();
    index += 1; // 1-based for conversion
    while index > 0 {
        let rem = ((index - 1) % 26) as u8;
        letters.push((b'A' + rem) as char);
        index = (index - 1) / 26;
    }
    letters.iter().rev().collect()
}

impl fmt::Display for CellAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

impl fmt::Display for CellRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.start, self.end)
    }
}

/// Iterator over a cell range in row-major order
pub struct CellRangeIter {
    current: CellAddress,
    start: CellAddress,
    end: CellAddress,
    done: bool,
}

impl Iterator for CellRangeIter {
    type Item = CellAddress;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let result = self.current;

        if self.current.row == self.end.row && self.current.col == self.end.col {
            self.done = true;
            return Some(result);
        }

        if self.current.col < self.end.col {
            self.current.col += 1;
        } else {
            self.current.col = self.start.col;
            self.current.row += 1;
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_range_contains() {
        let range = CellRange::new(
            CellAddress::new(1, 1), // B2
            CellAddress::new(3, 3), // D4
        );

        assert!(range.contains(&CellAddress::new(2, 2))); // C3
        assert!(!range.contains(&CellAddress::new(0, 0))); // A1
        assert!(!range.contains(&CellAddress::new(4, 4))); // E5
    }

    #[test]
    fn test_range_size() {
        let range = CellRange::new(
            CellAddress::new(0, 0), // A1
            CellAddress::new(2, 3), // D3
        );
        assert_eq!(range.size(), 12); // 3 rows * 4 cols
    }

    #[test]
    fn test_range_normalized_contains() {
        let range = CellRange::new(CellAddress::new(3, 3), CellAddress::new(1, 1));
        assert!(range.contains(&CellAddress::new(2, 2)));
        assert_eq!(range.rows(), 3);
        assert_eq!(range.cols(), 3);
    }

    #[test]
    fn test_range_iter() {
        let range = CellRange::new(CellAddress::new(0, 0), CellAddress::new(1, 1));
        let collected: Vec<_> = range.iter().collect();
        assert_eq!(
            collected,
            vec![
                CellAddress::new(0, 0),
                CellAddress::new(0, 1),
                CellAddress::new(1, 0),
                CellAddress::new(1, 1),
            ]
        );
    }

    #[test]
    fn test_a1_parse_basic() {
        let addr = CellAddress::from_a1("A1").unwrap();
        assert_eq!(addr.row, 0);
        assert_eq!(addr.col, 0);

        let addr = CellAddress::from_a1("C3").unwrap();
        assert_eq!(addr.row, 2);
        assert_eq!(addr.col, 2);

        let addr = CellAddress::from_a1("AA10").unwrap();
        assert_eq!(addr.row, 9);
        assert_eq!(addr.col, 26);
    }

    #[test]
    fn test_a1_parse_with_dollar() {
        let addr = CellAddress::from_a1("$B$2").unwrap();
        assert_eq!(addr.row, 1);
        assert_eq!(addr.col, 1);
    }

    #[test]
    fn test_a1_parse_invalid() {
        assert!(CellAddress::from_a1("").is_err());
        assert!(CellAddress::from_a1("1A").is_err());
        assert!(CellAddress::from_a1("A0").is_err());
        assert!(CellAddress::from_a1("A").is_err());
        assert!(CellAddress::from_a1("A1B").is_err());
    }

    #[test]
    fn test_a1_format() {
        let addr = CellAddress::new(0, 0);
        assert_eq!(addr.to_a1(), "A1");

        let addr = CellAddress::new(9, 26);
        assert_eq!(addr.to_a1(), "AA10");
    }

    #[test]
    fn test_r1c1_parsing_absolute() {
        let r = R1C1Ref::from_r1c1("R1C1").unwrap();
        assert!(r.is_absolute());
        let addr = r.resolve(None).unwrap();
        assert_eq!(addr, CellAddress::new(0, 0));

        let r = R1C1Ref::from_r1c1("R5C3").unwrap();
        let addr = r.resolve(None).unwrap();
        assert_eq!(addr, CellAddress::new(4, 2));
    }

    #[test]
    fn test_r1c1_parsing_relative() {
        let base = CellAddress::new(10, 10);
        let r = R1C1Ref::from_r1c1("R[1]C[-2]").unwrap();
        let addr = r.resolve(Some(base)).unwrap();
        assert_eq!(addr, CellAddress::new(11, 8));

        let r = R1C1Ref::from_r1c1("RC").unwrap();
        let addr = r.resolve(Some(base)).unwrap();
        assert_eq!(addr, base);
        assert_eq!(r.to_a1(Some(base)).unwrap(), "K11");
    }

    #[test]
    fn test_r1c1_errors() {
        assert!(R1C1Ref::from_r1c1("").is_err());
        assert!(R1C1Ref::from_r1c1("R0C1").is_err());
        assert!(R1C1Ref::from_r1c1("R1C0").is_err());
        assert!(R1C1Ref::from_r1c1("R[abc]C1").is_err());
        assert!(R1C1Ref::from_r1c1("R1").is_err());
    }

    #[test]
    fn test_r1c1_formatting() {
        let addr = CellAddress::new(0, 0);
        assert_eq!(addr.to_r1c1(), "R1C1");
        let r = R1C1Ref::from_r1c1("R[-1]C[2]").unwrap();
        assert_eq!(r.to_r1c1(), "R[-1]C[2]");
    }

    #[test]
    fn test_error_labels() {
        assert_eq!(ErrorValue::Div0.label(), "#DIV/0!");
        assert_eq!(ErrorValue::Name.label(), "#NAME?");
        assert_eq!(ErrorValue::Value.label(), "#VALUE!");
        assert_eq!(ErrorValue::Ref.label(), "#REF!");
        assert_eq!(ErrorValue::Null.label(), "#NULL!");
        assert_eq!(ErrorValue::Num.label(), "#NUM!");
        assert_eq!(ErrorValue::NA.label(), "#N/A");
    }

    #[test]
    fn test_display_impls() {
        let addr = CellAddress::new(0, 0);
        assert_eq!(format!("{addr}"), "A1");

        let range = CellRange::new(CellAddress::new(0, 0), CellAddress::new(1, 1));
        assert_eq!(format!("{range}"), "A1:B2");
    }

    #[test]
    fn test_column_letters_to_index_invalid() {
        assert!(column_letters_to_index("A!").is_err());
        let long_col = "Z".repeat(20);
        assert!(column_letters_to_index(&long_col).is_err());
    }

    #[test]
    fn test_cellref_parse_modes() {
        let cref = CellRef::from_a1("A1").unwrap();
        assert!(matches!(cref, CellRef::Relative(_)));
        if let CellRef::Relative(addr) = cref {
            assert_eq!(addr.row, 0);
            assert_eq!(addr.col, 0);
        }

        let cref = CellRef::from_a1("$A$1").unwrap();
        assert!(matches!(cref, CellRef::Absolute(_)));
        if let CellRef::Absolute(addr) = cref {
            assert_eq!(addr.row, 0);
            assert_eq!(addr.col, 0);
        }

        let cref = CellRef::from_a1("$A1").unwrap();
        assert!(matches!(cref, CellRef::Mixed { .. }));
        if let CellRef::Mixed {
            row_abs,
            col_abs,
            addr,
        } = cref
        {
            assert!(!row_abs);
            assert!(col_abs);
            assert_eq!(addr.row, 0);
            assert_eq!(addr.col, 0);
        }

        let cref = CellRef::from_a1("A$1").unwrap();
        assert!(matches!(cref, CellRef::Mixed { .. }));
        if let CellRef::Mixed {
            row_abs,
            col_abs,
            addr,
        } = cref
        {
            assert!(row_abs);
            assert!(!col_abs);
            assert_eq!(addr.row, 0);
            assert_eq!(addr.col, 0);
        }
    }

    #[test]
    fn test_cellref_parse_invalid() {
        assert!(CellRef::from_a1("").is_err());
        assert!(CellRef::from_a1("$1").is_err()); // no column letters
        assert!(CellRef::from_a1("A").is_err()); // no row digits
        assert!(CellRef::from_a1("A0").is_err()); // invalid row
        assert!(CellRef::from_a1("A1B").is_err()); // trailing chars
    }
}
