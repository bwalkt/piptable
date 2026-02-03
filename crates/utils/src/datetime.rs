//! Date and time utilities for spreadsheet operations

use chrono::{DateTime, Utc};

/// Excel epoch (January 1, 1900)
/// Note: Excel incorrectly treats 1900 as a leap year
const EXCEL_EPOCH: i32 = 25569; // Days between 1900-01-01 and 1970-01-01

/// Convert Excel serial date to DateTime
pub fn excel_date_to_datetime(serial: f64) -> Option<DateTime<Utc>> {
    // Excel dates are days since 1900-01-01
    // Need to adjust for the 1900 leap year bug
    let days = serial.floor() as i64;
    let time_fraction = serial - serial.floor();

    // Convert to Unix timestamp
    let unix_days = days - EXCEL_EPOCH as i64;
    let unix_seconds = unix_days * 86400 + (time_fraction * 86400.0) as i64;

    DateTime::from_timestamp(unix_seconds, 0)
}

/// Convert DateTime to Excel serial date
pub fn datetime_to_excel_date(dt: DateTime<Utc>) -> f64 {
    let unix_seconds = dt.timestamp();
    let unix_days = unix_seconds / 86400;
    let time_fraction = (unix_seconds % 86400) as f64 / 86400.0;

    (unix_days + EXCEL_EPOCH as i64) as f64 + time_fraction
}

/// Format date according to pattern
pub fn format_date(dt: DateTime<Utc>, pattern: &str) -> String {
    // Common Excel date formats
    match pattern {
        "mm/dd/yyyy" => dt.format("%m/%d/%Y").to_string(),
        "dd/mm/yyyy" => dt.format("%d/%m/%Y").to_string(),
        "yyyy-mm-dd" => dt.format("%Y-%m-%d").to_string(),
        "mmm dd, yyyy" => dt.format("%b %d, %Y").to_string(),
        _ => dt.format(pattern).to_string(),
    }
}

/// Datetime helper tests.
#[cfg(test)]
mod tests {
    use super::*;

/// Verifies Excel date conversion.
    #[test]
    fn test_excel_date_conversion() {
        // Test a known date
        let serial = 44562.0; // January 1, 2022
        let dt = excel_date_to_datetime(serial);
        assert!(dt.is_some());

        // Round trip
        if let Some(dt) = dt {
            let serial2 = datetime_to_excel_date(dt);
            assert!((serial - serial2).abs() < 0.001);
        }
    }
}
