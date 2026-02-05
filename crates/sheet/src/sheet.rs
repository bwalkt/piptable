use crate::cell::CellValue;
use crate::error::{Result, SheetError};
use indexmap::IndexMap;
use piptable_formulas::{FormulaEngine, ValueResolver};
use piptable_primitives::{CellAddress, CellRange, ErrorValue, Value};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use validator::ValidateEmail;

/// Strategy for handling null or empty values during cleaning.
#[derive(Debug, Clone)]
pub enum NullStrategy {
    Keep,
    EmptyToNull,
    NullToEmpty,
    FillWith(CellValue),
}

impl Default for NullStrategy {
    fn default() -> Self {
        NullStrategy::Keep
    }
}

/// Options for bulk data cleaning.
#[derive(Debug, Clone)]
pub struct CleanOptions {
    pub trim: bool,
    pub lower: bool,
    pub upper: bool,
    pub normalize_whitespace: bool,
    pub null_strategy: NullStrategy,
    pub preserve_formulas: bool,
}

impl Default for CleanOptions {
    fn default() -> Self {
        CleanOptions {
            trim: false,
            lower: false,
            upper: false,
            normalize_whitespace: false,
            null_strategy: NullStrategy::Keep,
            preserve_formulas: true,
        }
    }
}

/// Rules supported by `validate_column`.
#[derive(Debug, Clone)]
pub enum ValidationRule {
    Email,
    Phone,
    Range { min: f64, max: f64 },
    Regex(String),
}

/// A sheet representing a 2D grid of cells (row-major storage)
#[derive(Debug, Clone)]
pub struct Sheet {
    name: String,
    data: Vec<Vec<CellValue>>,
    column_names: Option<Vec<String>>,
    column_index: Option<HashMap<String, usize>>,
    row_names: Option<HashMap<String, usize>>,
    formula_engine: FormulaEngine,
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
            formula_engine: FormulaEngine::new(),
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
            formula_engine: FormulaEngine::new(),
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

    /// Remove duplicate rows based on the provided column names.
    /// Returns the number of rows removed.
    pub fn remove_duplicates_by_columns(&mut self, columns: &[&str]) -> Result<usize> {
        let start_row = if self.column_names.is_some() { 1 } else { 0 };
        let indices: Vec<usize> = if columns.is_empty() {
            (0..self.col_count()).collect()
        } else {
            columns
                .iter()
                .map(|name| self.column_index_by_name(name))
                .collect::<Result<Vec<_>>>()?
        };

        let mut seen = HashSet::new();
        let mut new_data = Vec::with_capacity(self.data.len());
        let mut removed = 0usize;

        if start_row == 1 && !self.data.is_empty() {
            new_data.push(self.data[0].clone());
        }

        for (_row_idx, row) in self.data.iter().enumerate().skip(start_row) {
            let mut key = String::new();
            for &col in &indices {
                let cell = row.get(col).unwrap_or(&CellValue::Null);
                key.push_str(&Self::cell_key(cell));
                key.push('\x1f');
            }
            if seen.insert(key) {
                new_data.push(row.clone());
            } else {
                removed += 1;
            }
        }

        self.data = new_data;
        self.invalidate_row_names();
        self.rebuild_formula_engine()?;
        Ok(removed)
    }

    /// Validate a column and return the row indices that fail the rule.
    pub fn validate_column(&self, column: &str, rule: ValidationRule) -> Result<Vec<usize>> {
        let col_index = self.column_index_by_name(column)?;
        let start_row = if self.column_names.is_some() { 1 } else { 0 };

        let phone_regex =
            if matches!(rule, ValidationRule::Phone) {
                Some(Regex::new(r"^\\+?[0-9().\\-\\s]{7,}$").map_err(|e| {
                    SheetError::Parse(format!("Invalid phone validation regex: {e}"))
                })?)
            } else {
                None
            };
        let custom_regex = if let ValidationRule::Regex(pattern) = &rule {
            Some(
                Regex::new(pattern)
                    .map_err(|e| SheetError::Parse(format!("Invalid validation regex: {e}")))?,
            )
        } else {
            None
        };

        let mut invalid = Vec::new();
        for (row_idx, row) in self.data.iter().enumerate().skip(start_row) {
            let cell = row.get(col_index).unwrap_or(&CellValue::Null);
            if !Self::cell_matches_rule(cell, &rule, phone_regex.as_ref(), custom_regex.as_ref()) {
                invalid.push(row_idx);
            }
        }
        Ok(invalid)
    }

