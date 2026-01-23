use crate::book::Book;
use crate::cell::CellValue;
use crate::error::Result;
use crate::sheet::Sheet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// CSV reader/writer options
#[derive(Debug, Clone)]
pub struct CsvOptions {
    /// Field delimiter (default: ',')
    pub delimiter: u8,
    /// Whether the first row contains headers
    pub has_headers: bool,
    /// Quote character (default: '"')
    pub quote: u8,
    /// Whether to use type inference when reading
    pub infer_types: bool,
}

impl Default for CsvOptions {
    fn default() -> Self {
        CsvOptions {
            delimiter: b',',
            has_headers: false,
            quote: b'"',
            infer_types: true,
        }
    }
}

impl CsvOptions {
    /// Create options for TSV (tab-separated values)
    #[must_use]
    pub fn tsv() -> Self {
        CsvOptions {
            delimiter: b'\t',
            ..Default::default()
        }
    }

    /// Set the delimiter
    #[must_use]
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Set whether the first row contains headers
    #[must_use]
    pub fn with_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }

    /// Set whether to infer types
    #[must_use]
    pub fn with_type_inference(mut self, infer_types: bool) -> Self {
        self.infer_types = infer_types;
        self
    }
}

impl Sheet {
    /// Load a sheet from a CSV file
    pub fn from_csv<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_csv_with_options(path, CsvOptions::default())
    }

    /// Load a sheet from a CSV file with custom options
    pub fn from_csv_with_options<P: AsRef<Path>>(path: P, options: CsvOptions) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        Self::from_csv_reader(reader, options)
    }

    /// Load a sheet from a CSV string
    pub fn from_csv_str(content: &str) -> Result<Self> {
        Self::from_csv_str_with_options(content, CsvOptions::default())
    }

    /// Load a sheet from a CSV string with custom options
    pub fn from_csv_str_with_options(content: &str, options: CsvOptions) -> Result<Self> {
        Self::from_csv_reader(content.as_bytes(), options)
    }

    /// Load a sheet from a reader
    pub fn from_csv_reader<R: Read>(reader: R, options: CsvOptions) -> Result<Self> {
        let mut csv_reader = csv::ReaderBuilder::new()
            .delimiter(options.delimiter)
            .quote(options.quote)
            .has_headers(false) // We handle headers ourselves
            .from_reader(reader);

        let mut data: Vec<Vec<CellValue>> = Vec::new();

        for result in csv_reader.records() {
            let record = result?;
            let row: Vec<CellValue> = record
                .iter()
                .map(|field| {
                    if options.infer_types {
                        CellValue::parse(field)
                    } else {
                        CellValue::String(field.to_string())
                    }
                })
                .collect();
            data.push(row);
        }

        let mut sheet = Sheet::with_name("Sheet1");
        *sheet.data_mut() = data;

        if options.has_headers && sheet.row_count() > 0 {
            sheet.name_columns_by_row(0)?;
        }

        Ok(sheet)
    }

    /// Save the sheet to a CSV file
    pub fn save_as_csv<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.save_as_csv_with_options(path, CsvOptions::default())
    }

    /// Save the sheet to a CSV file with custom options
    pub fn save_as_csv_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        options: CsvOptions,
    ) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.write_csv(writer, options)
    }

    /// Write the sheet to a writer as CSV
    pub fn write_csv<W: Write>(&self, writer: W, options: CsvOptions) -> Result<()> {
        let mut csv_writer = csv::WriterBuilder::new()
            .delimiter(options.delimiter)
            .quote(options.quote)
            .from_writer(writer);

        for row in self.data() {
            let record: Vec<String> = row.iter().map(CellValue::as_str).collect();
            csv_writer.write_record(&record)?;
        }

        csv_writer.flush()?;
        Ok(())
    }

    /// Convert the sheet to a CSV string
    #[must_use]
    pub fn to_csv_string(&self) -> String {
        self.to_csv_string_with_options(CsvOptions::default())
    }

    /// Convert the sheet to a CSV string with custom options
    #[must_use]
    pub fn to_csv_string_with_options(&self, options: CsvOptions) -> String {
        let mut buffer = Vec::new();
        // Ignore errors for string conversion
        let _ = self.write_csv(&mut buffer, options);
        String::from_utf8_lossy(&buffer).to_string()
    }

    /// Convert the sheet to a TSV string
    #[must_use]
    pub fn to_tsv_string(&self) -> String {
        self.to_csv_string_with_options(CsvOptions::tsv())
    }
}

