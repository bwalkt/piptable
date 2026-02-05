//! Book-related built-in functions.

use crate::book_conversions::{
    book_to_value_dict, consolidate_options_from_value, file_load_options_from_value,
    value_to_sheet_for_book,
};
use crate::Interpreter;
use piptable_core::{PipError, PipResult, Value};
use piptable_sheet::Book;

fn expect_book(value: &Value, line: usize) -> PipResult<&Book> {
    match value {
        Value::Book(book) => Ok(book),
        _ => Err(PipError::runtime(line, "First argument must be a Book")),
    }
}

/// Execute a book built-in function.
pub async fn call_book_builtin(
    _interpreter: &Interpreter,
    name: &str,
    args: Vec<Value>,
    line: usize,
) -> Option<PipResult<Value>> {
    match name {
        "book_sheet_names" => Some(book_sheet_names(args, line)),
        "book_sheet_count" => Some(book_sheet_count(args, line)),
        "book_has_sheet" => Some(book_has_sheet(args, line)),
        "book_get_sheet" => Some(book_get_sheet(args, line)),
        "book_get_sheet_by_index" => Some(book_get_sheet_by_index(args, line)),
        "book_active_sheet" => Some(book_active_sheet(args, line)),
        "book_set_active_sheet" => Some(book_set_active_sheet(args, line)),
        "book_add_sheet" => Some(book_add_sheet(args, line)),
        "book_remove_sheet" => Some(book_remove_sheet(args, line)),
        "book_rename_sheet" => Some(book_rename_sheet(args, line)),
        "book_merge" => Some(book_merge(args, line)),
        "book_to_dict" => Some(book_to_dict(args, line)),
        "book_from_dict" => Some(book_from_dict(args, line)),
        "book_sheets" => Some(book_sheets(args, line)),
        "book_add_empty_sheet" => Some(book_add_empty_sheet(args, line)),
        "book_consolidate" => Some(book_consolidate(args, line)),
        "book_consolidate_with_options" => Some(book_consolidate_with_options(args, line)),
        "book_from_files" => Some(book_from_files(args, line)),
        "book_from_files_with_options" => Some(book_from_files_with_options(args, line)),
        _ => None,
    }
}

