//! Lookup functions (VLOOKUP, HLOOKUP, INDEX, MATCH, XLOOKUP)

use crate::Interpreter;
use piptable_core::{PipError, PipResult, Value};

/// Handle lookup function calls
pub async fn call_lookup_builtin(
    _interpreter: &Interpreter,
    name: &str,
    args: Vec<Value>,
    line: usize,
) -> Option<PipResult<Value>> {
    match name {
        "vlookup" => Some(vlookup(args, line)),
        "hlookup" => Some(hlookup(args, line)),
        "index" => Some(index(args, line)),
        "match" => Some(match_fn(args, line)),
        "xlookup" => Some(xlookup(args, line)),
        _ => None,
    }
}

/// VLOOKUP(lookup_value, table_array, col_index_num, [range_lookup])
/// Searches for a value in the leftmost column of a table and returns a value in the same row from a specified column.
fn vlookup(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() < 3 || args.len() > 4 {
        return Err(PipError::runtime(
            line,
            "VLOOKUP requires 3 or 4 arguments: VLOOKUP(lookup_value, table_array, col_index_num, [range_lookup])",
        ));
    }

    let lookup_value = &args[0];
    let Value::Array(table_array) = &args[1] else {
        return Err(PipError::runtime(
            line,
            "VLOOKUP: table_array must be an array",
        ));
    };

    let col_index = match &args[2] {
        Value::Int(n) => {
            if *n < 1 {
                return Err(PipError::runtime(
                    line,
                    "VLOOKUP: col_index_num must be at least 1",
                ));
            }
            *n as usize
        }
        Value::Float(f) => {
            if f.is_nan() || f.is_infinite() || *f < 1.0 {
                return Err(PipError::runtime(
                    line,
                    "VLOOKUP: col_index_num must be a positive number",
                ));
            }
            *f as usize
        }
        _ => {
            return Err(PipError::runtime(
                line,
                "VLOOKUP: col_index_num must be a number",
            ))
        }
    };

    let exact_match = if args.len() == 4 {
        match &args[3] {
            Value::Bool(b) => !b,  // FALSE means exact match in Excel
            Value::Int(0) => true, // 0 means exact match
            _ => false,            // Any other value means approximate match
        }
    } else {
        false // Default is approximate match (TRUE in Excel)
    };

    if exact_match {
        // Search for exact match
        for row in table_array {
            match row {
                Value::Array(row_arr) if !row_arr.is_empty() => {
                    let first_col = &row_arr[0];
                    if values_equal(first_col, lookup_value) {
                        // Found exact match
                        if col_index > row_arr.len() {
                            return Err(PipError::runtime(
                                line,
                                format!(
                                    "VLOOKUP: col_index_num {} exceeds row width {}",
                                    col_index,
                                    row_arr.len()
                                ),
                            ));
                        }
                        return Ok(row_arr[col_index - 1].clone());
                    }
                }
                _ => continue,
            }
        }
    } else {
        // Approximate match: find the largest value that is less than or equal to lookup_value
        // This requires the first column to be sorted in ascending order
        let mut best_match: Option<&Vec<Value>> = None;

        for row in table_array {
            match row {
                Value::Array(row_arr) if !row_arr.is_empty() => {
                    let first_col = &row_arr[0];
                    match compare_values(first_col, lookup_value, line)? {
                        cmp if cmp <= 0 => {
                            // This value is less than or equal to lookup_value
                            best_match = Some(row_arr);
                        }
                        _ => {
                            // We've gone past the lookup value, stop searching
                            break;
                        }
                    }
                }
                _ => continue,
            }
        }

        if let Some(row_arr) = best_match {
            if col_index > row_arr.len() {
                return Err(PipError::runtime(
                    line,
                    format!(
                        "VLOOKUP: col_index_num {} exceeds row width {}",
                        col_index,
                        row_arr.len()
                    ),
                ));
            }
            return Ok(row_arr[col_index - 1].clone());
        }
    }

    // Not found - return #N/A error (Excel convention)
    Ok(Value::String("#N/A".to_string()))
}