    /// Clean data in-place using the provided options.
    pub fn clean_data(&mut self, options: &CleanOptions) -> Result<()> {
        for row in &mut self.data {
            for cell in row {
                if options.preserve_formulas && matches!(cell, CellValue::Formula(_)) {
                    continue;
                }
                *cell = Self::clean_cell(cell, options);
            }
        }

        self.invalidate_row_names();
        self.rebuild_formula_engine()?;
        Ok(())
    }

    /// Clean data in-place for a specific range (A1 or absolute R1C1).
    pub fn clean_data_range(&mut self, range: &str, options: &CleanOptions) -> Result<()> {
        let ((start_row, start_col), (end_row, end_col)) =
            crate::a1_notation::parse_range_notation(range)?;

        if end_row >= self.row_count() {
            return Err(SheetError::RowIndexOutOfBounds {
                index: end_row,
                count: self.row_count(),
            });
        }
        if end_col >= self.col_count() {
            return Err(SheetError::ColumnIndexOutOfBounds {
                index: end_col,
                count: self.col_count(),
            });
        }

        for row_idx in start_row..=end_row {
            if let Some(row) = self.data.get_mut(row_idx) {
                for col_idx in start_col..=end_col {
                    if let Some(cell) = row.get_mut(col_idx) {
                        if options.preserve_formulas && matches!(cell, CellValue::Formula(_)) {
                            continue;
                        }
                        *cell = Self::clean_cell(cell, options);
                    }
                }
            }
        }

        self.invalidate_row_names();
        self.rebuild_formula_engine()?;
        Ok(())
    }

