//! Sheet/Book module for piptable
//!
//! Provides a pyexcel-like API for working with tabular data, including support for
//! Excel-style formulas, multiple file formats, and data manipulation operations.
//!
//! # Examples
//!
//! ## Creating a sheet from data
//!
//! ```
//! use piptable_sheet::{Sheet, CellValue};
//!
//! let sheet = Sheet::from_data(vec![
//!     vec!["Name", "Age", "City"],
//!     vec!["Alice", "30", "NYC"],
//!     vec!["Bob", "25", "LA"],
//! ]);
//!
//! assert_eq!(sheet.row_count(), 3);
//! assert_eq!(sheet.col_count(), 3);
//! ```
//!
//! ## Using formulas
//!
//! ```
//! use piptable_sheet::Sheet;
//!
//! let mut sheet = Sheet::from_data(vec![
//!     vec![10, 20, 0],
//!     vec![30, 40, 0],
//! ]);
//!
//! // Add formulas
//! sheet.set_formula("C1", "=A1+B1").unwrap();
//! sheet.set_formula("C2", "=SUM(A2:B2)").unwrap();
//!
//! // Evaluate all formulas
//! sheet.evaluate_formulas().unwrap();
//!
//! // Access results
//! assert_eq!(sheet.get_a1("C1").unwrap().as_float(), Some(30.0));
//! assert_eq!(sheet.get_a1("C2").unwrap().as_float(), Some(70.0));
//! ```
//!
//! ## Loading from CSV
//!
//! ```no_run
//! use piptable_sheet::Sheet;
//!
//! let sheet = Sheet::from_csv("data.csv").unwrap();
//! ```
//!
//! ## Named column access
//!
//! ```
//! use piptable_sheet::Sheet;
//!
//! let mut sheet = Sheet::from_data(vec![
//!     vec!["Name", "Age"],
//!     vec!["Alice", "30"],
//! ]);
//!
//! sheet.name_columns_by_row(0).unwrap();
//! let ages = sheet.column_by_name("Age").unwrap();
//! ```
//!
//! ## Working with books
//!
//! ```
//! use piptable_sheet::{Book, Sheet};
//!
//! let mut book = Book::new();
//! book.add_sheet("Data", Sheet::new()).unwrap();
//! book.add_sheet("Summary", Sheet::new()).unwrap();
//!
//! assert_eq!(book.sheet_count(), 2);
//! ```
//!
//! # Formula Support
//!
//! The library includes a comprehensive formula engine supporting:
//! - Basic arithmetic operations (+, -, *, /, ^)
//! - Mathematical functions (SUM, AVERAGE, MIN, MAX, ROUND, etc.)
//! - String functions (CONCATENATE, UPPER, LOWER, TRIM, etc.)
//! - Logical functions (IF, AND, OR, NOT)
//! - Date/time functions (DATE, NOW, TODAY)
//! - Lookup functions (VLOOKUP, HLOOKUP, INDEX, MATCH)
//! - Statistical functions (COUNT, COUNTA, STDEV, VAR)
//!
//! Formulas support cell references (A1, B2), ranges (A1:B3), and automatic
//! recalculation when dependent cells change.

mod a1_notation;
mod book;
mod cell;
mod csv;
mod error;
#[cfg(not(target_arch = "wasm32"))]
mod html;
mod json;
#[cfg(not(target_arch = "wasm32"))]
mod parquet;
mod sheet;
mod toon;
#[cfg(not(target_arch = "wasm32"))]
mod xlsx;

/// Re-export book types and options.
pub use book::{Book, ConsolidateOptions, FileLoadOptions};
/// Re-export cell value type.
pub use cell::CellValue;
/// Re-export CSV options.
pub use csv::CsvOptions;
/// Re-export sheet error types.
pub use error::{Result, SheetError};
/// Re-export sheet type.
pub use sheet::{CleanOptions, NullStrategy, Sheet, ValidationRule};
#[cfg(not(target_arch = "wasm32"))]
/// Re-export XLSX read options (non-WASM only).
pub use xlsx::XlsxReadOptions;
