use thiserror::Error;

/// Errors that can occur during sheet operations
#[derive(Error, Debug)]
pub enum SheetError {
    #[error("Index out of bounds: row {row}, col {col} (sheet has {rows} rows, {cols} cols)")]
    IndexOutOfBounds {
        row: usize,
        col: usize,
        rows: usize,
        cols: usize,
    },

    #[error("Row index out of bounds: {index} (sheet has {count} rows)")]
    RowIndexOutOfBounds { index: usize, count: usize },

    #[error("Column index out of bounds: {index} (sheet has {count} columns)")]
    ColumnIndexOutOfBounds { index: usize, count: usize },

    #[error("Column not found: {name}")]
    ColumnNotFound { name: String },

    #[error("Row not found: {name}")]
    RowNotFound { name: String },

    #[error("Sheet not found: {name}")]
    SheetNotFound { name: String },

    #[error("Sheet already exists: {name}")]
    SheetAlreadyExists { name: String },

    #[error("Columns not named. Call name_columns_by_row() first")]
    ColumnsNotNamed,

    #[error("Rows not named. Call name_rows_by_column() first")]
    RowsNotNamed,

    #[error("Data length mismatch: expected {expected}, got {actual}")]
    LengthMismatch { expected: usize, actual: usize },

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SheetError>;
