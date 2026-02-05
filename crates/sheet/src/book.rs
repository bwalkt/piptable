use crate::cell::CellValue;
use crate::error::{Result, SheetError};
use crate::sheet::Sheet;
use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;
use std::path::Path;

/// A book containing multiple sheets (preserves insertion order)
#[derive(Debug, Clone)]
pub struct Book {
    name: String,
    sheets: IndexMap<String, Sheet>,
    active_sheet: Option<String>,
}

impl Book {
    /// Create a new empty book
    #[must_use]
    pub fn new() -> Self {
        Self::with_name("Book1")
    }

    /// Create a new empty book with a name
    #[must_use]
    pub fn with_name(name: &str) -> Self {
        Book {
            name: name.to_string(),
            sheets: IndexMap::new(),
            active_sheet: None,
        }
    }

    /// Get the book name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the book name
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Get the number of sheets
    #[must_use]
    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    /// Check if the book is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty()
    }

    /// Get all sheet names in order
    #[must_use]
    pub fn sheet_names(&self) -> Vec<&str> {
        self.sheets.keys().map(String::as_str).collect()
    }

    /// Check if a sheet exists
    #[must_use]
    pub fn has_sheet(&self, name: &str) -> bool {
        self.sheets.contains_key(name)
    }

    // ===== Sheet Access =====

    /// Get a sheet by name
    pub fn get_sheet(&self, name: &str) -> Result<&Sheet> {
        self.sheets
            .get(name)
            .ok_or_else(|| SheetError::SheetNotFound {
                name: name.to_string(),
            })
    }

    /// Get a mutable sheet by name
    pub fn get_sheet_mut(&mut self, name: &str) -> Result<&mut Sheet> {
        self.sheets
            .get_mut(name)
            .ok_or_else(|| SheetError::SheetNotFound {
                name: name.to_string(),
            })
    }

    /// Get a sheet by index (0-based)
    pub fn get_sheet_by_index(&self, index: usize) -> Result<&Sheet> {
        self.sheets
            .get_index(index)
            .map(|(_, sheet)| sheet)
            .ok_or_else(|| SheetError::SheetNotFound {
                name: format!("index {index}"),
            })
    }

    /// Get a mutable sheet by index (0-based)
    pub fn get_sheet_by_index_mut(&mut self, index: usize) -> Result<&mut Sheet> {
        self.sheets
            .get_index_mut(index)
            .map(|(_, sheet)| sheet)
            .ok_or_else(|| SheetError::SheetNotFound {
                name: format!("index {index}"),
            })
    }

    /// Get the active sheet
    pub fn active_sheet(&self) -> Option<&Sheet> {
        self.active_sheet
            .as_ref()
            .and_then(|name| self.sheets.get(name))
    }

    /// Get the active sheet mutably
    pub fn active_sheet_mut(&mut self) -> Option<&mut Sheet> {
        let name = self.active_sheet.clone()?;
        self.sheets.get_mut(&name)
    }

    /// Set the active sheet by name
    pub fn set_active_sheet(&mut self, name: &str) -> Result<()> {
        if !self.sheets.contains_key(name) {
            return Err(SheetError::SheetNotFound {
                name: name.to_string(),
            });
        }
        self.active_sheet = Some(name.to_string());
        Ok(())
    }

    // ===== Sheet Management =====

    /// Add a sheet to the book
    pub fn add_sheet(&mut self, name: &str, sheet: Sheet) -> Result<()> {
        if self.sheets.contains_key(name) {
            return Err(SheetError::SheetAlreadyExists {
                name: name.to_string(),
            });
        }

        let mut sheet = sheet;
        sheet.set_name(name);
        self.sheets.insert(name.to_string(), sheet);

        // Set as active if first sheet
        if self.active_sheet.is_none() {
            self.active_sheet = Some(name.to_string());
        }

        Ok(())
    }

    /// Add a new empty sheet with the given name
    pub fn add_empty_sheet(&mut self, name: &str) -> Result<&mut Sheet> {
        self.add_sheet(name, Sheet::new())?;
        self.get_sheet_mut(name)
    }

    /// Remove a sheet by name
    pub fn remove_sheet(&mut self, name: &str) -> Result<Sheet> {
        let sheet = self
            .sheets
            .shift_remove(name)
            .ok_or_else(|| SheetError::SheetNotFound {
                name: name.to_string(),
            })?;

        // Update active sheet if removed
        if self.active_sheet.as_deref() == Some(name) {
            self.active_sheet = self.sheets.keys().next().cloned();
        }

        Ok(sheet)
    }

    /// Rename a sheet (preserves position in sheet order)
    pub fn rename_sheet(&mut self, old_name: &str, new_name: &str) -> Result<()> {
        if !self.sheets.contains_key(old_name) {
            return Err(SheetError::SheetNotFound {
                name: old_name.to_string(),
            });
        }

        if self.sheets.contains_key(new_name) {
            return Err(SheetError::SheetAlreadyExists {
                name: new_name.to_string(),
            });
        }

        // Get the index to preserve position
        if let Some(index) = self.sheets.get_index_of(old_name) {
            let (_, mut sheet) = self.sheets.shift_remove_index(index).unwrap();
            sheet.set_name(new_name);
            self.sheets.shift_insert(index, new_name.to_string(), sheet);

            // Update active sheet reference
            if self.active_sheet.as_deref() == Some(old_name) {
                self.active_sheet = Some(new_name.to_string());
            }
        }

        Ok(())
    }

    /// Create a book from a dictionary of sheet name -> 2D data.
    pub fn from_dict<T: Into<CellValue> + Clone>(
        sheets: IndexMap<String, Vec<Vec<T>>>,
    ) -> Result<Self> {
        let mut book = Book::new();
        for (name, data) in sheets {
            let sheet = Sheet::from_data(data);
            book.add_sheet(&name, sheet)?;
        }
        Ok(book)
    }

    /// Convert the book into a dictionary of sheet name -> 2D cell data.
    #[must_use]
    pub fn to_dict(&self) -> IndexMap<String, Vec<Vec<CellValue>>> {
        self.sheets
            .iter()
            .map(|(name, sheet)| (name.clone(), sheet.data().to_vec()))
            .collect()
    }

    /// Apply a function to each sheet.
    pub fn for_each_sheet<F>(&self, mut f: F)
    where
        F: FnMut(&Sheet),
    {
        for sheet in self.sheets.values() {
            f(sheet);
        }
    }

    /// Apply a function to each sheet mutably.
    pub fn for_each_sheet_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Sheet),
    {
        for sheet in self.sheets.values_mut() {
            f(sheet);
        }
    }

    /// Apply a fallible function to each sheet mutably.
    pub fn try_for_each_sheet_mut<F, E>(&mut self, mut f: F) -> std::result::Result<(), E>
    where
        F: FnMut(&mut Sheet) -> std::result::Result<(), E>,
    {
        for sheet in self.sheets.values_mut() {
            f(sheet)?;
        }
        Ok(())
    }

    // ===== Merge Operations =====

    /// Merge another book into this one
    /// Sheets with conflicting names will be renamed with a suffix
    pub fn merge(&mut self, other: Book) {
        for (name, sheet) in other.sheets {
            let final_name = if self.sheets.contains_key(&name) {
                let mut suffix = 1;
                loop {
                    let new_name = format!("{name}_{suffix}");
                    if !self.sheets.contains_key(&new_name) {
                        break new_name;
                    }
                    suffix += 1;
                }
            } else {
                name
            };

            let mut sheet = sheet;
            sheet.set_name(&final_name);
            self.sheets.insert(final_name, sheet);
        }
    }

    // ===== Multi-File Loading =====

    /// Load multiple files into a single book.
    ///
    /// Each file becomes a sheet named after its filename (without extension).
    /// Supports: csv, tsv, xlsx, xls, json, jsonl, toon, parquet
    ///
    /// # Example
    /// ```no_run
    /// use piptable_sheet::Book;
    ///
    /// let book = Book::from_files(&["sales_q1.csv", "sales_q2.csv"]).unwrap();
    /// // Creates book with sheets: "sales_q1", "sales_q2"
    /// ```
    pub fn from_files<P: AsRef<Path>>(paths: &[P]) -> Result<Self> {
        Self::from_files_with_options(paths, FileLoadOptions::default())
    }

    /// Load multiple files into a single book with options.
    ///
    /// # Example
    /// ```no_run
    /// use piptable_sheet::{Book, FileLoadOptions};
    ///
    /// // Load headerless CSV files
    /// let book = Book::from_files_with_options(
    ///     &["data1.csv", "data2.csv"],
    ///     FileLoadOptions::without_headers()
    /// ).unwrap();
    /// ```
    pub fn from_files_with_options<P: AsRef<Path>>(
        paths: &[P],
        options: FileLoadOptions,
    ) -> Result<Self> {
        let mut book = Book::new();

        for path in paths {
            let path_ref = path.as_ref();

            // Extract sheet name from filename stem
            let sheet_name = path_ref
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| {
                    SheetError::Parse(format!("Invalid filename: {}", path_ref.display()))
                })?
                .to_string();

            // Load sheet based on extension
            let sheet = load_sheet_by_extension(path_ref, &options)?;

            // Handle duplicate names with suffix
            let final_name = get_unique_name(&book, &sheet_name);

            book.add_sheet(&final_name, sheet)?;
        }

        Ok(book)
    }

    // ===== Consolidation =====

    /// Consolidate all sheets into a single sheet by stacking rows vertically.
    ///
    /// All sheets must have named columns. Columns are aligned by name.
    /// Missing columns in a sheet are filled with Null.
    ///
    /// # Example
    /// ```no_run
    /// use piptable_sheet::Book;
    ///
    /// let book = Book::from_files(&["q1.csv", "q2.csv", "q3.csv"]).unwrap();
    /// let combined = book.consolidate().unwrap();
    /// ```
    pub fn consolidate(&self) -> Result<Sheet> {
        self.consolidate_with_options(ConsolidateOptions::default())
    }

    /// Consolidate with options (e.g., add source column)
    pub fn consolidate_with_options(&self, options: ConsolidateOptions) -> Result<Sheet> {
        if self.is_empty() {
            return Ok(Sheet::new());
        }

        // Collect all unique column names across all sheets (preserving order)
        let mut all_columns: IndexSet<String> = IndexSet::new();

        // Validate all sheets have named columns
        for (name, sheet) in self.sheets() {
            let col_names = sheet.column_names().ok_or_else(|| {
                SheetError::ColumnsNotNamed(format!(
                    "Sheet '{}' does not have named columns. All sheets must have named columns for consolidate.",
                    name
                ))
            })?;

            for col in col_names {
                all_columns.insert(col.clone());
            }
        }

        // Build final column list
        let final_columns: Vec<String> = if options.add_source_column {
            // Check for conflict
            if all_columns.contains(&options.source_column_name) {
                return Err(SheetError::DuplicateColumnName {
                    name: options.source_column_name,
                });
            }
            std::iter::once(options.source_column_name.clone())
                .chain(all_columns.iter().cloned())
                .collect()
        } else {
            all_columns.iter().cloned().collect()
        };

        let mut result = Sheet::with_name("consolidated");

        // Add header row
        let header: Vec<CellValue> = final_columns
            .iter()
            .map(|n| CellValue::String(n.clone()))
            .collect();
        result.data_mut().push(header);

        // Add data from each sheet
        for (sheet_name, sheet) in self.sheets() {
            let sheet_col_names = sheet.column_names().unwrap(); // Already validated

            // Determine start row (skip header if present in data)
            // Only match String cells to avoid treating data rows like [1, 2] as headers
            let start_row = if sheet.data().first().is_some_and(|r| {
                r.iter()
                    .zip(sheet_col_names.iter())
                    .all(|(c, n)| matches!(c, CellValue::String(s) if s == n))
            }) {
                1
            } else {
                0
            };

            // Build column index for this sheet
            let col_idx: HashMap<&str, usize> = sheet_col_names
                .iter()
                .enumerate()
                .map(|(i, n)| (n.as_str(), i))
                .collect();

            for row in sheet.data().iter().skip(start_row) {
                let mut new_row = Vec::with_capacity(final_columns.len());

                for (i, col_name) in final_columns.iter().enumerate() {
                    if options.add_source_column && i == 0 {
                        // Source column
                        new_row.push(CellValue::String(sheet_name.to_string()));
                    } else {
                        // Data column - look up in source sheet
                        if let Some(&idx) = col_idx.get(col_name.as_str()) {
                            new_row.push(row.get(idx).cloned().unwrap_or(CellValue::Null));
                        } else {
                            new_row.push(CellValue::Null);
                        }
                    }
                }

                result.data_mut().push(new_row);
            }
        }

        // Name columns
        result.name_columns_by_row(0)?;

        Ok(result)
    }

    // ===== Iteration =====

    /// Iterate over sheets
    pub fn sheets(&self) -> impl Iterator<Item = (&str, &Sheet)> {
        self.sheets.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Iterate over sheets mutably
    pub fn sheets_mut(&mut self) -> impl Iterator<Item = (&str, &mut Sheet)> {
        self.sheets.iter_mut().map(|(k, v)| (k.as_str(), v))
    }
}

