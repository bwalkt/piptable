use crate::cell::CellValue;
use crate::error::{Result, SheetError};
use indexmap::IndexMap;
use std::collections::HashMap;

/// A sheet representing a 2D grid of cells (row-major storage)
#[derive(Debug, Clone)]
pub struct Sheet {
    name: String,
    data: Vec<Vec<CellValue>>,
    column_names: Option<Vec<String>>,
    column_index: Option<HashMap<String, usize>>,
    row_names: Option<HashMap<String, usize>>,
}

impl Sheet {
    /// Create a new empty sheet
    #[must_use]
    pub fn new() -> Self {
        Self::with_name("Sheet1")
    }

    /// Create a new empty sheet with a name
    #[must_use]
    pub fn with_name(name: &str) -> Self {
        Sheet {
            name: name.to_string(),
            data: Vec::new(),
            column_names: None,
            column_index: None,
            row_names: None,
        }
    }

    /// Create a sheet from a 2D vector of values
    #[must_use]
    pub fn from_data<T: Into<CellValue> + Clone>(data: Vec<Vec<T>>) -> Self {
        let converted: Vec<Vec<CellValue>> = data
            .into_iter()
            .map(|row| row.into_iter().map(Into::into).collect())
            .collect();

        Sheet {
            name: "Sheet1".to_string(),
            data: converted,
            column_names: None,
            column_index: None,
            row_names: None,
        }
    }

