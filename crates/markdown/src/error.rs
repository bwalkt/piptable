use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarkdownError {
    #[error("Failed to parse markdown: {0}")]
    ParseError(String),

    #[error("No tables found in markdown")]
    NoTablesFound,

    #[error("Invalid table structure")]
    InvalidTable,

    #[error("Sheet error: {0}")]
    SheetError(#[from] piptable_sheet::SheetError),
}

pub type Result<T> = std::result::Result<T, MarkdownError>;