impl std::ops::Add for Book {
    type Output = Book;

    fn add(mut self, rhs: Book) -> Self::Output {
        self.merge(rhs);
        self
    }
}

impl std::ops::Add<&Book> for &Book {
    type Output = Book;

    fn add(self, rhs: &Book) -> Self::Output {
        let mut out = self.clone();
        out.merge(rhs.clone());
        out
    }
}

/// Options for consolidating sheets
#[derive(Debug, Clone)]
pub struct ConsolidateOptions {
    /// Add a column with the source sheet name
    pub add_source_column: bool,
    /// Name of the source column (default: "_source")
    pub source_column_name: String,
}

/// Options for loading files
#[derive(Debug, Clone)]
pub struct FileLoadOptions {
    /// Whether files have headers (default: true)
    /// Only affects CSV and TSV files.
    pub has_headers: bool,
}

impl Default for ConsolidateOptions {
    fn default() -> Self {
        Self {
            add_source_column: false,
            source_column_name: "_source".to_string(),
        }
    }
}

impl ConsolidateOptions {
    /// Enable adding a source column with a custom name
    #[must_use]
    pub fn with_source_column(mut self, name: &str) -> Self {
        self.add_source_column = true;
        self.source_column_name = name.to_string();
        self
    }
}

impl Default for FileLoadOptions {
    fn default() -> Self {
        Self { has_headers: true }
    }
}

