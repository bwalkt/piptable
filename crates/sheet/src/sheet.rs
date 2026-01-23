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
            .ok_or(SheetError::ColumnsNotNamed)?
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
}
