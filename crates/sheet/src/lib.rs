//! Sheet/Book module for piptable
//!
//! Provides a pyexcel-like API for working with tabular data.
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

mod book;
mod cell;
mod csv;
mod error;
mod json;
mod sheet;
mod toon;
mod xlsx;

pub use book::Book;
pub use cell::CellValue;
pub use csv::CsvOptions;
pub use error::{Result, SheetError};
pub use sheet::Sheet;
pub use xlsx::XlsxReadOptions;