impl Book {
    /// Load a book from a directory of CSV files
    /// Each CSV file becomes a sheet with the filename (without extension) as the sheet name
    pub fn from_csv_dir<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::from_csv_dir_with_options(path, CsvOptions::default())
    }

    /// Load a book from a directory of CSV files with custom options
    pub fn from_csv_dir_with_options<P: AsRef<Path>>(path: P, options: CsvOptions) -> Result<Self> {
        let mut book = Book::new();
        let dir = std::fs::read_dir(path)?;

        for entry in dir {
            let entry = entry?;
            let file_path = entry.path();

            if let Some(ext) = file_path.extension() {
                if ext == "csv" || ext == "tsv" {
                    let sheet_name = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Sheet")
                        .to_string();

                    let file_options = if ext == "tsv" {
                        CsvOptions::tsv()
                    } else {
                        options.clone()
                    };

                    let sheet = Sheet::from_csv_with_options(&file_path, file_options)?;
                    book.add_sheet(&sheet_name, sheet)?;
                }
            }
        }

        Ok(book)
    }

    /// Save all sheets to a directory as CSV files
    pub fn save_as_csv_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.save_as_csv_dir_with_options(path, CsvOptions::default())
    }

    /// Save all sheets to a directory as CSV files with custom options
    pub fn save_as_csv_dir_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        options: CsvOptions,
    ) -> Result<()> {
        std::fs::create_dir_all(path.as_ref())?;

        for (name, sheet) in self.sheets() {
            let file_path = path.as_ref().join(format!("{name}.csv"));
            sheet.save_as_csv_with_options(&file_path, options.clone())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_from_csv_str() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA";
        let sheet = Sheet::from_csv_str(csv).unwrap();

        assert_eq!(sheet.row_count(), 3);
        assert_eq!(sheet.col_count(), 3);
        assert_eq!(
            sheet.get(0, 0).unwrap(),
            &CellValue::String("name".to_string())
        );
        assert_eq!(sheet.get(1, 1).unwrap(), &CellValue::Int(30));
    }

    #[test]
    fn test_from_csv_with_headers() {
        let csv = "name,age,city\nAlice,30,NYC\nBob,25,LA";
        let options = CsvOptions::default().with_headers(true);
        let sheet = Sheet::from_csv_str_with_options(csv, options).unwrap();

        assert!(sheet.column_names().is_some());
        let age_col = sheet.column_by_name("age").unwrap();
        assert_eq!(age_col[0], CellValue::String("age".to_string()));
        assert_eq!(age_col[1], CellValue::Int(30));
    }

    #[test]
    fn test_type_inference() {
        let csv = "string,int,float,bool,empty\nhello,42,3.14,true,";
        let sheet = Sheet::from_csv_str(csv).unwrap();

        assert_eq!(
            sheet.get(1, 0).unwrap(),
            &CellValue::String("hello".to_string())
        );
        assert_eq!(sheet.get(1, 1).unwrap(), &CellValue::Int(42));
        assert_eq!(sheet.get(1, 2).unwrap(), &CellValue::Float(3.14));
        assert_eq!(sheet.get(1, 3).unwrap(), &CellValue::Bool(true));
        assert_eq!(sheet.get(1, 4).unwrap(), &CellValue::Null);
    }

    #[test]
    fn test_to_csv_string() {
        let sheet = Sheet::from_data(vec![vec![1, 2, 3], vec![4, 5, 6]]);

        let csv = sheet.to_csv_string();
        assert!(csv.contains("1,2,3"));
        assert!(csv.contains("4,5,6"));
    }

    #[test]
    fn test_csv_roundtrip() {
        let original = Sheet::from_data(vec![
            vec!["name", "value"],
            vec!["test", "42"],
        ]);

        let csv = original.to_csv_string();
        let restored = Sheet::from_csv_str(&csv).unwrap();

        assert_eq!(original.row_count(), restored.row_count());
        assert_eq!(original.col_count(), restored.col_count());
    }

    #[test]
    fn test_save_and_load_csv_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.csv");

        let sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);
        sheet.save_as_csv(&file_path).unwrap();

        let loaded = Sheet::from_csv(&file_path).unwrap();
        assert_eq!(loaded.row_count(), 2);
        assert_eq!(loaded.col_count(), 2);
    }

    #[test]
    fn test_tsv() {
        let tsv = "name\tage\nAlice\t30";
        let sheet = Sheet::from_csv_str_with_options(tsv, CsvOptions::tsv()).unwrap();

        assert_eq!(sheet.row_count(), 2);
        assert_eq!(
            sheet.get(0, 0).unwrap(),
            &CellValue::String("name".to_string())
        );

        let output = sheet.to_tsv_string();
        assert!(output.contains("name\tage"));
    }

    #[test]
    fn test_book_csv_dir() {
        let dir = tempdir().unwrap();

        // Create some CSV files
        let sheet1 = Sheet::from_data(vec![vec![1, 2]]);
        let sheet2 = Sheet::from_data(vec![vec![3, 4]]);

        sheet1
            .save_as_csv(dir.path().join("sheet1.csv"))
            .unwrap();
        sheet2
            .save_as_csv(dir.path().join("sheet2.csv"))
            .unwrap();

        // Load as book
        let book = Book::from_csv_dir(dir.path()).unwrap();

        assert_eq!(book.sheet_count(), 2);
        assert!(book.has_sheet("sheet1"));
        assert!(book.has_sheet("sheet2"));
    }
}
