use crate::book::Book;
use crate::cell::CellValue;
use crate::error::{Result, SheetError};
use crate::sheet::Sheet;
use calamine::{
    open_workbook, open_workbook_auto, Data, Error as CalamineError, Reader, Sheets, Xls, XlsError,
    Xlsx, XlsxError,
};
use rust_xlsxwriter::{Workbook, Worksheet};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Options for reading Excel files
#[derive(Debug, Clone, Default)]
pub struct XlsxReadOptions {
    /// Whether the first row contains headers
    pub has_headers: bool,
}

impl XlsxReadOptions {
    /// Set whether the first row contains headers
    #[must_use]
    pub fn with_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }
}

/// Convert calamine Data to CellValue
fn data_to_cell_value(data: &Data) -> CellValue {
    match data {
        Data::Empty => CellValue::Null,
        Data::Bool(b) => CellValue::Bool(*b),
        Data::Int(i) => CellValue::Int(*i),
        Data::Float(f) => CellValue::Float(*f),
        Data::String(s) => CellValue::String(s.clone()),
        Data::DateTime(dt) => {
            // Convert Excel datetime to float
            // Excel stores dates as days since 1899-12-30
            CellValue::Float(dt.as_f64())
        }
        Data::DateTimeIso(s) => CellValue::String(s.clone()),
        Data::DurationIso(s) => CellValue::String(s.clone()),
        Data::Error(e) => CellValue::String(format!("#ERROR: {e:?}")),
    }
}

/// Build a Sheet from raw data with optional header handling
fn build_sheet(
    sheet_name: &str,
    data: Vec<Vec<CellValue>>,
    options: &XlsxReadOptions,
) -> Result<Sheet> {
    let mut sheet = Sheet::with_name(sheet_name);
    *sheet.data_mut() = data;

    if options.has_headers && sheet.row_count() > 0 {
        sheet.name_columns_by_row(0)?;
    }

    Ok(sheet)
}

/// Build a Sheet from raw data, ignoring header naming errors.
///
/// This is used for Book loading where we don't want a single sheet's
/// duplicate column names to fail the entire book load. The asymmetry
/// with `build_sheet` (which propagates errors) is intentional:
/// - Single Sheet loading: caller expects to work with that specific sheet,
///   so header errors should be reported
/// - Book loading: caller wants all sheets, and some may have duplicate
///   headers that don't affect their use case
fn build_sheet_lenient(
    sheet_name: &str,
    data: Vec<Vec<CellValue>>,
    options: &XlsxReadOptions,
) -> Sheet {
    let mut sheet = Sheet::with_name(sheet_name);
    *sheet.data_mut() = data;

    if options.has_headers && sheet.row_count() > 0 {
        // Ignore duplicate column name errors when loading books
        let _ = sheet.name_columns_by_row(0);
    }

    sheet
}