    fn clean_cell(cell: &CellValue, options: &CleanOptions) -> CellValue {
        let mut updated = match cell.cached_or_self() {
            CellValue::String(s) => {
                let mut out = s.clone();
                if options.trim {
                    out = out.trim().to_string();
                }
                if options.normalize_whitespace {
                    out = Self::normalize_whitespace(&out);
                }
                if options.upper {
                    out = out.to_uppercase();
                } else if options.lower {
                    out = out.to_lowercase();
                }
                CellValue::String(out)
            }
            _ => cell.clone(),
        };

        if matches!(options.null_strategy, NullStrategy::EmptyToNull) {
            if let CellValue::String(s) = &updated {
                if s.is_empty() {
                    updated = CellValue::Null;
                }
            }
        }

        match &options.null_strategy {
            NullStrategy::NullToEmpty => {
                if matches!(updated.cached_or_self(), CellValue::Null) {
                    updated = CellValue::String(String::new());
                }
            }
            NullStrategy::FillWith(value) => {
                if matches!(updated.cached_or_self(), CellValue::Null) {
                    updated = value.clone();
                }
            }
            _ => {}
        }

        updated
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
        if matches!(self.get(row, col)?, CellValue::Formula(_)) {
            self.formula_engine
                .remove_formula(&CellAddress::new(row as u32, col as u32));
        }
        let cell = self.get_mut(row, col)?;
        *cell = value.into();
        self.formula_engine
            .mark_dirty(&CellAddress::new(row as u32, col as u32));
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

    // ===== A1-Style Notation Access =====

    /// Get a cell value using A1-style notation (e.g., "A1", "B2")
    pub fn get_a1(&self, notation: &str) -> Result<&CellValue> {
        let (row, col) = crate::a1_notation::parse_cell_notation(notation)?;
        self.get(row, col)
    }

    /// Get the resolved address for a cell notation string.
    pub fn get_a1_addr(&self, notation: &str) -> Result<piptable_primitives::CellAddress> {
        let (row, col) = crate::a1_notation::parse_cell_notation(notation)?;
        Ok(piptable_primitives::CellAddress::new(
            row as u32, col as u32,
        ))
    }

    /// Get a mutable cell reference using A1-style notation
    pub fn get_a1_mut(&mut self, notation: &str) -> Result<&mut CellValue> {
        let (row, col) = crate::a1_notation::parse_cell_notation(notation)?;
        self.get_mut(row, col)
    }

    /// Set a cell value using A1-style notation
    pub fn set_a1<T: Into<CellValue>>(&mut self, notation: &str, value: T) -> Result<()> {
        let (row, col) = crate::a1_notation::parse_cell_notation(notation)?;
        self.set(row, col, value)
    }

    /// Set a formula in a cell using A1-style notation (e.g., "C1", "=SUM(A1:B1)").
    ///
    /// This stores the formula source and marks the cell dirty; call
    /// [`Sheet::evaluate_formulas`] to compute cached results.
    pub fn set_formula(&mut self, notation: &str, formula: &str) -> Result<()> {
        let addr = self.get_a1_addr(notation)?;
        let _ = self.get(addr.row as usize, addr.col as usize)?;
        self.set_cell_value_raw(
            addr.row as usize,
            addr.col as usize,
            CellValue::formula(formula.to_string()),
        )?;
        self.formula_engine.set_formula(addr, formula)?;
        self.formula_engine.mark_dirty(&addr);
        Ok(())
    }

    /// Evaluate dirty formulas in dependency order and update cached results.
    ///
    /// Formula cells keep their source string and store the computed value in the cache.
    pub fn evaluate_formulas(&mut self) -> Result<()> {
        let dirty = self.formula_engine.get_dirty_nodes()?;
        for cell in dirty {
            let Some(compiled) = self.formula_engine.get_formula(&cell) else {
                continue;
            };
            let resolver = SheetValueResolver::new(self, Some(cell));
            let value = self.formula_engine.evaluate(compiled, &resolver)?;
            let cell_value = formula_value_to_cell_value(value);
            if let Ok(CellValue::Formula(formula)) =
                self.get_mut(cell.row as usize, cell.col as usize)
            {
                formula.cached = Some(Box::new(cell_value));
                continue;
            }
            self.set_cell_value_raw(cell.row as usize, cell.col as usize, cell_value)?;
        }
        Ok(())
    }

    /// Get a sub-sheet using A1-style range notation (e.g., "A1:C3")
    pub fn get_range(&self, range: &str) -> Result<Sheet> {
        let ((start_row, start_col), (end_row, end_col)) =
            crate::a1_notation::parse_range_notation(range)?;

        // Validate bounds
        if end_row >= self.row_count() {
            return Err(SheetError::RowIndexOutOfBounds {
                index: end_row,
                count: self.row_count(),
            });
        }
        if end_col >= self.col_count() {
            return Err(SheetError::ColumnIndexOutOfBounds {
                index: end_col,
                count: self.col_count(),
            });
        }

        // Extract sub-sheet
        let mut data = Vec::new();
        for row_idx in start_row..=end_row {
            let row = &self.data[row_idx];
            let mut new_row = Vec::new();
            for cell in row.iter().take(end_col + 1).skip(start_col) {
                new_row.push(cell.clone());
            }
            data.push(new_row);
        }

        let mut sub_sheet = Sheet::from_data(data);
        sub_sheet.set_name(&format!("Range {}", range));

        // Copy column names if they exist and are in range
        if let Some(col_names) = &self.column_names {
            if start_col < col_names.len() && end_col < col_names.len() {
                let sub_names: Vec<String> = col_names[start_col..=end_col].to_vec();
                sub_sheet.column_names = Some(sub_names);

                // Rebuild column index
                let mut index_map = HashMap::new();
                for (i, name) in sub_sheet.column_names.as_ref().unwrap().iter().enumerate() {
                    index_map.insert(name.clone(), i);
                }
                sub_sheet.column_index = Some(index_map);
            }
        }

        Ok(sub_sheet)
    }

    fn set_cell_value_raw(&mut self, row: usize, col: usize, value: CellValue) -> Result<()> {
        let cell = self.get_mut(row, col)?;
        *cell = value;
        Ok(())
    }

    fn mark_dirty_range(
        &mut self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) {
        for row in start_row..=end_row {
            for col in start_col..=end_col {
                self.formula_engine
                    .mark_dirty(&CellAddress::new(row as u32, col as u32));
            }
        }
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
        if self.col_count() > 0 {
            let row_idx = self.row_count().saturating_sub(1);
            self.mark_dirty_range(row_idx, 0, row_idx, self.col_count().saturating_sub(1));
        }
        self.rebuild_formula_engine()?;
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
        if self.col_count() > 0 {
            self.mark_dirty_range(index, 0, index, self.col_count().saturating_sub(1));
        }
        self.rebuild_formula_engine()?;
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
        if self.col_count() > 0 {
            self.mark_dirty_range(index, 0, index, self.col_count().saturating_sub(1));
        }
        self.rebuild_formula_engine()?;
        Ok(())
    }

    /// Update a row by name
    pub fn row_update_by_name<T: Into<CellValue>>(
        &mut self,
        name: &str,
        data: Vec<T>,
    ) -> Result<()> {
        let index = self.row_index_by_name(name)?;
        self.row_update(index, data)
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
        let removed = self.data.remove(index);
        self.rebuild_formula_engine()?;
        Ok(removed)
    }

    /// Delete multiple rows by indices (in descending order to maintain indices)
    pub fn row_delete_multi(&mut self, mut indices: Vec<usize>) -> Result<()> {
        indices.sort_unstable();
        indices.reverse();

        for index in indices {
            if index >= self.row_count() {
                return Err(SheetError::RowIndexOutOfBounds {
                    index,
                    count: self.row_count(),
                });
            }
            self.data.remove(index);
        }
        self.invalidate_row_names();
        self.rebuild_formula_engine()?;
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
        let _ = self.rebuild_formula_engine();
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
        let col_idx = self.col_count().saturating_sub(1);
        if self.row_count() > 0 && self.col_count() > 0 {
            self.mark_dirty_range(0, col_idx, self.row_count().saturating_sub(1), col_idx);
        }
        self.rebuild_formula_engine()?;
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
        if self.row_count() > 0 {
            self.mark_dirty_range(0, index, self.row_count().saturating_sub(1), index);
        }
        self.rebuild_formula_engine()?;
        Ok(())
    }

    /// Update a column at a specific index
    pub fn column_update<T: Into<CellValue>>(&mut self, index: usize, data: Vec<T>) -> Result<()> {
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

        if self.row_count() > 0 {
            self.mark_dirty_range(0, index, self.row_count().saturating_sub(1), index);
        }
        self.rebuild_formula_engine()?;
        Ok(())
    }

    /// Update a column by name
    pub fn column_update_by_name<T: Into<CellValue>>(
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
        self.rebuild_formula_engine()?;
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
            if index >= self.col_count() {
                return Err(SheetError::ColumnIndexOutOfBounds {
                    index,
                    count: self.col_count(),
                });
            }
            for row in &mut self.data {
                row.remove(index);
            }
        }

        self.invalidate_column_names();
        self.rebuild_formula_engine()?;
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

    fn normalize_whitespace(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut last_was_space = false;
        for ch in input.chars() {
            if ch.is_whitespace() {
                if !last_was_space {
                    out.push(' ');
                    last_was_space = true;
                }
            } else {
                out.push(ch);
                last_was_space = false;
            }
        }
        out.trim().to_string()
    }

    fn cell_key(cell: &CellValue) -> String {
        match cell.cached_or_self() {
            CellValue::Null => "N".to_string(),
            CellValue::Bool(b) => format!("B{b}"),
            CellValue::Int(i) => format!("I{i}"),
            CellValue::Float(f) => format!("F{f:?}"),
            CellValue::String(s) => format!("S{s}"),
            CellValue::Formula(formula) => format!("FML{}", formula.source),
        }
    }

    fn cell_matches_rule(
        cell: &CellValue,
        rule: &ValidationRule,
        phone_regex: Option<&Regex>,
        custom_regex: Option<&Regex>,
    ) -> bool {
        let value = match cell.cached_or_self() {
            CellValue::Null => return false,
            CellValue::String(s) => s.clone(),
            other => other.as_str(),
        };

        match rule {
            ValidationRule::Email => value.validate_email(),
            ValidationRule::Phone => phone_regex
                .map(|re| re.is_match(&value) && value.chars().any(|ch| ch.is_ascii_digit()))
                .unwrap_or(false),
            ValidationRule::Range { min, max } => value
                .parse::<f64>()
                .map(|v| v >= *min && v <= *max)
                .unwrap_or(false),
            ValidationRule::Regex(_) => custom_regex.map(|re| re.is_match(&value)).unwrap_or(false),
        }
    }

    fn invalidate_column_names(&mut self) {
        self.column_names = None;
        self.column_index = None;
    }

    fn invalidate_row_names(&mut self) {
        self.row_names = None;
    }

    fn rebuild_formula_engine(&mut self) -> Result<()> {
        let mut engine = FormulaEngine::new();
        let mut first_error: Option<SheetError> = None;
        for (row_idx, row) in self.data.iter_mut().enumerate() {
            for (col_idx, cell) in row.iter_mut().enumerate() {
                if let CellValue::Formula(formula) = cell {
                    formula.cached = None;
                    let addr = CellAddress::new(row_idx as u32, col_idx as u32);
                    if let Err(err) = engine.set_formula(addr, &formula.source) {
                        if first_error.is_none() {
                            first_error = Some(err.into());
                        }
                        continue;
                    }
                    engine.mark_dirty(&addr);
                }
            }
        }
        self.formula_engine = engine;
        if let Some(err) = first_error {
            Err(err)
        } else {
            Ok(())
        }
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
        if let Err(err) = self.rebuild_formula_engine() {
            eprintln!("Warning: formula engine rebuild failed: {err}");
        }
    }

    /// Apply a function to a range of cells (A1 or absolute R1C1).
    pub fn map_range<F>(&mut self, range: &str, f: F) -> Result<()>
    where
        F: Fn(&CellValue) -> CellValue,
    {
        let ((start_row, start_col), (end_row, end_col)) =
            crate::a1_notation::parse_range_notation(range)?;

        if end_row >= self.row_count() {
            return Err(SheetError::RowIndexOutOfBounds {
                index: end_row,
                count: self.row_count(),
            });
        }
        if end_col >= self.col_count() {
            return Err(SheetError::ColumnIndexOutOfBounds {
                index: end_col,
                count: self.col_count(),
            });
        }

        for row_idx in start_row..=end_row {
            if let Some(row) = self.data.get_mut(row_idx) {
                for col_idx in start_col..=end_col {
                    if let Some(cell) = row.get_mut(col_idx) {
                        *cell = f(cell);
                    }
                }
            }
        }

        self.invalidate_row_names();
        self.rebuild_formula_engine()?;
        Ok(())
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

        self.rebuild_formula_engine()?;
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
    /// The predicate receives the row index and the row data
    pub fn filter_rows<F>(&mut self, predicate: F)
    where
        F: Fn(usize, &[CellValue]) -> bool,
    {
        let mut keep = Vec::new();
        for (i, row) in self.data.iter().enumerate() {
            if predicate(i, row) {
                keep.push(row.clone());
            }
        }
        self.data = keep;
        self.invalidate_row_names();
        if let Err(err) = self.rebuild_formula_engine() {
            eprintln!("Warning: formula engine rebuild failed: {err}");
        }
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
        self.rebuild_formula_engine()?;
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
            formula_engine: FormulaEngine::new(),
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

    // ===== Bulk Operations =====

    /// Apply a function to every cell in the sheet (consuming version)
    #[must_use]
    pub fn map_into<F>(mut self, f: F) -> Self
    where
        F: Fn(&CellValue) -> CellValue,
    {
        self.map(f);
        self
    }

    /// Filter columns based on a predicate
    ///
    /// The predicate receives the column index and name. If columns are not
    /// named, the name parameter will be an empty string.
    pub fn filter_columns<F>(&mut self, f: F) -> Result<()>
    where
        F: Fn(usize, &str) -> bool,
    {
        if let Some(names) = &self.column_names {
            let mut keep_indices = Vec::new();
            let mut new_names = Vec::new();

            for (i, name) in names.iter().enumerate() {
                if f(i, name) {
                    keep_indices.push(i);
                    new_names.push(name.clone());
                }
            }

            // Filter data columns
            for row in &mut self.data {
                let new_row: Vec<CellValue> = keep_indices
                    .iter()
                    .filter_map(|&i| row.get(i).cloned())
                    .collect();
                *row = new_row;
            }

            // Update column names and index
            self.column_names = Some(new_names);
            self.rebuild_column_index();
            self.invalidate_row_names();
        } else {
            // Filter by index only if no names
            let col_count = self.col_count();
            let mut keep_indices = Vec::new();

            for i in 0..col_count {
                if f(i, "") {
                    keep_indices.push(i);
                }
            }

            for row in &mut self.data {
                let new_row: Vec<CellValue> = keep_indices
                    .iter()
                    .filter_map(|&i| row.get(i).cloned())
                    .collect();
                *row = new_row;
            }
            self.invalidate_row_names();
        }
        self.rebuild_formula_engine()?;
        Ok(())
    }

    /// Format a specific column using a function
    pub fn format_column<F>(&mut self, col_index: usize, f: F) -> Result<()>
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
            if let Some(cell) = row.get_mut(col_index) {
                *cell = f(cell);
            }
        }
        self.rebuild_formula_engine()?;
        Ok(())
    }

    /// Format a column by name using a function
    pub fn format_column_by_name<F>(&mut self, col_name: &str, f: F) -> Result<()>
    where
        F: Fn(&CellValue) -> CellValue,
    {
        let col_index = self.column_index_by_name(col_name)?;
        self.format_column(col_index, f)
    }

    /// Remove empty rows (rows where all cells are null or empty strings)
    pub fn remove_empty_rows(&mut self) {
        self.data.retain(|row| {
            !row.iter().all(|cell| match cell.cached_or_self() {
                CellValue::Null => true,
                CellValue::String(s) if s.is_empty() => true,
                _ => false,
            })
        });
        self.invalidate_row_names();
        if let Err(err) = self.rebuild_formula_engine() {
            eprintln!("Warning: formula engine rebuild failed: {err}");
        }
    }

    /// Transpose the sheet (swap rows and columns)
    pub fn transpose(&mut self) {
        if self.data.is_empty() {
            return;
        }

        let rows = self.row_count();
        let cols = self.col_count();

        let mut transposed = Vec::with_capacity(cols);
        for col in 0..cols {
            let mut new_row = Vec::with_capacity(rows);
            for row in 0..rows {
                new_row.push(self.data[row].get(col).cloned().unwrap_or(CellValue::Null));
            }
            transposed.push(new_row);
        }

        self.data = transposed;

        // Swap column names with first column if they exist
        if self.column_names.is_some() {
            // Column names become first column after transpose
            self.column_names = None;
            self.column_index = None;
        }

        // Clear row names as they're no longer valid
        self.row_names = None;
        if let Err(err) = self.rebuild_formula_engine() {
            eprintln!("Warning: formula engine rebuild failed: {err}");
        }
    }

    /// Cherry-pick columns: Keep only the specified columns
    pub fn select_columns(&mut self, columns: &[&str]) -> Result<()> {
        let indices: Result<Vec<usize>> = columns
            .iter()
            .map(|name| self.column_index_by_name(name))
            .collect();

        let indices = indices?;

        // Create new data with only selected columns
        for row in &mut self.data {
            let new_row: Vec<CellValue> = indices
                .iter()
                .filter_map(|&i| row.get(i).cloned())
                .collect();
            *row = new_row;
        }

        // Update column names
        if self.column_names.is_some() {
            let new_names: Vec<String> = columns.iter().map(|s| s.to_string()).collect();
            self.column_names = Some(new_names);
            self.rebuild_column_index();
        }
        self.invalidate_row_names();
        self.rebuild_formula_engine()?;

        Ok(())
    }

    /// Remove the specified columns (opposite of select_columns)
    pub fn remove_columns(&mut self, columns: &[&str]) -> Result<()> {
        if let Some(names) = &self.column_names {
            let remove_indices: Result<Vec<usize>> = columns
                .iter()
                .map(|name| self.column_index_by_name(name))
                .collect();

            let remove_indices = remove_indices?;
            let remove_set: HashSet<usize> = remove_indices.into_iter().collect();

            let mut keep_indices = Vec::new();
            let mut new_names = Vec::new();

            for (i, name) in names.iter().enumerate() {
                if !remove_set.contains(&i) {
                    keep_indices.push(i);
                    new_names.push(name.clone());
                }
            }

            // Filter data columns
            for row in &mut self.data {
                let new_row: Vec<CellValue> = keep_indices
                    .iter()
                    .filter_map(|&i| row.get(i).cloned())
                    .collect();
                *row = new_row;
            }

            // Update column names and index
            self.column_names = Some(new_names);
            self.rebuild_column_index();
            self.invalidate_row_names();
        } else {
            return Err(SheetError::ColumnsNotNamed(
                "Cannot remove columns by name without named columns".to_string(),
            ));
        }

        self.rebuild_formula_engine()?;
        Ok(())
    }

    // Helper to rebuild column index after modifications
    fn rebuild_column_index(&mut self) {
        if let Some(names) = &self.column_names {
            let mut index_map = HashMap::new();
            for (i, name) in names.iter().enumerate() {
                index_map.insert(name.clone(), i);
            }
            self.column_index = Some(index_map);
        }
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
        // Use strict matching: first row must match ALL column names in order
        let right_start = if other.data.first().is_some_and(|r| {
            r.iter()
                .zip(right_names.iter())
                .all(|(c, n)| c.as_str() == *n)
        }) {
            1
        } else {
            0
        };
        let left_start = if self.data.first().is_some_and(|r| {
            r.iter()
                .zip(left_names.iter())
                .all(|(c, n)| c.as_str() == *n)
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
        let mut matched_right: HashSet<usize> = HashSet::new();

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
            formula_engine: FormulaEngine::new(),
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
            self.rebuild_formula_engine()?;
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
        self.rebuild_formula_engine()?;
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
        let mut existing_keys: HashSet<String> = self
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
        self.rebuild_formula_engine()?;
        Ok(())
    }

    /// Concatenate columns from another sheet (like a horizontal merge).
    ///
    /// Row counts must match. If both sheets have named columns, names are
    /// preserved and right-side duplicates are suffixed with _1, _2, etc.
    pub fn concat_columns(&self, other: &Sheet) -> Result<Sheet> {
        if self.row_count() != other.row_count() {
            return Err(SheetError::LengthMismatch {
                expected: self.row_count(),
                actual: other.row_count(),
            });
        }

        let mut data = Vec::with_capacity(self.row_count());
        for (left, right) in self.data.iter().zip(other.data.iter()) {
            let mut row = Vec::with_capacity(left.len() + right.len());
            row.extend(left.iter().cloned());
            row.extend(right.iter().cloned());
            data.push(row);
        }

        let mut result = Sheet::from_data(data);
        result.set_name(&format!("{}_concat", self.name));

        if let (Some(left_names), Some(right_names)) = (&self.column_names, &other.column_names) {
            let mut names = left_names.clone();
            let mut used: HashSet<String> = names.iter().cloned().collect();
            for name in right_names {
                let mut candidate = name.clone();
                if used.contains(&candidate) {
                    let mut suffix = 1;
                    loop {
                        let next = format!("{candidate}_{suffix}");
                        if !used.contains(&next) {
                            candidate = next;
                            break;
                        }
                        suffix += 1;
                    }
                }
                used.insert(candidate.clone());
                names.push(candidate);
            }
            result.column_names = Some(names);
            result.rebuild_column_index();
        }

        result.rebuild_formula_engine()?;
        Ok(result)
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
        self.rebuild_formula_engine()?;
        Ok(())
    }
}

impl std::ops::Add for Sheet {
    type Output = Result<Sheet>;

    fn add(mut self, rhs: Sheet) -> Self::Output {
        self.append(&rhs)?;
        Ok(self)
    }
}

impl std::ops::Add<&Sheet> for &Sheet {
    type Output = Result<Sheet>;

    fn add(self, rhs: &Sheet) -> Self::Output {
        let mut out = self.clone();
        out.append(rhs)?;
        Ok(out)
    }
}

impl std::ops::BitOr for Sheet {
    type Output = Result<Sheet>;

    fn bitor(self, rhs: Sheet) -> Self::Output {
        self.concat_columns(&rhs)
    }
}

impl std::ops::BitOr<&Sheet> for &Sheet {
    type Output = Result<Sheet>;

    fn bitor(self, rhs: &Sheet) -> Self::Output {
        self.concat_columns(rhs)
    }
}

struct SheetValueResolver<'a> {
    sheet: &'a Sheet,
    current: Option<CellAddress>,
}

impl<'a> SheetValueResolver<'a> {
    fn new(sheet: &'a Sheet, current: Option<CellAddress>) -> Self {
        Self { sheet, current }
    }
}

impl ValueResolver for SheetValueResolver<'_> {
    fn get_cell(&self, addr: &CellAddress) -> Value {
        let row = addr.row as usize;
        let col = addr.col as usize;
        match self.sheet.get(row, col) {
            Ok(cell) => cell_value_to_formula_value(cell),
            Err(_) => Value::Error(ErrorValue::Ref),
        }
    }

    fn get_range(&self, range: &CellRange) -> Vec<Value> {
        let range = range.normalized();
        let mut rows = Vec::new();
        for row in range.start.row..=range.end.row {
            let mut cols = Vec::new();
            for col in range.start.col..=range.end.col {
                let addr = CellAddress::new(row, col);
                cols.push(self.get_cell(&addr));
            }
            rows.push(Value::Array(cols));
        }
        rows
    }

    fn current_cell(&self) -> Option<CellAddress> {
        self.current
    }
}

fn cell_value_to_formula_value(value: &CellValue) -> Value {
    match value.cached_or_self() {
        CellValue::Null => Value::Empty,
        CellValue::Bool(v) => Value::Bool(*v),
        CellValue::Int(v) => Value::Int(*v),
        CellValue::Float(v) => Value::Float(*v),
        CellValue::String(v) => Value::String(v.clone()),
        CellValue::Formula(_) => Value::Error(ErrorValue::Value),
    }
}

fn formula_value_to_cell_value(value: Value) -> CellValue {
    match value {
        Value::Empty => CellValue::Null,
        Value::Bool(v) => CellValue::Bool(v),
        Value::Int(v) => CellValue::Int(v),
        Value::Float(v) => CellValue::Float(v),
        Value::String(v) => CellValue::String(v),
        Value::Error(err) => CellValue::String(err.label().to_string()),
        Value::Array(values) => {
            let Some(first) = values.first() else {
                return CellValue::Null;
            };
            if values.len() == 1 {
                return match first {
                    Value::Array(inner) if inner.len() == 1 => {
                        formula_value_to_cell_value(inner[0].clone())
                    }
                    _ => formula_value_to_cell_value(first.clone()),
                };
            }
            let serialized = serde_json::to_string(&values).unwrap_or_default();
            CellValue::String(serialized)
        }
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

        sheet.filter_rows(|_idx, row| row[0].as_int().unwrap_or(0) > 2);

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
    fn test_map_range_a1_and_r1c1() {
        let mut sheet = Sheet::from_data(vec![vec![1, 2], vec![3, 4]]);

        sheet
            .map_range("A1:B1", |cell| {
                if let Some(i) = cell.as_int() {
                    CellValue::Int(i * 10)
                } else {
                    cell.clone()
                }
            })
            .unwrap();

        assert_eq!(sheet.get(0, 0).unwrap(), &CellValue::Int(10));
        assert_eq!(sheet.get(0, 1).unwrap(), &CellValue::Int(20));
        assert_eq!(sheet.get(1, 0).unwrap(), &CellValue::Int(3));

        sheet
            .map_range("R2C1:R2C2", |_cell| CellValue::Int(0))
            .unwrap();
        assert_eq!(sheet.get(1, 0).unwrap(), &CellValue::Int(0));
        assert_eq!(sheet.get(1, 1).unwrap(), &CellValue::Int(0));
    }

    #[test]
    fn test_clean_data_range() {
        let mut sheet = Sheet::from_data(vec![
            vec!["Name", "Note"],
            vec!["  Alice  ", " Keep "],
            vec!["  BOB  ", ""],
        ]);

        let mut options = CleanOptions::default();
        options.trim = true;
        options.lower = true;
        options.null_strategy = NullStrategy::EmptyToNull;

        sheet.clean_data_range("A2:A3", &options).unwrap();

        assert_eq!(
            sheet.get(1, 0).unwrap(),
            &CellValue::String("alice".to_string())
        );
        assert_eq!(
            sheet.get(2, 0).unwrap(),
            &CellValue::String("bob".to_string())
        );
        assert_eq!(sheet.get(2, 1).unwrap(), &CellValue::String("".to_string()));
    }

    #[test]
    fn test_concat_columns_and_operators() {
        let mut left = Sheet::from_data(vec![
            vec![
                CellValue::String("A".to_string()),
                CellValue::String("B".to_string()),
            ],
            vec![CellValue::Int(1), CellValue::Int(2)],
        ]);
        left.name_columns_by_row(0).unwrap();
        let mut right = Sheet::from_data(vec![
            vec![
                CellValue::String("B".to_string()),
                CellValue::String("C".to_string()),
            ],
            vec![CellValue::Int(3), CellValue::Int(4)],
        ]);
        right.name_columns_by_row(0).unwrap();

        let merged = left.concat_columns(&right).unwrap();
        assert_eq!(merged.col_count(), 4);
        let names = merged.column_names().unwrap();
        assert_eq!(
            names,
            &vec![
                "A".to_string(),
                "B".to_string(),
                "B_1".to_string(),
                "C".to_string()
            ]
        );

        let append = (&left + &right).unwrap();
        assert_eq!(append.row_count(), 3);

        let columns = (&left | &right).unwrap();
        assert_eq!(columns.col_count(), 4);
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

    #[test]
    fn test_uncached_formula_resolves_to_error_value() {
        let cell = CellValue::formula("=A1+1");
        let value = cell_value_to_formula_value(&cell);
        assert!(matches!(value, Value::Error(ErrorValue::Value)));
    }
}