impl FileLoadOptions {
    /// Create options for files without headers
    #[must_use]
    pub fn without_headers() -> Self {
        Self { has_headers: false }
    }

    /// Set whether files have headers
    #[must_use]
    pub fn with_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }
}

/// Load a sheet by auto-detecting format from file extension
fn load_sheet_by_extension(path: &Path, options: &FileLoadOptions) -> Result<Sheet> {
    use crate::csv::CsvOptions;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::xlsx::XlsxReadOptions;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let mut sheet = match ext.as_str() {
        "csv" => Sheet::from_csv(path)?,
        "tsv" => Sheet::from_csv_with_options(path, CsvOptions::tsv())?,
        #[cfg(not(target_arch = "wasm32"))]
        "xlsx" => Sheet::from_xlsx_with_options(
            path,
            XlsxReadOptions::default().with_headers(options.has_headers),
        )?,
        #[cfg(not(target_arch = "wasm32"))]
        "xls" => Sheet::from_xls_with_options(
            path,
            XlsxReadOptions::default().with_headers(options.has_headers),
        )?,
        "json" => Sheet::from_json(path)?,
        "jsonl" | "ndjson" => Sheet::from_jsonl(path)?,
        "toon" => Sheet::from_toon(path)?,
        #[cfg(not(target_arch = "wasm32"))]
        "parquet" => Sheet::from_parquet(path)?,
        #[cfg(target_arch = "wasm32")]
        "xlsx" | "xls" | "parquet" => {
            return Err(SheetError::Parse(
                "Format not supported in WASM builds".to_string(),
            ))
        }
        _ => {
            #[cfg(not(target_arch = "wasm32"))]
            return Err(SheetError::Parse(format!(
                "Unsupported file format: '{}'. Supported: csv, tsv, xlsx, xls, json, jsonl, toon, parquet",
                ext
            )));
            #[cfg(target_arch = "wasm32")]
            return Err(SheetError::Parse(format!(
                "Unsupported file format: '{}'. Supported: csv, tsv, json, jsonl, toon",
                ext
            )));
        }
    };

    // Ensure columns are named for CSV/TSV (first row as header) when has_headers is true
    if options.has_headers
        && matches!(ext.as_str(), "csv" | "tsv")
        && sheet.column_names().is_none()
    {
        sheet.name_columns_by_row(0)?;
    }

    Ok(sheet)
}