/// HLOOKUP(lookup_value, table_array, row_index_num, [range_lookup])
/// Horizontal version of VLOOKUP - searches in the top row and returns from a specified row.
fn hlookup(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() < 3 || args.len() > 4 {
        return Err(PipError::runtime(
            line,
            "HLOOKUP requires 3 or 4 arguments: HLOOKUP(lookup_value, table_array, row_index_num, [range_lookup])",
        ));
    }

    let lookup_value = &args[0];
    let Value::Array(table_array) = &args[1] else {
        return Err(PipError::runtime(
            line,
            "HLOOKUP: table_array must be an array",
        ));
    };

    if table_array.is_empty() {
        return Ok(Value::String("#N/A".to_string()));
    }

    let row_index = match &args[2] {
        Value::Int(n) => {
            if *n < 1 {
                return Err(PipError::runtime(
                    line,
                    "HLOOKUP: row_index_num must be at least 1",
                ));
            }
            *n as usize
        }
        Value::Float(f) => {
            if f.is_nan() || f.is_infinite() || *f < 1.0 {
                return Err(PipError::runtime(
                    line,
                    "HLOOKUP: row_index_num must be a positive number",
                ));
            }
            *f as usize
        }
        _ => {
            return Err(PipError::runtime(
                line,
                "HLOOKUP: row_index_num must be a number",
            ))
        }
    };

    if row_index > table_array.len() {
        return Err(PipError::runtime(
            line,
            format!(
                "HLOOKUP: row_index_num {} exceeds table height {}",
                row_index,
                table_array.len()
            ),
        ));
    }

    let exact_match = if args.len() == 4 {
        match &args[3] {
            Value::Bool(b) => !b,  // FALSE means exact match in Excel
            Value::Int(0) => true, // 0 means exact match
            _ => false,            // Any other value means approximate match
        }
    } else {
        false // Default is approximate match
    };

    // Get the first row for searching
    let Value::Array(first_row) = &table_array[0] else {
        return Err(PipError::runtime(
            line,
            "HLOOKUP: First row must be an array",
        ));
    };

    if exact_match {
        // Search for exact match
        for (col_index, cell) in first_row.iter().enumerate() {
            if values_equal(cell, lookup_value) {
                // Found exact match, return value from specified row
                let Value::Array(target_row) = &table_array[row_index - 1] else {
                    return Err(PipError::runtime(
                        line,
                        "HLOOKUP: Target row must be an array",
                    ));
                };

                if col_index < target_row.len() {
                    return Ok(target_row[col_index].clone());
                }
                return Ok(Value::Null);
            }
        }
    } else {
        // Approximate match: find the largest value that is less than or equal to lookup_value
        // This requires the first row to be sorted in ascending order
        let mut best_match_index: Option<usize> = None;

        for (col_index, cell) in first_row.iter().enumerate() {
            match compare_values(cell, lookup_value, line)? {
                cmp if cmp <= 0 => {
                    // This value is less than or equal to lookup_value
                    best_match_index = Some(col_index);
                }
                _ => {
                    // We've gone past the lookup value, stop searching
                    break;
                }
            }
        }

        if let Some(col_idx) = best_match_index {
            let Value::Array(target_row) = &table_array[row_index - 1] else {
                return Err(PipError::runtime(
                    line,
                    "HLOOKUP: Target row must be an array",
                ));
            };

            if col_idx < target_row.len() {
                return Ok(target_row[col_idx].clone());
            }
            return Ok(Value::Null);
        }
    }

    // Not found
    Ok(Value::String("#N/A".to_string()))
}

