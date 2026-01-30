//! Number and text formatting utilities

/// Number format types
#[derive(Debug, Clone)]
pub enum NumberFormat {
    General,
    Number { decimals: usize, use_separator: bool },
    Currency { symbol: String, decimals: usize },
    Percentage { decimals: usize },
    Scientific { decimals: usize },
    Date(String),
    Time(String),
    Custom(String),
}

impl NumberFormat {
    /// Parse a format string into a NumberFormat
    pub fn parse(format: &str) -> Self {
        match format {
            "General" | "" => NumberFormat::General,
            "0" => NumberFormat::Number { decimals: 0, use_separator: false },
            "#,##0" => NumberFormat::Number { decimals: 0, use_separator: true },
            "0.00" => NumberFormat::Number { decimals: 2, use_separator: false },
            "#,##0.00" => NumberFormat::Number { decimals: 2, use_separator: true },
            "0%" => NumberFormat::Percentage { decimals: 0 },
            "0.00%" => NumberFormat::Percentage { decimals: 2 },
            s if s.starts_with('$') => NumberFormat::Currency {
                symbol: "$".to_string(),
                decimals: 2,
            },
            _ => NumberFormat::Custom(format.to_string()),
        }
    }
}