/// Generate a unique sheet name by appending _1, _2, etc.
fn get_unique_name(book: &Book, base_name: &str) -> String {
    if !book.has_sheet(base_name) {
        return base_name.to_string();
    }
    let mut suffix = 1;
    loop {
        let new_name = format!("{base_name}_{suffix}");
        if !book.has_sheet(&new_name) {
            return new_name;
        }
        suffix += 1;
    }
}

impl Default for Book {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for Book {
    type Item = (String, Sheet);
    type IntoIter = indexmap::map::IntoIter<String, Sheet>;

    fn into_iter(self) -> Self::IntoIter {
        self.sheets.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_book() {
        let book = Book::new();
        assert_eq!(book.name(), "Book1");
        assert!(book.is_empty());
        assert_eq!(book.sheet_count(), 0);
    }

    #[test]
    fn test_add_sheet() {
        let mut book = Book::new();
        let sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);

        book.add_sheet("Data", sheet).unwrap();

        assert_eq!(book.sheet_count(), 1);
        assert!(book.has_sheet("Data"));
        assert_eq!(book.sheet_names(), vec!["Data"]);
    }

    #[test]
    fn test_active_sheet() {
        let mut book = Book::new();

        book.add_sheet("Sheet1", Sheet::new()).unwrap();
        book.add_sheet("Sheet2", Sheet::new()).unwrap();

        // First sheet is active by default
        assert_eq!(book.active_sheet().unwrap().name(), "Sheet1");

        // Change active sheet
        book.set_active_sheet("Sheet2").unwrap();
        assert_eq!(book.active_sheet().unwrap().name(), "Sheet2");
    }