fn book_sheet_names(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 1 {
        return Err(PipError::runtime(
            line,
            "book_sheet_names() takes exactly 1 argument (book)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let names = book
        .sheet_names()
        .into_iter()
        .map(|name| Value::String(name.to_string()))
        .collect();
    Ok(Value::Array(names))
}

fn book_sheet_count(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 1 {
        return Err(PipError::runtime(
            line,
            "book_sheet_count() takes exactly 1 argument (book)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    Ok(Value::Int(book.sheet_count() as i64))
}

fn book_has_sheet(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 2 {
        return Err(PipError::runtime(
            line,
            "book_has_sheet() takes exactly 2 arguments (book, name)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let name = args[1]
        .as_str()
        .ok_or_else(|| PipError::runtime(line, "Sheet name must be a string"))?;
    Ok(Value::Bool(book.has_sheet(name)))
}

fn book_get_sheet(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 2 {
        return Err(PipError::runtime(
            line,
            "book_get_sheet() takes exactly 2 arguments (book, name)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let name = args[1]
        .as_str()
        .ok_or_else(|| PipError::runtime(line, "Sheet name must be a string"))?;
    let sheet = book
        .get_sheet(name)
        .map_err(|e| PipError::runtime(line, format!("Failed to get sheet: {}", e)))?;
    Ok(Value::Sheet(Box::new(sheet.clone())))
}

fn book_get_sheet_by_index(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 2 {
        return Err(PipError::runtime(
            line,
            "book_get_sheet_by_index() takes exactly 2 arguments (book, index)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let index = args[1]
        .as_int()
        .ok_or_else(|| PipError::runtime(line, "Index must be an integer"))?;
    let index_usize = if index < 0 {
        let adjusted = book.sheet_count() as i64 + index;
        if adjusted < 0 {
            return Err(PipError::runtime(line, "Book index out of bounds"));
        }
        usize::try_from(adjusted)
            .map_err(|_| PipError::runtime(line, "Index out of range for usize"))?
    } else {
        usize::try_from(index).map_err(|_| PipError::runtime(line, "Index is too large"))?
    };
    let sheet = book
        .get_sheet_by_index(index_usize)
        .map_err(|e| PipError::runtime(line, format!("Failed to get sheet by index: {}", e)))?;
    Ok(Value::Sheet(Box::new(sheet.clone())))
}

fn book_active_sheet(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 1 {
        return Err(PipError::runtime(
            line,
            "book_active_sheet() takes exactly 1 argument (book)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    match book.active_sheet() {
        Some(sheet) => Ok(Value::Sheet(Box::new(sheet.clone()))),
        None => Ok(Value::Null),
    }
}

fn book_set_active_sheet(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 2 {
        return Err(PipError::runtime(
            line,
            "book_set_active_sheet() takes exactly 2 arguments (book, name)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let name = args[1]
        .as_str()
        .ok_or_else(|| PipError::runtime(line, "Sheet name must be a string"))?;
    let mut new_book = book.clone();
    new_book
        .set_active_sheet(name)
        .map_err(|e| PipError::runtime(line, format!("Failed to set active sheet: {}", e)))?;
    Ok(Value::Book(Box::new(new_book)))
}

fn book_add_sheet(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 3 {
        return Err(PipError::runtime(
            line,
            "book_add_sheet() takes exactly 3 arguments (book, name, sheet)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let name = args[1]
        .as_str()
        .ok_or_else(|| PipError::runtime(line, "Sheet name must be a string"))?;
    let sheet = value_to_sheet_for_book(&args[2])
        .map_err(|e| PipError::runtime(line, format!("Invalid sheet data: {}", e)))?;
    let mut new_book = book.clone();
    new_book
        .add_sheet(name, sheet)
        .map_err(|e| PipError::runtime(line, format!("Failed to add sheet: {}", e)))?;
    Ok(Value::Book(Box::new(new_book)))
}

fn book_add_empty_sheet(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 2 {
        return Err(PipError::runtime(
            line,
            "book_add_empty_sheet() takes exactly 2 arguments (book, name)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let name = args[1]
        .as_str()
        .ok_or_else(|| PipError::runtime(line, "Sheet name must be a string"))?;
    let mut new_book = book.clone();
    new_book
        .add_empty_sheet(name)
        .map_err(|e| PipError::runtime(line, format!("Failed to add sheet: {}", e)))?;
    Ok(Value::Book(Box::new(new_book)))
}

fn book_remove_sheet(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 2 {
        return Err(PipError::runtime(
            line,
            "book_remove_sheet() takes exactly 2 arguments (book, name)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let name = args[1]
        .as_str()
        .ok_or_else(|| PipError::runtime(line, "Sheet name must be a string"))?;
    let mut new_book = book.clone();
    new_book
        .remove_sheet(name)
        .map_err(|e| PipError::runtime(line, format!("Failed to remove sheet: {}", e)))?;
    Ok(Value::Book(Box::new(new_book)))
}

fn book_rename_sheet(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 3 {
        return Err(PipError::runtime(
            line,
            "book_rename_sheet() takes exactly 3 arguments (book, old_name, new_name)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let old_name = args[1]
        .as_str()
        .ok_or_else(|| PipError::runtime(line, "Old name must be a string"))?;
    let new_name = args[2]
        .as_str()
        .ok_or_else(|| PipError::runtime(line, "New name must be a string"))?;
    let mut new_book = book.clone();
    new_book
        .rename_sheet(old_name, new_name)
        .map_err(|e| PipError::runtime(line, format!("Failed to rename sheet: {}", e)))?;
    Ok(Value::Book(Box::new(new_book)))
}

fn book_merge(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 2 {
        return Err(PipError::runtime(
            line,
            "book_merge() takes exactly 2 arguments (book, other_book)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let Value::Book(other) = &args[1] else {
        return Err(PipError::runtime(
            line,
            "book_merge() requires a Book as the second argument",
        ));
    };
    let mut new_book = book.clone();
    new_book.merge((**other).clone());
    Ok(Value::Book(Box::new(new_book)))
}

fn book_to_dict(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 1 {
        return Err(PipError::runtime(
            line,
            "book_to_dict() takes exactly 1 argument (book)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    Ok(book_to_value_dict(book))
}

fn book_from_dict(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 1 {
        return Err(PipError::runtime(
            line,
            "book_from_dict() takes exactly 1 argument (sheet_map)",
        ));
    }

    let Value::Object(map) = &args[0] else {
        return Err(PipError::runtime(
            line,
            "book_from_dict() requires an object of sheet_name -> data",
        ));
    };

    let mut book = Book::new();
    for (name, value) in map {
        let sheet = value_to_sheet_for_book(value)
            .map_err(|e| PipError::runtime(line, format!("Invalid sheet data: {}", e)))?;
        book.add_sheet(name, sheet)
            .map_err(|e| PipError::runtime(line, format!("Failed to add sheet: {}", e)))?;
    }

    Ok(Value::Book(Box::new(book)))
}

fn book_sheets(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 1 {
        return Err(PipError::runtime(
            line,
            "book_sheets() takes exactly 1 argument (book)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let sheets = book
        .sheets()
        .map(|(_, sheet)| Value::Sheet(Box::new(sheet.clone())))
        .collect();
    Ok(Value::Array(sheets))
}

fn book_consolidate(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 1 {
        return Err(PipError::runtime(
            line,
            "book_consolidate() takes exactly 1 argument (book)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let sheet = book
        .consolidate()
        .map_err(|e| PipError::runtime(line, format!("Failed to consolidate: {}", e)))?;
    Ok(Value::Sheet(Box::new(sheet)))
}

fn book_consolidate_with_options(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 2 {
        return Err(PipError::runtime(
            line,
            "book_consolidate_with_options() takes exactly 2 arguments (book, options)",
        ));
    }
    let book = expect_book(&args[0], line)?;
    let options = consolidate_options_from_value(Some(&args[1]), line)?;
    let sheet = book
        .consolidate_with_options(options)
        .map_err(|e| PipError::runtime(line, format!("Failed to consolidate: {}", e)))?;
    Ok(Value::Sheet(Box::new(sheet)))
}

fn book_from_files(args: Vec<Value>, line: usize) -> PipResult<Value> {
    book_from_files_impl("book_from_files", args, line, None)
}

fn book_from_files_with_options(args: Vec<Value>, line: usize) -> PipResult<Value> {
    if args.len() != 2 {
        return Err(PipError::runtime(
            line,
            "book_from_files_with_options() takes exactly 2 arguments (paths, options)",
        ));
    }
    book_from_files_impl(
        "book_from_files_with_options",
        vec![args[0].clone()],
        line,
        Some(&args[1]),
    )
}

fn book_from_files_impl(
    func_name: &str,
    args: Vec<Value>,
    line: usize,
    options: Option<&Value>,
) -> PipResult<Value> {
    if args.len() != 1 {
        return Err(PipError::runtime(
            line,
            format!("{func_name}() takes exactly 1 argument (paths)"),
        ));
    }

    if cfg!(target_arch = "wasm32") {
        return Err(PipError::runtime(
            line,
            format!("{func_name}() is not supported in the playground"),
        ));
    }

    let paths = match &args[0] {
        Value::Array(items) => items
            .iter()
            .map(|v| {
                v.as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| PipError::runtime(line, "Paths must be strings"))
            })
            .collect::<PipResult<Vec<_>>>()?,
        _ => {
            return Err(PipError::runtime(
                line,
                format!("{func_name}() requires an array of paths"),
            ))
        }
    };

    let opts = file_load_options_from_value(options, line)?;
    let book = piptable_sheet::Book::from_files_with_options(&paths, opts)
        .map_err(|e| PipError::runtime(line, format!("Failed to load book from files: {}", e)))?;
    Ok(Value::Book(Box::new(book)))
}