    /// Get the sheet name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the sheet name
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Get the number of rows
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.data.len()
    }

    /// Get the number of columns
    #[must_use]
    pub fn col_count(&self) -> usize {
        self.data.first().map_or(0, Vec::len)
    }

    /// Check if the sheet is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // ===== Cell Access =====

    /// Get a cell value by row and column index (0-based)
    pub fn get(&self, row: usize, col: usize) -> Result<&CellValue> {
        self.data
            .get(row)
            .and_then(|r| r.get(col))
            .ok_or(SheetError::IndexOutOfBounds {
                row,
                col,
                rows: self.row_count(),
                cols: self.col_count(),
            })
    }

    /// Get a mutable cell value by row and column index (0-based)
    pub fn get_mut(&mut self, row: usize, col: usize) -> Result<&mut CellValue> {
        let rows = self.row_count();
        let cols = self.col_count();
        self.data
            .get_mut(row)
            .and_then(|r| r.get_mut(col))
            .ok_or(SheetError::IndexOutOfBounds {
                row,
                col,
                rows,
                cols,
            })
    }

    /// Set a cell value by row and column index (0-based)
    pub fn set<T: Into<CellValue>>(&mut self, row: usize, col: usize, value: T) -> Result<()> {
        let cell = self.get_mut(row, col)?;
        *cell = value.into();
        Ok(())
    }

    /// Get a cell value by row index and column name
    pub fn get_by_name(&self, row: usize, col_name: &str) -> Result<&CellValue> {
        let col = self.column_index_by_name(col_name)?;
        self.get(row, col)
    }

    /// Set a cell value by row index and column name
    pub fn set_by_name<T: Into<CellValue>>(
        &mut self,
        row: usize,
        col_name: &str,
        value: T,
    ) -> Result<()> {
        let col = self.column_index_by_name(col_name)?;
        self.set(row, col, value)
    }

    // ===== Row Operations =====

    /// Get an entire row by index (0-based)
    pub fn row(&self, index: usize) -> Result<&Vec<CellValue>> {
        self.data.get(index).ok_or(SheetError::RowIndexOutOfBounds {
            index,
            count: self.row_count(),
        })
    }

    /// Get an entire row by name (after calling `name_rows_by_column`)
    pub fn row_by_name(&self, name: &str) -> Result<&Vec<CellValue>> {
        let index = self.row_index_by_name(name)?;
        self.row(index)
    }

    /// Append a row to the end of the sheet
    pub fn row_append<T: Into<CellValue>>(&mut self, data: Vec<T>) -> Result<()> {
        let row: Vec<CellValue> = data.into_iter().map(Into::into).collect();

        // Ensure consistent column count
        if !self.data.is_empty() && row.len() != self.col_count() {
            return Err(SheetError::LengthMismatch {
                expected: self.col_count(),
                actual: row.len(),
            });
        }

        self.data.push(row);
        Ok(())
    }

    /// Insert a row at a specific index
    pub fn row_insert<T: Into<CellValue>>(&mut self, index: usize, data: Vec<T>) -> Result<()> {
        if index > self.row_count() {
            return Err(SheetError::RowIndexOutOfBounds {
                index,
                count: self.row_count(),
            });
        }

        let row: Vec<CellValue> = data.into_iter().map(Into::into).collect();

        // Ensure consistent column count
        if !self.data.is_empty() && row.len() != self.col_count() {
            return Err(SheetError::LengthMismatch {
                expected: self.col_count(),
                actual: row.len(),
            });
        }

        self.data.insert(index, row);
        self.invalidate_row_names();
        Ok(())
    }

    /// Update a row at a specific index
    pub fn row_update<T: Into<CellValue>>(&mut self, index: usize, data: Vec<T>) -> Result<()> {
        if index >= self.row_count() {
            return Err(SheetError::RowIndexOutOfBounds {
                index,
                count: self.row_count(),
            });
        }

        let row: Vec<CellValue> = data.into_iter().map(Into::into).collect();

        if row.len() != self.col_count() {
            return Err(SheetError::LengthMismatch {
                expected: self.col_count(),
                actual: row.len(),
            });
        }

        self.data[index] = row;
        Ok(())
    }

    /// Delete a row at a specific index
    pub fn row_delete(&mut self, index: usize) -> Result<Vec<CellValue>> {
        if index >= self.row_count() {
            return Err(SheetError::RowIndexOutOfBounds {
                index,
                count: self.row_count(),
            });
        }

        self.invalidate_row_names();
        Ok(self.data.remove(index))
    }

    /// Delete multiple rows by indices (in descending order to maintain indices)
    pub fn row_delete_multi(&mut self, mut indices: Vec<usize>) -> Result<()> {
        indices.sort_unstable();
        indices.reverse();

        for index in indices {
            self.row_delete(index)?;
        }
        Ok(())
    }

    /// Delete rows matching a predicate
    pub fn row_delete_where<F>(&mut self, predicate: F) -> usize
    where
        F: Fn(&[CellValue]) -> bool,
    {
        let original_len = self.data.len();
        self.data.retain(|row| !predicate(row));
        self.invalidate_row_names();
        original_len - self.data.len()
    }

    // ===== Column Operations =====

    /// Get an entire column by index (0-based)
    pub fn column(&self, index: usize) -> Result<Vec<CellValue>> {
        if index >= self.col_count() {
            return Err(SheetError::ColumnIndexOutOfBounds {
                index,
                count: self.col_count(),
            });
        }

        Ok(self.data.iter().map(|row| row[index].clone()).collect())
    }

    /// Get an entire column by name
    pub fn column_by_name(&self, name: &str) -> Result<Vec<CellValue>> {
        let index = self.column_index_by_name(name)?;
        self.column(index)
    }

    /// Append a column to the end of each row
    pub fn column_append<T: Into<CellValue> + Clone>(&mut self, data: Vec<T>) -> Result<()> {
        if !self.data.is_empty() && data.len() != self.row_count() {
            return Err(SheetError::LengthMismatch {
                expected: self.row_count(),
                actual: data.len(),
            });
        }

        // If sheet is empty, create rows
        if self.data.is_empty() {
            for value in data {
                self.data.push(vec![value.into()]);
            }
        } else {
            for (row, value) in self.data.iter_mut().zip(data.into_iter()) {
                row.push(value.into());
            }
        }

        self.invalidate_column_names();
        Ok(())
    }

    /// Insert a column at a specific index
    pub fn column_insert<T: Into<CellValue> + Clone>(
        &mut self,
        index: usize,
        data: Vec<T>,
    ) -> Result<()> {
        if index > self.col_count() {
            return Err(SheetError::ColumnIndexOutOfBounds {
                index,
                count: self.col_count(),
            });
        }

        if !self.data.is_empty() && data.len() != self.row_count() {
            return Err(SheetError::LengthMismatch {
                expected: self.row_count(),
                actual: data.len(),
            });
        }

        for (row, value) in self.data.iter_mut().zip(data.into_iter()) {
            row.insert(index, value.into());
        }

        self.invalidate_column_names();
        Ok(())
    }

    /// Update a column at a specific index
    pub fn column_update<T: Into<CellValue> + Clone>(
        &mut self,
        index: usize,
        data: Vec<T>,
    ) -> Result<()> {
        if index >= self.col_count() {
            return Err(SheetError::ColumnIndexOutOfBounds {
                index,
                count: self.col_count(),
            });
        }

        if data.len() != self.row_count() {
            return Err(SheetError::LengthMismatch {
                expected: self.row_count(),
                actual: data.len(),
            });
        }

        for (row, value) in self.data.iter_mut().zip(data.into_iter()) {
            row[index] = value.into();
        }

        Ok(())
    }

    /// Update a column by name
    pub fn column_update_by_name<T: Into<CellValue> + Clone>(
        &mut self,
        name: &str,
        data: Vec<T>,
    ) -> Result<()> {
        let index = self.column_index_by_name(name)?;
        self.column_update(index, data)
    }

    /// Delete a column at a specific index
    pub fn column_delete(&mut self, index: usize) -> Result<Vec<CellValue>> {
        if index >= self.col_count() {
            return Err(SheetError::ColumnIndexOutOfBounds {
                index,
                count: self.col_count(),
            });
        }

        let removed: Vec<CellValue> = self.data.iter_mut().map(|row| row.remove(index)).collect();

        self.invalidate_column_names();
        Ok(removed)
    }

    /// Delete a column by name
    pub fn column_delete_by_name(&mut self, name: &str) -> Result<Vec<CellValue>> {
        let index = self.column_index_by_name(name)?;
        self.column_delete(index)
    }

    /// Delete multiple columns by names
    pub fn column_delete_multi_by_name(&mut self, names: &[&str]) -> Result<()> {
        // Get indices in reverse order to maintain positions during deletion
        let mut indices: Vec<usize> = names
            .iter()
            .map(|name| self.column_index_by_name(name))
            .collect::<Result<Vec<_>>>()?;

        indices.sort_unstable();
        indices.reverse();

        for index in indices {
            self.column_delete(index)?;
        }

        Ok(())
    }

    // ===== Named Access =====

    /// Use the specified row as column headers
    ///
    /// # Errors
    ///
    /// Returns `SheetError::DuplicateColumnName` if the header row contains duplicate names.
    pub fn name_columns_by_row(&mut self, row_index: usize) -> Result<()> {
        let header_row = self.row(row_index)?;
        let names: Vec<String> = header_row.iter().map(|c| c.as_str()).collect();

        let mut index_map = HashMap::new();
        for (i, name) in names.iter().enumerate() {
            if index_map.contains_key(name) {
                return Err(SheetError::DuplicateColumnName { name: name.clone() });
            }
            index_map.insert(name.clone(), i);
        }

        self.column_names = Some(names);
        self.column_index = Some(index_map);
        Ok(())
    }

    /// Use the specified column as row names
    pub fn name_rows_by_column(&mut self, col_index: usize) -> Result<()> {
        let name_col = self.column(col_index)?;

        let mut name_map = HashMap::new();
        for (i, cell) in name_col.iter().enumerate() {
            name_map.insert(cell.as_str(), i);
        }

        self.row_names = Some(name_map);
        Ok(())
    }

    /// Get column names (if set)
    #[must_use]
    pub fn column_names(&self) -> Option<&Vec<String>> {
        self.column_names.as_ref()
    }

    /// Get the column index by name
    fn column_index_by_name(&self, name: &str) -> Result<usize> {
        self.column_index
            .as_ref()
            .ok_or_else(|| {
                SheetError::ColumnsNotNamed("Call name_columns_by_row() first".to_string())
            })?
            .get(name)
            .copied()
            .ok_or_else(|| SheetError::ColumnNotFound {
                name: name.to_string(),
            })
    }

    /// Get the row index by name
    fn row_index_by_name(&self, name: &str) -> Result<usize> {
        self.row_names
            .as_ref()
            .ok_or(SheetError::RowsNotNamed)?
            .get(name)
            .copied()
            .ok_or_else(|| SheetError::RowNotFound {
                name: name.to_string(),
            })
    }

    fn invalidate_column_names(&mut self) {
        self.column_names = None;
        self.column_index = None;
    }

    fn invalidate_row_names(&mut self) {
        self.row_names = None;
    }

    // ===== Transformation =====

    /// Apply a function to all cells
    pub fn map<F>(&mut self, f: F)
    where
        F: Fn(&CellValue) -> CellValue,
    {
        for row in &mut self.data {
            for cell in row {
                *cell = f(cell);
            }
        }
    }

    /// Apply a function to a specific column
    pub fn column_map<F>(&mut self, col_index: usize, f: F) -> Result<()>
    where
        F: Fn(&CellValue) -> CellValue,
    {
        if col_index >= self.col_count() {
            return Err(SheetError::ColumnIndexOutOfBounds {
                index: col_index,
                count: self.col_count(),
            });
        }

        for row in &mut self.data {
            row[col_index] = f(&row[col_index]);
        }

        Ok(())
    }

    /// Apply a function to a specific column by name
    pub fn column_map_by_name<F>(&mut self, name: &str, f: F) -> Result<()>
    where
        F: Fn(&CellValue) -> CellValue,
    {
        let index = self.column_index_by_name(name)?;
        self.column_map(index, f)
    }

    /// Filter rows, keeping only those that match the predicate
    pub fn filter_rows<F>(&mut self, predicate: F)
    where
        F: Fn(&[CellValue]) -> bool,
    {
        self.data.retain(|row| predicate(row));
        self.invalidate_row_names();
    }

    /// Remove columns at the specified indices
    pub fn remove_columns_at(&mut self, indices: &[usize]) -> Result<()> {
        for &index in indices {
            if index >= self.col_count() {
                return Err(SheetError::ColumnIndexOutOfBounds {
                    index,
                    count: self.col_count(),
                });
            }
        }

        // Sort indices in descending order for safe removal
        let mut sorted_indices: Vec<usize> = indices.to_vec();
        sorted_indices.sort_unstable();
        sorted_indices.reverse();

        for row in &mut self.data {
            for &index in &sorted_indices {
                row.remove(index);
            }
        }

        self.invalidate_column_names();
        Ok(())
    }

    // ===== Conversion =====

    /// Convert to a 2D array (list of lists)
    #[must_use]
    pub fn to_array(&self) -> Vec<Vec<CellValue>> {
        self.data.clone()
    }

    /// Convert to a dictionary (column name -> values)
    /// Returns None if columns are not named
    #[must_use]
    pub fn to_dict(&self) -> Option<IndexMap<String, Vec<CellValue>>> {
        let names = self.column_names.as_ref()?;
        let mut dict = IndexMap::new();

        for (i, name) in names.iter().enumerate() {
            let col: Vec<CellValue> = self.data.iter().map(|row| row[i].clone()).collect();
            dict.insert(name.clone(), col);
        }

        Some(dict)
    }

    /// Convert to a list of records (list of dictionaries)
    ///
    /// Each row becomes an IndexMap with column names as keys.
    /// Column order is preserved in each record.
    /// Returns None if columns are not named.
    ///
    /// # Example
    /// ```
    /// use piptable_sheet::Sheet;
    ///
    /// let mut sheet = Sheet::from_data(vec![
    ///     vec!["name", "age"],
    ///     vec!["Alice", "30"],
    ///     vec!["Bob", "25"],
    /// ]);
    /// sheet.name_columns_by_row(0).unwrap();
    ///
    /// let records = sheet.to_records().unwrap();
    /// // [{"name": "name", "age": "age"}, {"name": "Alice", "age": "30"}, ...]
    /// ```
    #[must_use]
    pub fn to_records(&self) -> Option<Vec<IndexMap<String, CellValue>>> {
        let names = self.column_names.as_ref()?;
        let mut records = Vec::with_capacity(self.data.len());

        for row in &self.data {
            let mut record = IndexMap::new();
            for (i, name) in names.iter().enumerate() {
                if i < row.len() {
                    record.insert(name.clone(), row[i].clone());
                } else {
                    record.insert(name.clone(), CellValue::Null);
                }
            }
            records.push(record);
        }

        Some(records)
    }

    /// Create a sheet from a list of records (list of dictionaries)
    ///
    /// All records should have the same keys. Column order is determined
    /// by the first record.
    ///
    /// # Example
    /// ```
    /// use piptable_sheet::{Sheet, CellValue};
    /// use indexmap::IndexMap;
    ///
    /// let mut record1 = IndexMap::new();
    /// record1.insert("name".to_string(), CellValue::String("Alice".to_string()));
    /// record1.insert("age".to_string(), CellValue::Int(30));
    ///
    /// let mut record2 = IndexMap::new();
    /// record2.insert("name".to_string(), CellValue::String("Bob".to_string()));
    /// record2.insert("age".to_string(), CellValue::Int(25));
    ///
    /// let sheet = Sheet::from_records(vec![record1, record2]).unwrap();
    /// assert_eq!(sheet.row_count(), 3); // header + 2 data rows
    /// ```
    pub fn from_records(records: Vec<IndexMap<String, CellValue>>) -> Result<Self> {
        if records.is_empty() {
            return Ok(Sheet::new());
        }

        // Get column names from first record
        let col_names: Vec<String> = records[0].keys().cloned().collect();

        // Build data with header row
        let mut data = Vec::with_capacity(records.len() + 1);

        // Add header row
        let header: Vec<CellValue> = col_names
            .iter()
            .map(|n| CellValue::String(n.clone()))
            .collect();
        data.push(header);

        // Add data rows
        for record in &records {
            let row: Vec<CellValue> = col_names
                .iter()
                .map(|name| record.get(name).cloned().unwrap_or(CellValue::Null))
                .collect();
            data.push(row);
        }

        let mut sheet = Sheet {
            name: "Sheet1".to_string(),
            data,
            column_names: None,
            column_index: None,
            row_names: None,
        };

        // Name columns by header row
        sheet.name_columns_by_row(0)?;

        Ok(sheet)
    }

    /// Get rows iterator (excluding header row if named)
    pub fn rows(&self) -> impl Iterator<Item = &Vec<CellValue>> {
        self.data.iter()
    }

    /// Get mutable rows iterator
    pub fn rows_mut(&mut self) -> impl Iterator<Item = &mut Vec<CellValue>> {
        self.data.iter_mut()
    }

    /// Get internal data reference
    #[must_use]
    pub fn data(&self) -> &Vec<Vec<CellValue>> {
        &self.data
    }

    /// Get mutable internal data reference
    pub fn data_mut(&mut self) -> &mut Vec<Vec<CellValue>> {
        &mut self.data
    }

    // ===== Join Operations =====

    /// Inner join with another sheet on a key column.
    ///
    /// Returns only rows where the key exists in both sheets.
    /// Both sheets must have named columns.
    ///
    /// # Errors
    ///
    /// Returns error if columns are not named or key column not found.
    pub fn inner_join(&self, other: &Sheet, key: &str) -> Result<Sheet> {
        self.join_impl(other, key, key, JoinType::Inner)
    }

    /// Inner join with different column names for the key.
    pub fn inner_join_on(&self, other: &Sheet, left_key: &str, right_key: &str) -> Result<Sheet> {
        self.join_impl(other, left_key, right_key, JoinType::Inner)
    }

    /// Left outer join with another sheet on a key column.
    ///
    /// Returns all rows from the left sheet, with matching rows from right.
    /// Non-matching rows have null values for right columns.
    pub fn left_join(&self, other: &Sheet, key: &str) -> Result<Sheet> {
        self.join_impl(other, key, key, JoinType::Left)
    }

    /// Left outer join with different column names for the key.
    pub fn left_join_on(&self, other: &Sheet, left_key: &str, right_key: &str) -> Result<Sheet> {
        self.join_impl(other, left_key, right_key, JoinType::Left)
    }

    /// Right outer join with another sheet on a key column.
    ///
    /// Returns all rows from the right sheet, with matching rows from left.
    /// Non-matching rows have null values for left columns.
    pub fn right_join(&self, other: &Sheet, key: &str) -> Result<Sheet> {
        self.join_impl(other, key, key, JoinType::Right)
    }

    /// Right outer join with different column names for the key.
    pub fn right_join_on(&self, other: &Sheet, left_key: &str, right_key: &str) -> Result<Sheet> {
        self.join_impl(other, left_key, right_key, JoinType::Right)
    }

    /// Full outer join with another sheet on a key column.
    ///
    /// Returns all rows from both sheets.
    /// Non-matching rows have null values for missing columns.
    pub fn full_join(&self, other: &Sheet, key: &str) -> Result<Sheet> {
        self.join_impl(other, key, key, JoinType::Full)
    }

    /// Full outer join with different column names for the key.
    pub fn full_join_on(&self, other: &Sheet, left_key: &str, right_key: &str) -> Result<Sheet> {
        self.join_impl(other, left_key, right_key, JoinType::Full)
    }

    /// Internal join implementation
    fn join_impl(
        &self,
        other: &Sheet,
        left_key: &str,
        right_key: &str,
        join_type: JoinType,
    ) -> Result<Sheet> {
        // Validate both sheets have named columns
        let left_names = self.column_names.as_ref().ok_or_else(|| {
            SheetError::ColumnsNotNamed("Left sheet columns not named".to_string())
        })?;
        let right_names = other.column_names.as_ref().ok_or_else(|| {
            SheetError::ColumnsNotNamed("Right sheet columns not named".to_string())
        })?;

        // Find key column indices
        let left_key_idx = self
            .column_index
            .as_ref()
            .and_then(|m| m.get(left_key).copied())
            .ok_or_else(|| SheetError::JoinKeyNotFound {
                key: left_key.to_string(),
                sheet: "left".to_string(),
            })?;
        let right_key_idx = other
            .column_index
            .as_ref()
            .and_then(|m| m.get(right_key).copied())
            .ok_or_else(|| SheetError::JoinKeyNotFound {
                key: right_key.to_string(),
                sheet: "right".to_string(),
            })?;

        // Build right key -> row indices map (skip header row if present)
        let right_start = if right_names.iter().any(|n| {
            other
                .data
                .first()
                .is_some_and(|r| r.iter().any(|c| c.as_str() == *n))
        }) {
            1
        } else {
            0
        };
        let left_start = if left_names.iter().any(|n| {
            self.data
                .first()
                .is_some_and(|r| r.iter().any(|c| c.as_str() == *n))
        }) {
            1
        } else {
            0
        };

        let mut right_map: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, row) in other.data.iter().enumerate().skip(right_start) {
            if let Some(cell) = row.get(right_key_idx) {
                let key_val = cell.as_str();
                right_map.entry(key_val).or_default().push(i);
            }
        }

        // Build result columns (left cols + right cols except key)
        let mut result_names: Vec<String> = left_names.clone();
        let right_cols_to_add: Vec<(usize, String)> = right_names
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != right_key_idx)
            .map(|(i, name)| {
                let final_name = if result_names.contains(name) {
                    format!("{}_right", name)
                } else {
                    name.clone()
                };
                (i, final_name)
            })
            .collect();

        for (_, name) in &right_cols_to_add {
            result_names.push(name.clone());
        }

        // Build result data
        let mut result_data: Vec<Vec<CellValue>> = Vec::new();

        // Add header row
        let header: Vec<CellValue> = result_names
            .iter()
            .map(|n| CellValue::String(n.clone()))
            .collect();
        result_data.push(header);

        let left_col_count = left_names.len();
        let right_col_count = right_cols_to_add.len();
        let mut matched_right: std::collections::HashSet<usize> = std::collections::HashSet::new();

        // Process left rows
        for left_row in self.data.iter().skip(left_start) {
            let left_key_val = left_row
                .get(left_key_idx)
                .map(|c| c.as_str())
                .unwrap_or_default();

            if let Some(right_indices) = right_map.get(&left_key_val) {
                // Matching rows found
                for &right_idx in right_indices {
                    matched_right.insert(right_idx);
                    let right_row = &other.data[right_idx];
                    let mut new_row = left_row.clone();
                    for (col_idx, _) in &right_cols_to_add {
                        new_row.push(right_row.get(*col_idx).cloned().unwrap_or(CellValue::Null));
                    }
                    result_data.push(new_row);
                }
            } else if matches!(join_type, JoinType::Left | JoinType::Full) {
                // No match - include left row with nulls for right
                let mut new_row = left_row.clone();
                for _ in 0..right_col_count {
                    new_row.push(CellValue::Null);
                }
                result_data.push(new_row);
            }
        }

        // For right/full join, add unmatched right rows
        if matches!(join_type, JoinType::Right | JoinType::Full) {
            for (i, right_row) in other.data.iter().enumerate().skip(right_start) {
                if !matched_right.contains(&i) {
                    let mut new_row: Vec<CellValue> = vec![CellValue::Null; left_col_count];
                    // Set the key column value from right
                    if left_key_idx < new_row.len() {
                        new_row[left_key_idx] = right_row
                            .get(right_key_idx)
                            .cloned()
                            .unwrap_or(CellValue::Null);
                    }
                    for (col_idx, _) in &right_cols_to_add {
                        new_row.push(right_row.get(*col_idx).cloned().unwrap_or(CellValue::Null));
                    }
                    result_data.push(new_row);
                }
            }
        }

        let mut result = Sheet {
            name: format!("{}_joined", self.name),
            data: result_data,
            column_names: None,
            column_index: None,
            row_names: None,
        };
        result.name_columns_by_row(0)?;

        Ok(result)
    }

    // ===== Append/Upsert Operations =====

    /// Append all rows from another sheet (like SQL UNION ALL).
    ///
    /// Rows are stacked vertically. Column count must match or columns
    /// must be named to align by name.
    pub fn append(&mut self, other: &Sheet) -> Result<()> {
        if self.is_empty() {
            self.data.clone_from(&other.data);
            self.column_names.clone_from(&other.column_names);
            self.column_index.clone_from(&other.column_index);
            return Ok(());
        }

        // If both have named columns, align by name
        if let (Some(self_names), Some(other_names)) = (&self.column_names, &other.column_names) {
            let start_row = if other.data.first().is_some_and(|r| {
                r.iter()
                    .zip(other_names.iter())
                    .all(|(c, n)| c.as_str() == *n)
            }) {
                1 // Skip header row
            } else {
                0
            };

            for other_row in other.data.iter().skip(start_row) {
                let mut new_row = vec![CellValue::Null; self_names.len()];
                for (i, name) in other_names.iter().enumerate() {
                    if let Some(self_idx) = self.column_index.as_ref().and_then(|m| m.get(name)) {
                        if let Some(val) = other_row.get(i) {
                            new_row[*self_idx] = val.clone();
                        }
                    }
                }
                self.data.push(new_row);
            }
        } else {
            // No named columns - must have same column count
            if self.col_count() != other.col_count() {
                return Err(SheetError::ColumnCountMismatch {
                    left: self.col_count(),
                    right: other.col_count(),
                });
            }
            for row in &other.data {
                self.data.push(row.clone());
            }
        }

        self.invalidate_row_names();
        Ok(())
    }

    /// Append rows, skipping duplicates by key column (like SQL UNION).
    ///
    /// Only adds rows from `other` whose key value doesn't exist in self.
    pub fn append_distinct(&mut self, other: &Sheet, key: &str) -> Result<()> {
        let self_key_idx = self.column_index_by_name(key)?;
        let other_key_idx = other
            .column_index
            .as_ref()
            .and_then(|m| m.get(key).copied())
            .ok_or_else(|| SheetError::JoinKeyNotFound {
                key: key.to_string(),
                sheet: "other".to_string(),
            })?;

        // Build set of existing keys (skip header row if columns are named)
        let start_idx = if self.column_names.is_some() { 1 } else { 0 };
        let mut existing_keys: std::collections::HashSet<String> = self
            .data
            .iter()
            .skip(start_idx)
            .filter_map(|row| row.get(self_key_idx).map(|c| c.as_str()))
            .collect();

        let other_names = other.column_names.as_ref();
        let start_row = if let Some(names) = other_names {
            if other
                .data
                .first()
                .is_some_and(|r| r.iter().zip(names.iter()).all(|(c, n)| c.as_str() == *n))
            {
                1
            } else {
                0
            }
        } else {
            0
        };

        for other_row in other.data.iter().skip(start_row) {
            let other_key_val = other_row
                .get(other_key_idx)
                .map(|c| c.as_str())
                .unwrap_or_default();
            if !existing_keys.contains(&other_key_val) {
                // Align columns if named
                if let Some(self_names) = &self.column_names {
                    if let Some(other_names) = &other.column_names {
                        let mut new_row = vec![CellValue::Null; self_names.len()];
                        for (i, name) in other_names.iter().enumerate() {
                            if let Some(self_idx) =
                                self.column_index.as_ref().and_then(|m| m.get(name))
                            {
                                if let Some(val) = other_row.get(i) {
                                    new_row[*self_idx] = val.clone();
                                }
                            }
                        }
                        self.data.push(new_row);
                        existing_keys.insert(other_key_val);
                        continue;
                    }
                }
                self.data.push(other_row.clone());
                existing_keys.insert(other_key_val);
            }
        }

        self.invalidate_row_names();
        Ok(())
    }

    /// Upsert rows from another sheet by key column.
    ///
    /// Updates existing rows (by key) and inserts new rows.
    /// Both sheets must have named columns.
    ///
    /// # Errors
    ///
    /// Returns error if columns are not named or key column not found.
    pub fn upsert(&mut self, other: &Sheet, key: &str) -> Result<()> {
        // Require both sheets to have named columns
        if self.column_names.is_none() {
            return Err(SheetError::ColumnsNotNamed(
                "Self sheet columns not named".to_string(),
            ));
        }
        if other.column_names.is_none() {
            return Err(SheetError::ColumnsNotNamed(
                "Other sheet columns not named".to_string(),
            ));
        }

        let self_key_idx = self.column_index_by_name(key)?;
        let other_key_idx = other
            .column_index
            .as_ref()
            .and_then(|m| m.get(key).copied())
            .ok_or_else(|| SheetError::JoinKeyNotFound {
                key: key.to_string(),
                sheet: "other".to_string(),
            })?;

        // Build map of existing keys to row indices (skip header row)
        let mut key_to_row: HashMap<String, usize> = HashMap::new();
        for (i, row) in self.data.iter().enumerate().skip(1) {
            if let Some(cell) = row.get(self_key_idx) {
                key_to_row.insert(cell.as_str(), i);
            }
        }

        let other_names = other.column_names.as_ref();
        let start_row = if let Some(names) = other_names {
            if other
                .data
                .first()
                .is_some_and(|r| r.iter().zip(names.iter()).all(|(c, n)| c.as_str() == *n))
            {
                1
            } else {
                0
            }
        } else {
            0
        };

        for other_row in other.data.iter().skip(start_row) {
            let other_key_val = other_row
                .get(other_key_idx)
                .map(|c| c.as_str())
                .unwrap_or_default();

            if let Some(&existing_idx) = key_to_row.get(&other_key_val) {
                // Update existing row
                if self.column_names.is_some() {
                    if let Some(other_names) = &other.column_names {
                        for (i, name) in other_names.iter().enumerate() {
                            if let Some(self_idx) =
                                self.column_index.as_ref().and_then(|m| m.get(name))
                            {
                                if let Some(val) = other_row.get(i) {
                                    self.data[existing_idx][*self_idx] = val.clone();
                                }
                            }
                        }
                    }
                }
            } else {
                // Insert new row
                if let Some(self_names) = &self.column_names {
                    if let Some(other_names) = &other.column_names {
                        let mut new_row = vec![CellValue::Null; self_names.len()];
                        for (i, name) in other_names.iter().enumerate() {
                            if let Some(self_idx) =
                                self.column_index.as_ref().and_then(|m| m.get(name))
                            {
                                if let Some(val) = other_row.get(i) {
                                    new_row[*self_idx] = val.clone();
                                }
                            }
                        }
                        self.data.push(new_row.clone());
                        // Update key map for subsequent duplicates in other
                        key_to_row.insert(other_key_val, self.data.len() - 1);
                        continue;
                    }
                }
                self.data.push(other_row.clone());
                key_to_row.insert(other_key_val, self.data.len() - 1);
            }
        }

        self.invalidate_row_names();
        Ok(())
    }
}