    #[test]
    fn test_remove_sheet() {
        let mut book = Book::new();
        book.add_sheet("Sheet1", Sheet::new()).unwrap();
        book.add_sheet("Sheet2", Sheet::new()).unwrap();

        book.remove_sheet("Sheet1").unwrap();

        assert_eq!(book.sheet_count(), 1);
        assert!(!book.has_sheet("Sheet1"));
        assert!(book.has_sheet("Sheet2"));
    }

    #[test]
    fn test_rename_sheet() {
        let mut book = Book::new();
        book.add_sheet("OldName", Sheet::new()).unwrap();

        book.rename_sheet("OldName", "NewName").unwrap();

        assert!(!book.has_sheet("OldName"));
        assert!(book.has_sheet("NewName"));
        assert_eq!(book.get_sheet("NewName").unwrap().name(), "NewName");
    }

    #[test]
    fn test_merge_books() {
        let mut book1 = Book::new();
        book1.add_sheet("Sheet1", Sheet::new()).unwrap();

        let mut book2 = Book::new();
        book2.add_sheet("Sheet1", Sheet::new()).unwrap(); // Conflict
        book2.add_sheet("Sheet2", Sheet::new()).unwrap();

        book1.merge(book2);

        assert_eq!(book1.sheet_count(), 3);
        assert!(book1.has_sheet("Sheet1"));
        assert!(book1.has_sheet("Sheet1_1")); // Renamed
        assert!(book1.has_sheet("Sheet2"));
    }