/// INDEX(array, row_num, [column_num])
/// Returns the value at the intersection of a particular row and column in a range.
fn index(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() < 2 || args.len() > 3 {
        return Err(PipError::runtime(
            line,
            "INDEX requires 2 or 3 arguments: INDEX(array, row_num, [column_num])",
        ));
    }

    let Value::Array(array) = &args[0] else {
        return Err(PipError::runtime(line, "INDEX: array must be an array"));
    };

    let row_num = match &args[1] {
        Value::Int(n) => *n as usize,
        Value::Float(f) => *f as usize,
        _ => return Err(PipError::runtime(line, "INDEX: row_num must be a number")),
    };

    if row_num == 0 || row_num > array.len() {
        return Err(PipError::runtime(
            line,
            format!(
                "INDEX: row_num {} is out of bounds (array has {} rows)",
                row_num,
                array.len()
            ),
        ));
    }

    let row_data = &array[row_num - 1];

    // If no column number specified, return the entire row (for 1D arrays)
    if args.len() == 2 {
        return Ok(row_data.clone());
    }

    // Column number is specified
    let col_num = match &args[2] {
        Value::Int(n) => *n as usize,
        Value::Float(f) => *f as usize,
        _ => {
            return Err(PipError::runtime(
                line,
                "INDEX: column_num must be a number",
            ))
        }
    };

    if col_num == 0 {
        return Err(PipError::runtime(
            line,
            "INDEX: column_num must be at least 1",
        ));
    }

    // Handle 2D array indexing
    match row_data {
        Value::Array(row_arr) => {
            if col_num > row_arr.len() {
                return Err(PipError::runtime(
                    line,
                    format!(
                        "INDEX: column_num {} is out of bounds (row has {} columns)",
                        col_num,
                        row_arr.len()
                    ),
                ));
            }
            Ok(row_arr[col_num - 1].clone())
        }
        _ => {
            // If the row is not an array but column is specified, it's an error
            if col_num != 1 {
                return Err(PipError::runtime(
                    line,
                    "INDEX: Cannot index column on non-array row",
                ));
            }
            Ok(row_data.clone())
        }
    }
}

/// MATCH(lookup_value, lookup_array, [match_type])
/// Returns the relative position of a value in an array.
fn match_fn(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() < 2 || args.len() > 3 {
        return Err(PipError::runtime(
            line,
            "MATCH requires 2 or 3 arguments: MATCH(lookup_value, lookup_array, [match_type])",
        ));
    }

    let lookup_value = &args[0];
    let Value::Array(lookup_array) = &args[1] else {
        return Err(PipError::runtime(
            line,
            "MATCH: lookup_array must be an array",
        ));
    };

    let match_type = if args.len() == 3 {
        match &args[2] {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            _ => 1, // Default to 1
        }
    } else {
        1 // Default match_type is 1 (less than or equal)
    };

    // Flatten the array if it's 2D
    let flat_array: Vec<&Value> = lookup_array
        .iter()
        .flat_map(|v| match v {
            Value::Array(arr) => arr.iter().collect(),
            _ => vec![v],
        })
        .collect();

    match match_type {
        0 => {
            // Exact match
            for (i, val) in flat_array.iter().enumerate() {
                if values_equal(val, lookup_value) {
                    return Ok(Value::Int((i + 1) as i64)); // 1-based index
                }
            }
            Ok(Value::String("#N/A".to_string()))
        }
        1 => {
            // Less than or equal (array must be in ascending order)
            let mut last_valid_index = None;
            for (i, val) in flat_array.iter().enumerate() {
                match compare_values(val, lookup_value, line)? {
                    cmp if cmp <= 0 => last_valid_index = Some(i + 1),
                    _ => break,
                }
            }
            match last_valid_index {
                Some(idx) => Ok(Value::Int(idx as i64)),
                None => Ok(Value::String("#N/A".to_string())),
            }
        }
        -1 => {
            // Greater than or equal (array must be in descending order)
            for (i, val) in flat_array.iter().enumerate() {
                if compare_values(val, lookup_value, line)? >= 0 {
                    return Ok(Value::Int((i + 1) as i64));
                }
            }
            Ok(Value::String("#N/A".to_string()))
        }
        _ => Err(PipError::runtime(
            line,
            "MATCH: match_type must be -1, 0, or 1",
        )),
    }
}

/// XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])
/// Modern replacement for VLOOKUP/HLOOKUP with more flexibility.
fn xlookup(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() < 3 || args.len() > 6 {
        return Err(PipError::runtime(
            line,
            "XLOOKUP requires 3-6 arguments: XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])",
        ));
    }

    let lookup_value = &args[0];

    let Value::Array(lookup_array) = &args[1] else {
        return Err(PipError::runtime(
            line,
            "XLOOKUP: lookup_array must be an array",
        ));
    };

    let Value::Array(return_array) = &args[2] else {
        return Err(PipError::runtime(
            line,
            "XLOOKUP: return_array must be an array",
        ));
    };

    // Flatten arrays if they're 2D
    let flat_lookup: Vec<&Value> = lookup_array
        .iter()
        .flat_map(|v| match v {
            Value::Array(arr) => arr.iter().collect(),
            _ => vec![v],
        })
        .collect();

    let flat_return: Vec<&Value> = return_array
        .iter()
        .flat_map(|v| match v {
            Value::Array(arr) => arr.iter().collect(),
            _ => vec![v],
        })
        .collect();

    if flat_lookup.len() != flat_return.len() {
        return Err(PipError::runtime(
            line,
            format!(
                "XLOOKUP: lookup_array and return_array must have the same length ({} vs {})",
                flat_lookup.len(),
                flat_return.len()
            ),
        ));
    }

    let if_not_found = if args.len() > 3 {
        args[3].clone()
    } else {
        Value::String("#N/A".to_string())
    };

    let match_mode = if args.len() > 4 {
        match &args[4] {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            _ => 0, // Default to exact match
        }
    } else {
        0 // Default to exact match
    };

    let search_mode = if args.len() > 5 {
        match &args[5] {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            _ => 1, // Default to first-to-last
        }
    } else {
        1 // Default to first-to-last
    };

    // Determine search order
    let search_iter: Box<dyn Iterator<Item = (usize, &Value)>> = match search_mode {
        -1 | -2 => Box::new(flat_lookup.iter().enumerate().rev().map(|(i, v)| (i, *v))),
        _ => Box::new(flat_lookup.iter().enumerate().map(|(i, v)| (i, *v))),
    };

    // Search based on match mode
    match match_mode {
        0 => {
            // Exact match
            for (i, val) in search_iter {
                if values_equal(val, lookup_value) {
                    return Ok(flat_return[i].clone());
                }
            }
        }
        -1 => {
            // Exact match or next smallest
            let mut best_match: Option<(usize, &Value)> = None;
            for (i, val) in flat_lookup.iter().enumerate().map(|(i, v)| (i, *v)) {
                if values_equal(val, lookup_value) {
                    return Ok(flat_return[i].clone());
                } else if compare_values(val, lookup_value, line)? < 0 {
                    best_match = Some((i, val));
                }
            }
            if let Some((i, _)) = best_match {
                return Ok(flat_return[i].clone());
            }
        }
        1 => {
            // Exact match or next largest
            let mut best_match: Option<(usize, &Value)> = None;
            for (i, val) in flat_lookup.iter().enumerate().map(|(i, v)| (i, *v)) {
                if values_equal(val, lookup_value) {
                    return Ok(flat_return[i].clone());
                } else if compare_values(val, lookup_value, line)? > 0 && best_match.is_none() {
                    best_match = Some((i, val));
                }
            }
            if let Some((i, _)) = best_match {
                return Ok(flat_return[i].clone());
            }
        }
        2 => {
            // Wildcard match
            // TODO: Implement wildcard matching
            return Err(PipError::runtime(
                line,
                "XLOOKUP: Wildcard match mode not yet implemented",
            ));
        }
        _ => {
            return Err(PipError::runtime(
                line,
                "XLOOKUP: match_mode must be -1, 0, 1, or 2",
            ));
        }
    }

    Ok(if_not_found)
}

/// Helper function to check if two values are equal
fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
        (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
            (*a as f64 - b).abs() < f64::EPSILON
        }
        (Value::String(a), Value::String(b)) => a == b,
        _ => false,
    }
}

/// Helper function to compare two values
/// Returns -1 if left < right, 0 if equal, 1 if left > right
fn compare_values(left: &Value, right: &Value, line: usize) -> PipResult<i32> {
    match (left, right) {
        (Value::Null, Value::Null) => Ok(0),
        (Value::Null, _) => Ok(-1),
        (_, Value::Null) => Ok(1),
        (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b) as i32),
        (Value::Float(a), Value::Float(b)) => Ok(if (a - b).abs() < f64::EPSILON {
            0
        } else if a < b {
            -1
        } else {
            1
        }),
        (Value::Int(a), Value::Float(b)) => {
            let a_f = *a as f64;
            Ok(if (a_f - b).abs() < f64::EPSILON {
                0
            } else if a_f < *b {
                -1
            } else {
                1
            })
        }
        (Value::Float(a), Value::Int(b)) => {
            let b_f = *b as f64;
            Ok(if (a - b_f).abs() < f64::EPSILON {
                0
            } else if *a < b_f {
                -1
            } else {
                1
            })
        }
        (Value::String(a), Value::String(b)) => Ok(a.cmp(b) as i32),
        _ => Err(PipError::runtime(
            line,
            format!(
                "Cannot compare {} with {}",
                left.type_name(),
                right.type_name()
            ),
        )),
    }
}
