use thiserror::Error;

#[derive(Debug, Error)]
pub enum PdfError {
    #[error("Failed to read PDF file: {0}")]
    ReadError(String),
    
    #[error("Failed to parse PDF: {0}")]
    ParseError(String),
    
    #[error("No tables found in PDF")]
    NoTablesFound,
    
    #[error("OCR error: {0}")]
    OcrError(String),
    
    #[error("Invalid page range: {0}")]
    InvalidPageRange(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("PDF extraction error: {0}")]
    ExtractionError(String),
}

pub type Result<T> = std::result::Result<T, PdfError>;