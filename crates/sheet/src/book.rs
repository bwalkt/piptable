use crate::error::{Result, SheetError};
use crate::sheet::Sheet;
use indexmap::IndexMap;

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
        self.sheets.get(name).ok_or_else(|| SheetError::SheetNotFound {
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

    /// Rename a sheet
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

        // Get the sheet, update its name, and reinsert with new key
        if let Some(mut sheet) = self.sheets.shift_remove(old_name) {
            sheet.set_name(new_name);
            self.sheets.insert(new_name.to_string(), sheet);

            // Update active sheet reference
            if self.active_sheet.as_deref() == Some(old_name) {
                self.active_sheet = Some(new_name.to_string());
            }
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
    fn test_sheet_already_exists() {
        let mut book = Book::new();
        book.add_sheet("Sheet1", Sheet::new()).unwrap();

        let result = book.add_sheet("Sheet1", Sheet::new());
        assert!(result.is_err());
    }
}