impl Sheet {
    /// Load a sheet from an Excel file (first sheet)
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_xlsx<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_xlsx_with_options(path, XlsxReadOptions::default())
    }

    /// Load a sheet from an Excel file with options
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_xlsx_with_options<P: AsRef<Path>>(
        path: P,
        options: XlsxReadOptions,
    ) -> Result<Self> {
        let workbook: Xlsx<BufReader<File>> =
            open_workbook(path.as_ref()).map_err(|e: XlsxError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let sheet_names = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            return Ok(Sheet::new());
        }

        Self::from_xlsx_sheet_with_options(path, &sheet_names[0], options)
    }

    /// Load a specific sheet from an Excel file by name
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened, sheet not found, or read fails.
    pub fn from_xlsx_sheet<P: AsRef<Path>>(path: P, sheet_name: &str) -> Result<Self> {
        Self::from_xlsx_sheet_with_options(path, sheet_name, XlsxReadOptions::default())
    }

    /// Load a specific sheet from an Excel file with options
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened, sheet not found, or read fails.
    pub fn from_xlsx_sheet_with_options<P: AsRef<Path>>(
        path: P,
        sheet_name: &str,
        options: XlsxReadOptions,
    ) -> Result<Self> {
        let mut workbook: Xlsx<BufReader<File>> =
            open_workbook(path.as_ref()).map_err(|e: XlsxError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e: XlsxError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let data: Vec<Vec<CellValue>> = range
            .rows()
            .map(|row| row.iter().map(data_to_cell_value).collect())
            .collect();

        build_sheet(sheet_name, data, &options)
    }

    /// Save the sheet to an Excel file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be created or written.
    pub fn save_as_xlsx<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        self.write_to_worksheet(worksheet)?;

        workbook.save(path.as_ref()).map_err(|e| {
            SheetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        Ok(())
    }

    // =========================================================================
    // XLS (Legacy Excel 97-2003) Support
    // =========================================================================

    /// Load a sheet from a legacy Excel file (.xls) - first sheet
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_xls<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_xls_with_options(path, XlsxReadOptions::default())
    }

    /// Load a sheet from a legacy Excel file with options
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_xls_with_options<P: AsRef<Path>>(
        path: P,
        options: XlsxReadOptions,
    ) -> Result<Self> {
        let workbook: Xls<BufReader<File>> =
            open_workbook(path.as_ref()).map_err(|e: XlsError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let sheet_names = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            return Ok(Sheet::new());
        }

        Self::from_xls_sheet_with_options(path, &sheet_names[0], options)
    }

    /// Load a specific sheet from a legacy Excel file by name
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened, sheet not found, or read fails.
    pub fn from_xls_sheet<P: AsRef<Path>>(path: P, sheet_name: &str) -> Result<Self> {
        Self::from_xls_sheet_with_options(path, sheet_name, XlsxReadOptions::default())
    }

    /// Load a specific sheet from a legacy Excel file with options
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened, sheet not found, or read fails.
    pub fn from_xls_sheet_with_options<P: AsRef<Path>>(
        path: P,
        sheet_name: &str,
        options: XlsxReadOptions,
    ) -> Result<Self> {
        let mut workbook: Xls<BufReader<File>> =
            open_workbook(path.as_ref()).map_err(|e: XlsError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e: XlsError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let data: Vec<Vec<CellValue>> = range
            .rows()
            .map(|row| row.iter().map(data_to_cell_value).collect())
            .collect();

        build_sheet(sheet_name, data, &options)
    }

    // =========================================================================
    // Auto-detect Excel format (XLS or XLSX)
    // =========================================================================

    /// Load a sheet from an Excel file, auto-detecting format (.xls or .xlsx)
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_excel<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_excel_with_options(path, XlsxReadOptions::default())
    }

    /// Load a sheet from an Excel file with options, auto-detecting format
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_excel_with_options<P: AsRef<Path>>(
        path: P,
        options: XlsxReadOptions,
    ) -> Result<Self> {
        let mut workbook: Sheets<BufReader<File>> =
            open_workbook_auto(path.as_ref()).map_err(|e: CalamineError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            return Ok(Sheet::new());
        }

        let sheet_name = &sheet_names[0];
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e: CalamineError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let data: Vec<Vec<CellValue>> = range
            .rows()
            .map(|row| row.iter().map(data_to_cell_value).collect())
            .collect();

        build_sheet(sheet_name, data, &options)
    }

    /// Write sheet data to a worksheet
    fn write_to_worksheet(&self, worksheet: &mut Worksheet) -> Result<()> {
        worksheet.set_name(self.name()).map_err(|e| {
            SheetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        for (row_idx, row) in self.data().iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                let row_num = u32::try_from(row_idx).map_err(|_| {
                    SheetError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Row index overflow",
                    ))
                })?;
                let col_num = u16::try_from(col_idx).map_err(|_| {
                    SheetError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Column index overflow",
                    ))
                })?;

                match cell {
                    CellValue::Null => {} // Leave empty
                    CellValue::Bool(b) => {
                        worksheet.write_boolean(row_num, col_num, *b).map_err(|e| {
                            SheetError::Io(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string(),
                            ))
                        })?;
                    }
                    CellValue::Int(i) => {
                        // Note: Excel stores all numbers as f64, so integers > 2^53
                        // (9,007,199,254,740,992) may lose precision
                        worksheet
                            .write_number(row_num, col_num, *i as f64)
                            .map_err(|e| {
                                SheetError::Io(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e.to_string(),
                                ))
                            })?;
                    }
                    CellValue::Float(f) => {
                        worksheet.write_number(row_num, col_num, *f).map_err(|e| {
                            SheetError::Io(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string(),
                            ))
                        })?;
                    }
                    CellValue::String(s) => {
                        worksheet.write_string(row_num, col_num, s).map_err(|e| {
                            SheetError::Io(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string(),
                            ))
                        })?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Book {
    /// Load a book from an Excel file (all sheets)
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_xlsx<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_xlsx_with_options(path, XlsxReadOptions::default())
    }

    /// Load a book from an Excel file with options
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_xlsx_with_options<P: AsRef<Path>>(
        path: P,
        options: XlsxReadOptions,
    ) -> Result<Self> {
        let mut workbook: Xlsx<BufReader<File>> =
            open_workbook(path.as_ref()).map_err(|e: XlsxError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let sheet_names: Vec<String> = workbook
            .sheet_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut book = Book::new();

        for sheet_name in sheet_names {
            let range = workbook
                .worksheet_range(&sheet_name)
                .map_err(|e: XlsxError| {
                    SheetError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })?;

            let data: Vec<Vec<CellValue>> = range
                .rows()
                .map(|row| row.iter().map(data_to_cell_value).collect())
                .collect();

            let sheet = build_sheet_lenient(&sheet_name, data, &options);
            book.add_sheet(&sheet_name, sheet)?;
        }

        Ok(book)
    }

    /// Save the book to an Excel file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be created or written.
    pub fn save_as_xlsx<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut workbook = Workbook::new();

        for (name, sheet) in self.sheets() {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(name).map_err(|e| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

            for (row_idx, row) in sheet.data().iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    let row_num = u32::try_from(row_idx).map_err(|_| {
                        SheetError::Io(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Row index overflow",
                        ))
                    })?;
                    let col_num = u16::try_from(col_idx).map_err(|_| {
                        SheetError::Io(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Column index overflow",
                        ))
                    })?;

                    match cell {
                        CellValue::Null => {}
                        CellValue::Bool(b) => {
                            worksheet.write_boolean(row_num, col_num, *b).map_err(|e| {
                                SheetError::Io(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e.to_string(),
                                ))
                            })?;
                        }
                        CellValue::Int(i) => {
                            // Note: Excel stores all numbers as f64, so integers > 2^53
                            // (9,007,199,254,740,992) may lose precision
                            worksheet
                                .write_number(row_num, col_num, *i as f64)
                                .map_err(|e| {
                                    SheetError::Io(std::io::Error::new(
                                        std::io::ErrorKind::Other,
                                        e.to_string(),
                                    ))
                                })?;
                        }
                        CellValue::Float(f) => {
                            worksheet.write_number(row_num, col_num, *f).map_err(|e| {
                                SheetError::Io(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e.to_string(),
                                ))
                            })?;
                        }
                        CellValue::String(s) => {
                            worksheet.write_string(row_num, col_num, s).map_err(|e| {
                                SheetError::Io(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e.to_string(),
                                ))
                            })?;
                        }
                    }
                }
            }
        }

        workbook.save(path.as_ref()).map_err(|e| {
            SheetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        Ok(())
    }

    /// Get sheet names from an Excel file without loading data
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened.
    pub fn xlsx_sheet_names<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
        let workbook: Xlsx<BufReader<File>> =
            open_workbook(path.as_ref()).map_err(|e: XlsxError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        Ok(workbook
            .sheet_names()
            .iter()
            .map(|s| s.to_string())
            .collect())
    }

    // =========================================================================
    // XLS (Legacy Excel 97-2003) Support
    // =========================================================================

    /// Load a book from a legacy Excel file (.xls) - all sheets
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_xls<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_xls_with_options(path, XlsxReadOptions::default())
    }

    /// Load a book from a legacy Excel file with options
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_xls_with_options<P: AsRef<Path>>(
        path: P,
        options: XlsxReadOptions,
    ) -> Result<Self> {
        let mut workbook: Xls<BufReader<File>> =
            open_workbook(path.as_ref()).map_err(|e: XlsError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let sheet_names: Vec<String> = workbook
            .sheet_names()
            .iter()
            .map(|s: &String| s.to_string())
            .collect();
        let mut book = Book::new();

        for sheet_name in sheet_names {
            let range = workbook
                .worksheet_range(&sheet_name)
                .map_err(|e: XlsError| {
                    SheetError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })?;

            let data: Vec<Vec<CellValue>> = range
                .rows()
                .map(|row| row.iter().map(data_to_cell_value).collect())
                .collect();

            let sheet = build_sheet_lenient(&sheet_name, data, &options);
            book.add_sheet(&sheet_name, sheet)?;
        }

        Ok(book)
    }

    /// Get sheet names from a legacy Excel file without loading data
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened.
    pub fn xls_sheet_names<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
        let workbook: Xls<BufReader<File>> =
            open_workbook(path.as_ref()).map_err(|e: XlsError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        Ok(workbook
            .sheet_names()
            .iter()
            .map(|s: &String| s.to_string())
            .collect())
    }

    // =========================================================================
    // Auto-detect Excel format (XLS or XLSX)
    // =========================================================================

    /// Load a book from an Excel file, auto-detecting format (.xls or .xlsx)
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_excel<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_excel_with_options(path, XlsxReadOptions::default())
    }

    /// Load a book from an Excel file with options, auto-detecting format
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or read.
    pub fn from_excel_with_options<P: AsRef<Path>>(
        path: P,
        options: XlsxReadOptions,
    ) -> Result<Self> {
        let mut workbook: Sheets<BufReader<File>> =
            open_workbook_auto(path.as_ref()).map_err(|e: CalamineError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
        let mut book = Book::new();

        for sheet_name in sheet_names {
            let range = workbook
                .worksheet_range(&sheet_name)
                .map_err(|e: CalamineError| {
                    SheetError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })?;

            let data: Vec<Vec<CellValue>> = range
                .rows()
                .map(|row| row.iter().map(data_to_cell_value).collect())
                .collect();

            let sheet = build_sheet_lenient(&sheet_name, data, &options);
            book.add_sheet(&sheet_name, sheet)?;
        }

        Ok(book)
    }

    /// Get sheet names from an Excel file without loading data, auto-detecting format
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened.
    pub fn excel_sheet_names<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
        let workbook: Sheets<BufReader<File>> =
            open_workbook_auto(path.as_ref()).map_err(|e: CalamineError| {
                SheetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        Ok(workbook.sheet_names().to_vec())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::approx_constant)]
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_xlsx_write_and_read() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.xlsx");

        // Create and save
        let sheet = Sheet::from_data(vec![
            vec!["Name", "Age", "Active"],
            vec!["Alice", "30", "true"],
            vec!["Bob", "25", "false"],
        ]);

        sheet.save_as_xlsx(&path).unwrap();

        // Read back
        let loaded = Sheet::from_xlsx(&path).unwrap();

        assert_eq!(loaded.row_count(), 3);
        assert_eq!(loaded.col_count(), 3);
    }

    #[test]
    fn test_xlsx_types() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("types.xlsx");

        let mut sheet = Sheet::new();
        *sheet.data_mut() = vec![vec![
            CellValue::String("text".to_string()),
            CellValue::Int(42),
            CellValue::Float(3.14),
            CellValue::Bool(true),
            CellValue::Null,
        ]];

        sheet.save_as_xlsx(&path).unwrap();

        let loaded = Sheet::from_xlsx(&path).unwrap();

        assert_eq!(loaded.row_count(), 1);
        // Note: trailing empty cells are not preserved in Excel files
        assert_eq!(loaded.col_count(), 4);

        // Verify string
        assert!(matches!(loaded.get(0, 0).unwrap(), CellValue::String(s) if s == "text"));

        // Verify number (Int becomes Float in Excel)
        assert!(
            matches!(loaded.get(0, 1).unwrap(), CellValue::Float(f) if (*f - 42.0).abs() < 0.01)
        );

        // Verify float
        assert!(
            matches!(loaded.get(0, 2).unwrap(), CellValue::Float(f) if (*f - 3.14).abs() < 0.01)
        );

        // Verify bool
        assert!(matches!(loaded.get(0, 3).unwrap(), CellValue::Bool(true)));
    }

    #[test]
    fn test_book_xlsx_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("book.xlsx");

        // Create book with multiple sheets
        let mut book = Book::new();

        let sheet1 = Sheet::from_data(vec![vec![1, 2, 3]]);
        let sheet2 = Sheet::from_data(vec![vec!["a", "b", "c"]]);

        book.add_sheet("Numbers", sheet1).unwrap();
        book.add_sheet("Letters", sheet2).unwrap();

        book.save_as_xlsx(&path).unwrap();

        // Read back
        let loaded = Book::from_xlsx(&path).unwrap();

        assert_eq!(loaded.sheet_count(), 2);
        assert!(loaded.has_sheet("Numbers"));
        assert!(loaded.has_sheet("Letters"));
    }

    #[test]
    fn test_xlsx_sheet_names() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("multi.xlsx");

        let mut book = Book::new();
        book.add_sheet("First", Sheet::new()).unwrap();
        book.add_sheet("Second", Sheet::new()).unwrap();
        book.add_sheet("Third", Sheet::new()).unwrap();

        book.save_as_xlsx(&path).unwrap();

        let names = Book::xlsx_sheet_names(&path).unwrap();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"First".to_string()));
        assert!(names.contains(&"Second".to_string()));
        assert!(names.contains(&"Third".to_string()));
    }

    #[test]
    fn test_xlsx_specific_sheet() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("specific.xlsx");

        let mut book = Book::new();
        book.add_sheet("Data", Sheet::from_data(vec![vec![1, 2, 3]]))
            .unwrap();
        book.add_sheet("Other", Sheet::from_data(vec![vec![4, 5, 6]]))
            .unwrap();

        book.save_as_xlsx(&path).unwrap();

        // Load specific sheet
        let sheet = Sheet::from_xlsx_sheet(&path, "Other").unwrap();

        assert_eq!(sheet.name(), "Other");
        assert_eq!(sheet.row_count(), 1);
    }

    #[test]
    fn test_xlsx_with_headers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("headers.xlsx");

        // Create sheet with header row
        let sheet = Sheet::from_data(vec![
            vec!["Name", "Age", "City"],
            vec!["Alice", "30", "NYC"],
            vec!["Bob", "25", "LA"],
        ]);

        sheet.save_as_xlsx(&path).unwrap();

        // Load without headers option - columns should not be named
        let no_headers = Sheet::from_xlsx(&path).unwrap();
        assert!(no_headers.column_names().is_none());

        // Load with headers option - first row becomes column names
        let with_headers =
            Sheet::from_xlsx_with_options(&path, XlsxReadOptions::default().with_headers(true))
                .unwrap();

        assert!(with_headers.column_names().is_some());
        let names = with_headers.column_names().unwrap();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "Name");
        assert_eq!(names[1], "Age");
        assert_eq!(names[2], "City");

        // Verify column access by name works
        let ages = with_headers.column_by_name("Age").unwrap();
        assert_eq!(ages.len(), 3); // includes header row in data
    }

    #[test]
    fn test_book_xlsx_with_headers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("book_headers.xlsx");

        let mut book = Book::new();
        book.add_sheet(
            "Data",
            Sheet::from_data(vec![
                vec!["Product", "Price"],
                vec!["Widget", "10"],
                vec!["Gadget", "20"],
            ]),
        )
        .unwrap();

        book.save_as_xlsx(&path).unwrap();

        // Load book with headers
        let loaded =
            Book::from_xlsx_with_options(&path, XlsxReadOptions::default().with_headers(true))
                .unwrap();

        let sheet = loaded.get_sheet("Data").unwrap();
        assert!(sheet.column_names().is_some());

        let prices = sheet.column_by_name("Price").unwrap();
        assert_eq!(prices.len(), 3);
    }

    // =========================================================================
    // Auto-detect format tests (from_excel)
    // =========================================================================

    #[test]
    fn test_sheet_from_excel_auto_detect() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("auto.xlsx");

        // Create and save as xlsx
        let sheet = Sheet::from_data(vec![vec!["Name", "Value"], vec!["Test", "123"]]);
        sheet.save_as_xlsx(&path).unwrap();

        // Load using auto-detect (should recognize xlsx)
        let loaded = Sheet::from_excel(&path).unwrap();
        assert_eq!(loaded.row_count(), 2);
        assert_eq!(loaded.col_count(), 2);
    }

    #[test]
    fn test_sheet_from_excel_with_options() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("auto_headers.xlsx");

        let sheet = Sheet::from_data(vec![vec!["Col1", "Col2"], vec!["A", "B"]]);
        sheet.save_as_xlsx(&path).unwrap();

        let loaded =
            Sheet::from_excel_with_options(&path, XlsxReadOptions::default().with_headers(true))
                .unwrap();

        assert!(loaded.column_names().is_some());
        let names = loaded.column_names().unwrap();
        assert_eq!(names[0], "Col1");
        assert_eq!(names[1], "Col2");
    }

    #[test]
    fn test_book_from_excel_auto_detect() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("book_auto.xlsx");

        let mut book = Book::new();
        book.add_sheet("Sheet1", Sheet::from_data(vec![vec![1, 2]]))
            .unwrap();
        book.add_sheet("Sheet2", Sheet::from_data(vec![vec![3, 4]]))
            .unwrap();
        book.save_as_xlsx(&path).unwrap();

        // Load using auto-detect
        let loaded = Book::from_excel(&path).unwrap();
        assert_eq!(loaded.sheet_count(), 2);
        assert!(loaded.has_sheet("Sheet1"));
        assert!(loaded.has_sheet("Sheet2"));
    }

    #[test]
    fn test_book_excel_sheet_names() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("names_auto.xlsx");

        let mut book = Book::new();
        book.add_sheet("Alpha", Sheet::new()).unwrap();
        book.add_sheet("Beta", Sheet::new()).unwrap();
        book.save_as_xlsx(&path).unwrap();

        // Get sheet names using auto-detect
        let names = Book::excel_sheet_names(&path).unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Alpha".to_string()));
        assert!(names.contains(&"Beta".to_string()));
    }

    // =========================================================================
    // XLS (Legacy Excel 97-2003) format tests
    // =========================================================================

    /// Path to the XLS test fixture.
    /// Panics if the fixture file is missing (it should be committed to the repo).
    fn xls_fixture_path() -> std::path::PathBuf {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("sample.xls");
        assert!(
            path.exists(),
            "XLS test fixture not found at {:?}. This file should be committed to the repo.",
            path
        );
        path
    }

    #[test]
    fn test_xls_read() {
        let path = xls_fixture_path();
        let sheet = Sheet::from_xls(&path).unwrap();
        assert_eq!(sheet.row_count(), 3);
        assert_eq!(sheet.col_count(), 3);
        assert_eq!(sheet.name(), "Data");
    }

    #[test]
    fn test_xls_with_headers() {
        let path = xls_fixture_path();
        let sheet =
            Sheet::from_xls_with_options(&path, XlsxReadOptions::default().with_headers(true))
                .unwrap();

        assert!(sheet.column_names().is_some());
        let names = sheet.column_names().unwrap();
        assert_eq!(names[0], "Name");
        assert_eq!(names[1], "Age");
        assert_eq!(names[2], "City");
    }

    #[test]
    fn test_xls_specific_sheet() {
        let path = xls_fixture_path();
        let sheet = Sheet::from_xls_sheet(&path, "Numbers").unwrap();
        assert_eq!(sheet.name(), "Numbers");
        assert_eq!(sheet.row_count(), 1);
        assert_eq!(sheet.col_count(), 3);
    }

    #[test]
    fn test_book_from_xls() {
        let path = xls_fixture_path();
        let book = Book::from_xls(&path).unwrap();
        assert_eq!(book.sheet_count(), 2);
        assert!(book.has_sheet("Data"));
        assert!(book.has_sheet("Numbers"));
    }

    #[test]
    fn test_xls_sheet_names() {
        let path = xls_fixture_path();
        let names = Book::xls_sheet_names(&path).unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Data".to_string()));
        assert!(names.contains(&"Numbers".to_string()));
    }

    #[test]
    fn test_xls_auto_detect() {
        let path = xls_fixture_path();
        // Auto-detect should work with .xls files
        let sheet = Sheet::from_excel(&path).unwrap();
        assert_eq!(sheet.row_count(), 3);

        let book = Book::from_excel(&path).unwrap();
        assert_eq!(book.sheet_count(), 2);
    }
}
