use crate::book::Book;
use crate::cell::CellValue;
use crate::error::{Result, SheetError};
use crate::sheet::Sheet;
use calamine::{open_workbook, Data, Reader, Xlsx, XlsxError};
use rust_xlsxwriter::{Workbook, Worksheet};
use std::io::BufReader;
use std::fs::File;
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
    pub fn from_xlsx_with_options<P: AsRef<Path>>(path: P, options: XlsxReadOptions) -> Result<Self> {
        let workbook: Xlsx<BufReader<File>> = open_workbook(path.as_ref())
            .map_err(|e: XlsxError| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

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
        let mut workbook: Xlsx<BufReader<File>> = open_workbook(path.as_ref())
            .map_err(|e: XlsxError| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e: XlsxError| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let mut data: Vec<Vec<CellValue>> = Vec::new();

        for row in range.rows() {
            let row_data: Vec<CellValue> = row.iter().map(data_to_cell_value).collect();
            data.push(row_data);
        }

        let mut sheet = Sheet::with_name(sheet_name);
        *sheet.data_mut() = data;

        if options.has_headers && sheet.row_count() > 0 {
            sheet.name_columns_by_row(0)?;
        }

        Ok(sheet)
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

        workbook
            .save(path.as_ref())
            .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        Ok(())
    }

    /// Write sheet data to a worksheet
    fn write_to_worksheet(&self, worksheet: &mut Worksheet) -> Result<()> {
        worksheet
            .set_name(self.name())
            .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        for (row_idx, row) in self.data().iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                let row_num = u32::try_from(row_idx)
                    .map_err(|_| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Row index overflow")))?;
                let col_num = u16::try_from(col_idx)
                    .map_err(|_| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Column index overflow")))?;

                match cell {
                    CellValue::Null => {} // Leave empty
                    CellValue::Bool(b) => {
                        worksheet
                            .write_boolean(row_num, col_num, *b)
                            .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                    }
                    CellValue::Int(i) => {
                        // Note: Excel stores all numbers as f64, so integers > 2^53
                        // (9,007,199,254,740,992) may lose precision
                        worksheet
                            .write_number(row_num, col_num, *i as f64)
                            .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                    }
                    CellValue::Float(f) => {
                        worksheet
                            .write_number(row_num, col_num, *f)
                            .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                    }
                    CellValue::String(s) => {
                        worksheet
                            .write_string(row_num, col_num, s)
                            .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
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
    pub fn from_xlsx_with_options<P: AsRef<Path>>(path: P, options: XlsxReadOptions) -> Result<Self> {
        let mut workbook: Xlsx<BufReader<File>> = open_workbook(path.as_ref())
            .map_err(|e: XlsxError| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let sheet_names: Vec<String> = workbook.sheet_names().iter().map(|s| s.to_string()).collect();
        let mut book = Book::new();

        for sheet_name in sheet_names {
            let range = workbook
                .worksheet_range(&sheet_name)
                .map_err(|e: XlsxError| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

            let mut data: Vec<Vec<CellValue>> = Vec::new();

            for row in range.rows() {
                let row_data: Vec<CellValue> = row.iter().map(data_to_cell_value).collect();
                data.push(row_data);
            }

            let mut sheet = Sheet::with_name(&sheet_name);
            *sheet.data_mut() = data;

            if options.has_headers && sheet.row_count() > 0 {
                // Ignore duplicate column name errors when loading
                let _ = sheet.name_columns_by_row(0);
            }

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
            worksheet
                .set_name(name)
                .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

            for (row_idx, row) in sheet.data().iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    let row_num = u32::try_from(row_idx)
                        .map_err(|_| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Row index overflow")))?;
                    let col_num = u16::try_from(col_idx)
                        .map_err(|_| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Column index overflow")))?;

                    match cell {
                        CellValue::Null => {}
                        CellValue::Bool(b) => {
                            worksheet
                                .write_boolean(row_num, col_num, *b)
                                .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                        }
                        CellValue::Int(i) => {
                            // Note: Excel stores all numbers as f64, so integers > 2^53
                            // (9,007,199,254,740,992) may lose precision
                            worksheet
                                .write_number(row_num, col_num, *i as f64)
                                .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                        }
                        CellValue::Float(f) => {
                            worksheet
                                .write_number(row_num, col_num, *f)
                                .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                        }
                        CellValue::String(s) => {
                            worksheet
                                .write_string(row_num, col_num, s)
                                .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                        }
                    }
                }
            }
        }

        workbook
            .save(path.as_ref())
            .map_err(|e| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        Ok(())
    }

    /// Get sheet names from an Excel file without loading data
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened.
    pub fn xlsx_sheet_names<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
        let workbook: Xlsx<BufReader<File>> = open_workbook(path.as_ref())
            .map_err(|e: XlsxError| SheetError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        Ok(workbook.sheet_names().iter().map(|s| s.to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
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
        assert!(matches!(loaded.get(0, 1).unwrap(), CellValue::Float(f) if (*f - 42.0).abs() < 0.01));

        // Verify float
        assert!(matches!(loaded.get(0, 2).unwrap(), CellValue::Float(f) if (*f - 3.14).abs() < 0.01));

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
        book.add_sheet("Data", Sheet::from_data(vec![vec![1, 2, 3]])).unwrap();
        book.add_sheet("Other", Sheet::from_data(vec![vec![4, 5, 6]])).unwrap();

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
        let with_headers = Sheet::from_xlsx_with_options(
            &path,
            XlsxReadOptions::default().with_headers(true),
        ).unwrap();

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
        book.add_sheet("Data", Sheet::from_data(vec![
            vec!["Product", "Price"],
            vec!["Widget", "10"],
            vec!["Gadget", "20"],
        ])).unwrap();

        book.save_as_xlsx(&path).unwrap();

        // Load book with headers
        let loaded = Book::from_xlsx_with_options(
            &path,
            XlsxReadOptions::default().with_headers(true),
        ).unwrap();

        let sheet = loaded.get_sheet("Data").unwrap();
        assert!(sheet.column_names().is_some());

        let prices = sheet.column_by_name("Price").unwrap();
        assert_eq!(prices.len(), 3);
    }
}