    #[test]
    fn test_from_dict_and_to_dict() {
        let mut input = IndexMap::new();
        input.insert("Sheet1".to_string(), vec![vec![1, 2], vec![3, 4]]);
        input.insert("Sheet2".to_string(), vec![vec![5, 6], vec![7, 8]]);

        let book = Book::from_dict(input.clone()).unwrap();
        assert_eq!(book.sheet_count(), 2);
        assert!(book.has_sheet("Sheet1"));
        assert!(book.has_sheet("Sheet2"));

        let output = book.to_dict();
        assert_eq!(output.len(), 2);
        assert_eq!(output.get("Sheet1").unwrap().len(), 2);
        assert_eq!(output.get("Sheet2").unwrap().len(), 2);
    }

    #[test]
    fn test_book_add_operator_merges() {
        let mut book1 = Book::new();
        book1.add_sheet("Sheet1", Sheet::new()).unwrap();

        let mut book2 = Book::new();
        book2.add_sheet("Sheet1", Sheet::new()).unwrap();
        book2.add_sheet("Sheet2", Sheet::new()).unwrap();

        let merged = book1 + book2;
        assert_eq!(merged.sheet_count(), 3);
        assert!(merged.has_sheet("Sheet1"));
        assert!(merged.has_sheet("Sheet1_1"));
        assert!(merged.has_sheet("Sheet2"));
    }

    #[test]
    fn test_for_each_sheet_mut() {
        let mut book = Book::new();
        book.add_sheet("A", Sheet::from_data(vec![vec![1]]))
            .unwrap();
        book.add_sheet("B", Sheet::from_data(vec![vec![2]]))
            .unwrap();

        book.for_each_sheet_mut(|sheet| {
            sheet.map(|cell| {
                if let Some(i) = cell.as_int() {
                    CellValue::Int(i + 1)
                } else {
                    cell.clone()
                }
            });
        });

        assert_eq!(
            book.get_sheet("A").unwrap().get(0, 0).unwrap(),
            &CellValue::Int(2)
        );
        assert_eq!(
            book.get_sheet("B").unwrap().get(0, 0).unwrap(),
            &CellValue::Int(3)
        );
    }