/// Join type for internal use
#[derive(Clone, Copy)]
enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

impl Default for Sheet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sheet() {
        let sheet = Sheet::new();
        assert_eq!(sheet.name(), "Sheet1");
        assert!(sheet.is_empty());
        assert_eq!(sheet.row_count(), 0);
        assert_eq!(sheet.col_count(), 0);
    }

    #[test]
    fn test_from_data() {
        let data = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let sheet = Sheet::from_data(data);

        assert_eq!(sheet.row_count(), 2);
        assert_eq!(sheet.col_count(), 3);
        assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(1));
        assert_eq!(sheet.get(1, 2).unwrap(), &CellValue::Int(6));
    }

    #[test]
    fn test_row_operations() {
        let mut sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);

        // Append
        sheet.row_append(vec![5, 6]).unwrap();
        assert_eq!(sheet.row_count(), 3);

        // Insert
        sheet.row_insert(1, vec![7, 8]).unwrap();
        assert_eq!(sheet.get(1, 0).unwrap(), &CellValue::Int(7));

        // Delete
        sheet.row_delete(1).unwrap();
        assert_eq!(sheet.get(1, 0).unwrap(), &CellValue::Int(3));
    }

    #[test]
    fn test_column_operations() {
        let mut sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);

        // Append column
        sheet.column_append(vec![5, 6]).unwrap();
        assert_eq!(sheet.col_count(), 3);
        assert_eq!(sheet.get(0, 2).unwrap(), &CellValue::Int(5));

        // Delete column
        sheet.column_delete(1).unwrap();
        assert_eq!(sheet.col_count(), 2);
    }

    #[test]
    fn test_named_columns() {
        let mut sheet = Sheet::from_data(vec![
            vec!["Name", "Age", "City"],
            vec!["Alice", "30", "NYC"],
            vec!["Bob", "25", "LA"],
        ]);

        sheet.name_columns_by_row(0).unwrap();

        let age_col = sheet.column_by_name("Age").unwrap();
        assert_eq!(age_col.len(), 3);
        assert_eq!(age_col[1], CellValue::String("30".to_string()));
    }

    #[test]
    fn test_filter_rows() {
        let mut sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4], vec![5, 6]]);

        sheet.filter_rows(|row| row[0].as_int().unwrap_or(0) > 2);

        assert_eq!(sheet.row_count(), 2);
        assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(3));
    }

    #[test]
    fn test_map() {
        let mut sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);

        sheet.map(|cell| {
            if let Some(i) = cell.as_int() {
                CellValue::Int(i * 2)
            } else {
                cell.clone()
            }
        });

        assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(2));
        assert_eq!(sheet.get(1, 1).unwrap(), &CellValue::Int(8));
    }

    #[test]
    fn test_to_records() {
        let mut sheet = Sheet::from_data(vec![
            vec!["name", "age"],
            vec!["Alice", "30"],
            vec!["Bob", "25"],
        ]);

        // Without named columns, should return None
        assert!(sheet.to_records().is_none());

        // Name the columns
        sheet.name_columns_by_row(0).unwrap();

        let records = sheet.to_records().unwrap();
        assert_eq!(records.len(), 3); // includes header row

        // Check second record (first data row)
        let alice = &records[1];
        assert_eq!(
            alice.get("name").unwrap(),
            &CellValue::String("Alice".to_string())
        );
        assert_eq!(
            alice.get("age").unwrap(),
            &CellValue::String("30".to_string())
        );
    }

    #[test]
    fn test_from_records() {
        let mut record1 = IndexMap::new();
        record1.insert("name".to_string(), CellValue::String("Alice".to_string()));
        record1.insert("age".to_string(), CellValue::Int(30));

        let mut record2 = IndexMap::new();
        record2.insert("name".to_string(), CellValue::String("Bob".to_string()));
        record2.insert("age".to_string(), CellValue::Int(25));

        let sheet = Sheet::from_records(vec![record1, record2]).unwrap();

        assert_eq!(sheet.row_count(), 3); // header + 2 data rows
        assert_eq!(sheet.col_count(), 2);
        assert!(sheet.column_names().is_some());

        // Check data
        let name_col = sheet.column_by_name("name").unwrap();
        assert_eq!(name_col[1], CellValue::String("Alice".to_string()));
        assert_eq!(name_col[2], CellValue::String("Bob".to_string()));
    }

    #[test]
    fn test_records_roundtrip() {
        let mut record1 = IndexMap::new();
        record1.insert("id".to_string(), CellValue::Int(1));
        record1.insert("value".to_string(), CellValue::String("one".to_string()));

        let mut record2 = IndexMap::new();
        record2.insert("id".to_string(), CellValue::Int(2));
        record2.insert("value".to_string(), CellValue::String("two".to_string()));

        // Create sheet from records
        let sheet = Sheet::from_records(vec![record1.clone(), record2.clone()]).unwrap();

        // Convert back to records
        let records = sheet.to_records().unwrap();

        // Skip header row (index 0), check data rows
        assert_eq!(records[1].get("id").unwrap(), &CellValue::Int(1));
        assert_eq!(
            records[2].get("value").unwrap(),
            &CellValue::String("two".to_string())
        );
    }
}