    #[test]
    fn test_try_for_each_sheet_mut() {
        let mut book = Book::new();
        book.add_sheet("A", Sheet::from_data(vec![vec![1]]))
            .unwrap();
        book.add_sheet("B", Sheet::from_data(vec![vec![2]]))
            .unwrap();

        let result: std::result::Result<(), &'static str> = book.try_for_each_sheet_mut(|sheet| {
            if sheet.row_count() == 0 {
                return Err("empty");
            }
            Ok(())
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_sheet_already_exists() {
        let mut book = Book::new();
        book.add_sheet("Sheet1", Sheet::new()).unwrap();

        let result = book.add_sheet("Sheet1", Sheet::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_consolidate() {
        let mut book = Book::new();

        let mut sheet1 = Sheet::from_data(vec![vec!["name", "value"], vec!["a", "1"]]);
        sheet1.name_columns_by_row(0).unwrap();

        let mut sheet2 = Sheet::from_data(vec![vec!["name", "value"], vec!["b", "2"]]);
        sheet2.name_columns_by_row(0).unwrap();

        book.add_sheet("Sheet1", sheet1).unwrap();
        book.add_sheet("Sheet2", sheet2).unwrap();

        let consolidated = book.consolidate().unwrap();

        assert_eq!(consolidated.row_count(), 3); // header + 2 data rows
        assert_eq!(consolidated.col_count(), 2);
        assert_eq!(
            consolidated.column_names(),
            Some(&vec!["name".to_string(), "value".to_string()])
        );
    }

    #[test]
    fn test_consolidate_with_source_column() {
        let mut book = Book::new();

        let mut sheet1 = Sheet::from_data(vec![vec!["name"], vec!["a"]]);
        sheet1.name_columns_by_row(0).unwrap();

        let mut sheet2 = Sheet::from_data(vec![vec!["name"], vec!["b"]]);
        sheet2.name_columns_by_row(0).unwrap();

        book.add_sheet("Q1", sheet1).unwrap();
        book.add_sheet("Q2", sheet2).unwrap();

        let consolidated = book
            .consolidate_with_options(ConsolidateOptions::default().with_source_column("_source"))
            .unwrap();

        assert_eq!(consolidated.row_count(), 3);
        assert_eq!(consolidated.col_count(), 2);

        // Check source column values
        assert_eq!(
            consolidated.get(1, 0).unwrap(),
            &CellValue::String("Q1".to_string())
        );
        assert_eq!(
            consolidated.get(2, 0).unwrap(),
            &CellValue::String("Q2".to_string())
        );
    }

    #[test]
    fn test_consolidate_different_columns() {
        let mut book = Book::new();

        let mut sheet1 = Sheet::from_data(vec![vec!["a", "b"], vec!["1", "2"]]);
        sheet1.name_columns_by_row(0).unwrap();

        let mut sheet2 = Sheet::from_data(vec![vec!["b", "c"], vec!["3", "4"]]);
        sheet2.name_columns_by_row(0).unwrap();

        book.add_sheet("Sheet1", sheet1).unwrap();
        book.add_sheet("Sheet2", sheet2).unwrap();

        let consolidated = book.consolidate().unwrap();

        // Should have all columns: a, b, c
        assert_eq!(consolidated.col_count(), 3);
        assert_eq!(
            consolidated.column_names(),
            Some(&vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );

        // Row from sheet1: a=1, b=2, c=null
        assert_eq!(
            consolidated.get(1, 0).unwrap(),
            &CellValue::String("1".to_string())
        );
        assert_eq!(
            consolidated.get(1, 1).unwrap(),
            &CellValue::String("2".to_string())
        );
        assert!(consolidated.get(1, 2).unwrap().is_null());

        // Row from sheet2: a=null, b=3, c=4
        assert!(consolidated.get(2, 0).unwrap().is_null());
        assert_eq!(
            consolidated.get(2, 1).unwrap(),
            &CellValue::String("3".to_string())
        );
        assert_eq!(
            consolidated.get(2, 2).unwrap(),
            &CellValue::String("4".to_string())
        );
    }

    #[test]
    fn test_consolidate_empty_book() {
        let book = Book::new();
        let result = book.consolidate().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_consolidate_columns_not_named_error() {
        let mut book = Book::new();
        let sheet = Sheet::from_data(vec![vec!["a", "b"]]); // No named columns
        book.add_sheet("Sheet1", sheet).unwrap();

        let result = book.consolidate();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_unique_name() {
        let mut book = Book::new();
        book.add_sheet("data", Sheet::new()).unwrap();
        book.add_sheet("data_1", Sheet::new()).unwrap();

        assert_eq!(get_unique_name(&book, "data"), "data_2");
        assert_eq!(get_unique_name(&book, "other"), "other");
    }

    #[test]
    fn test_file_load_options_default() {
        let opts = FileLoadOptions::default();
        assert!(opts.has_headers);
    }

    #[test]
    fn test_file_load_options_without_headers() {
        let opts = FileLoadOptions::without_headers();
        assert!(!opts.has_headers);
    }

    #[test]
    fn test_file_load_options_builder() {
        let opts = FileLoadOptions::default().with_headers(false);
        assert!(!opts.has_headers);

        let opts = FileLoadOptions::without_headers().with_headers(true);
        assert!(opts.has_headers);
    }
}
