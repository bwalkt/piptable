use regex::Regex;

use piptable_primitives::CellAddress;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceKind {
    Cell,
    Range,
    ColumnRange,
    RowRange,
    R1C1Cell,
    R1C1Range,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceMode {
    Relative,
    Absolute,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaReference {
    pub text: String,
    pub sheet: Option<String>,
    pub kind: ReferenceKind,
    pub mode: ReferenceMode,
}

#[derive(Debug, Clone, Default)]
pub struct FormulaToRelativeReferenceOptions {
    /// TODO: implement exclusion range and circular handling.
    pub exclusion_range: Option<(CellAddress, CellAddress)>,
    pub ignore_circular: bool,
}

#[derive(Debug, Clone, Copy)]
struct A1Cell {
    row: i32,
    col: i32,
    abs_row: bool,
    abs_col: bool,
}

#[derive(Debug, Clone, Copy)]
struct A1Column {
    col: i32,
    abs_col: bool,
}

#[derive(Debug, Clone, Copy)]
struct A1Row {
    row: i32,
    abs_row: bool,
}

#[derive(Debug, Clone, Copy)]
enum A1Ref {
    Cell(A1Cell),
    Column(A1Column),
    Row(A1Row),
}

pub fn formula_to_relative_reference(
    formula: &str,
    source_cell: CellAddress,
    destination_cell: CellAddress,
    _options: Option<FormulaToRelativeReferenceOptions>,
) -> String {
    if formula.is_empty() {
        return formula.to_string();
    }

    if source_cell.row == destination_cell.row && source_cell.col == destination_cell.col {
        return formula.to_string();
    }

    let d_row = destination_cell.row as i32 - source_cell.row as i32;
    let d_col = destination_cell.col as i32 - source_cell.col as i32;

    let mut updated = replace_cell_refs(formula, d_row, d_col);
    updated = replace_column_ranges(&updated, d_col);
    updated = replace_row_ranges(&updated, d_row);
    updated
}

pub fn extract_references(formula: &str) -> Vec<FormulaReference> {
    let mut matches: Vec<(usize, FormulaReference)> = Vec::new();

    for cap in cell_regex()
        .captures_iter(formula)
        .filter_map(|cap| cap.get(0).map(|m| (m.start(), cap)))
    {
        let start = cap.0;
        let cap = cap.1;
        let sheet = cap
            .get(1)
            .map(|m| m.as_str().trim_end_matches('!').to_string());
        let left = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let right = cap.get(3).map(|m| m.as_str());

        let (kind, mode) = if right.is_some() {
            let left_ref = parse_a1_ref(left);
            let right_ref = right.and_then(parse_a1_ref);
            (ReferenceKind::Range, combine_modes(left_ref, right_ref))
        } else {
            let left_ref = parse_a1_ref(left);
            (ReferenceKind::Cell, mode_from_ref(left_ref))
        };

        matches.push((
            start,
            FormulaReference {
                text: cap.get(0).unwrap().as_str().to_string(),
                sheet,
                kind,
                mode,
            },
        ));
    }

    for cap in column_range_regex()
        .captures_iter(formula)
        .filter_map(|cap| cap.get(0).map(|m| (m.start(), cap)))
    {
        let start = cap.0;
        let cap = cap.1;
        let sheet = cap
            .get(1)
            .map(|m| m.as_str().trim_end_matches('!').to_string());
        let left = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let right = cap.get(3).map(|m| m.as_str()).unwrap_or("");

        let left_ref = parse_a1_ref(left);
        let right_ref = parse_a1_ref(right);

        matches.push((
            start,
            FormulaReference {
                text: cap.get(0).unwrap().as_str().to_string(),
                sheet,
                kind: ReferenceKind::ColumnRange,
                mode: combine_modes(left_ref, right_ref),
            },
        ));
    }

    for cap in row_range_regex()
        .captures_iter(formula)
        .filter_map(|cap| cap.get(0).map(|m| (m.start(), cap)))
    {
        let start = cap.0;
        let cap = cap.1;
        let sheet = cap
            .get(1)
            .map(|m| m.as_str().trim_end_matches('!').to_string());
        let left = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let right = cap.get(3).map(|m| m.as_str()).unwrap_or("");

        let left_ref = parse_a1_ref(left);
        let right_ref = parse_a1_ref(right);

        matches.push((
            start,
            FormulaReference {
                text: cap.get(0).unwrap().as_str().to_string(),
                sheet,
                kind: ReferenceKind::RowRange,
                mode: combine_modes(left_ref, right_ref),
            },
        ));
    }

    for cap in r1c1_regex()
        .captures_iter(formula)
        .filter_map(|cap| cap.get(0).map(|m| (m.start(), cap)))
    {
        let start = cap.0;
        let cap = cap.1;
        let sheet = cap
            .get(1)
            .map(|m| m.as_str().trim_end_matches('!').to_string());
        let left = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let right = cap.get(3).map(|m| m.as_str());

        let left_mode = r1c1_mode(left);
        let mode = if let Some(r) = right {
            let right_mode = r1c1_mode(r);
            match (left_mode, right_mode) {
                (ReferenceMode::Absolute, ReferenceMode::Absolute) => ReferenceMode::Absolute,
                (ReferenceMode::Relative, ReferenceMode::Relative) => ReferenceMode::Relative,
                _ => ReferenceMode::Mixed,
            }
        } else {
            left_mode
        };

        let kind = if right.is_some() {
            ReferenceKind::R1C1Range
        } else {
            ReferenceKind::R1C1Cell
        };

        matches.push((
            start,
            FormulaReference {
                text: cap.get(0).unwrap().as_str().to_string(),
                sheet,
                kind,
                mode,
            },
        ));
    }

    matches.sort_by(|a, b| a.0.cmp(&b.0));
    matches.into_iter().map(|(_, m)| m).collect()
}

fn replace_cell_refs(formula: &str, d_row: i32, d_col: i32) -> String {
    cell_regex()
        .replace_all(formula, |caps: &regex::Captures| {
            let sheet_prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let left = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let right = caps.get(3).map(|m| m.as_str());

            let adjust_one = |addr: &str| -> String {
                match parse_a1_ref(addr) {
                    Some(A1Ref::Cell(cell)) => format_cell(adjust_cell(cell, d_row, d_col)),
                    _ => addr.to_string(),
                }
            };

            if let Some(right) = right {
                let left_adj = adjust_one(left);
                let right_adj = adjust_one(right);
                format!("{}{}:{}", sheet_prefix, left_adj, right_adj)
            } else {
                let left_adj = adjust_one(left);
                format!("{}{}", sheet_prefix, left_adj)
            }
        })
        .to_string()
}

fn replace_column_ranges(formula: &str, d_col: i32) -> String {
    column_range_regex()
        .replace_all(formula, |caps: &regex::Captures| {
            let sheet_prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let left = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let right = caps.get(3).map(|m| m.as_str()).unwrap_or("");

            let left_ref = parse_a1_ref(left);
            let right_ref = parse_a1_ref(right);

            let left_adj = match left_ref {
                Some(A1Ref::Column(col)) => format_column(adjust_column(col, d_col)),
                _ => left.to_string(),
            };
            let right_adj = match right_ref {
                Some(A1Ref::Column(col)) => format_column(adjust_column(col, d_col)),
                _ => right.to_string(),
            };

            format!("{}{}:{}", sheet_prefix, left_adj, right_adj)
        })
        .to_string()
}

fn replace_row_ranges(formula: &str, d_row: i32) -> String {
    row_range_regex()
        .replace_all(formula, |caps: &regex::Captures| {
            let sheet_prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let left = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let right = caps.get(3).map(|m| m.as_str()).unwrap_or("");

            let left_ref = parse_a1_ref(left);
            let right_ref = parse_a1_ref(right);

            let left_adj = match left_ref {
                Some(A1Ref::Row(row)) => format_row(adjust_row(row, d_row)),
                _ => left.to_string(),
            };
            let right_adj = match right_ref {
                Some(A1Ref::Row(row)) => format_row(adjust_row(row, d_row)),
                _ => right.to_string(),
            };

            format!("{}{}:{}", sheet_prefix, left_adj, right_adj)
        })
        .to_string()
}

fn adjust_cell(cell: A1Cell, d_row: i32, d_col: i32) -> A1Cell {
    let row = if cell.abs_row {
        cell.row
    } else {
        cell.row + d_row
    };
    let col = if cell.abs_col {
        cell.col
    } else {
        cell.col + d_col
    };

    A1Cell {
        row: row.max(1),
        col: col.max(1),
        ..cell
    }
}

fn adjust_column(col: A1Column, d_col: i32) -> A1Column {
    let new_col = if col.abs_col {
        col.col
    } else {
        col.col + d_col
    };
    A1Column {
        col: new_col.max(1),
        ..col
    }
}

fn adjust_row(row: A1Row, d_row: i32) -> A1Row {
    let new_row = if row.abs_row {
        row.row
    } else {
        row.row + d_row
    };
    A1Row {
        row: new_row.max(1),
        ..row
    }
}

fn format_cell(cell: A1Cell) -> String {
    let col_letters = number_to_letters(cell.col);
    format!(
        "{}{}{}{}",
        if cell.abs_col { "$" } else { "" },
        col_letters,
        if cell.abs_row { "$" } else { "" },
        cell.row
    )
}

fn format_column(col: A1Column) -> String {
    let col_letters = number_to_letters(col.col);
    format!("{}{}", if col.abs_col { "$" } else { "" }, col_letters)
}

fn format_row(row: A1Row) -> String {
    format!("{}{}", if row.abs_row { "$" } else { "" }, row.row)
}

fn parse_a1_ref(text: &str) -> Option<A1Ref> {
    if let Some(cell) = parse_a1_cell(text) {
        return Some(A1Ref::Cell(cell));
    }
    if let Some(col) = parse_a1_column(text) {
        return Some(A1Ref::Column(col));
    }
    parse_a1_row(text).map(A1Ref::Row)
}

fn parse_a1_cell(text: &str) -> Option<A1Cell> {
    let caps = a1_cell_regex().captures(text)?;
    let abs_col = caps.get(1).map(|m| m.as_str()) == Some("$");
    let abs_row = caps.get(3).map(|m| m.as_str()) == Some("$");
    let col = letters_to_number(caps.get(2)?.as_str())?;
    let row = caps.get(4)?.as_str().parse::<i32>().ok()?;
    if row < 1 {
        return None;
    }
    Some(A1Cell {
        row,
        col,
        abs_row,
        abs_col,
    })
}

fn parse_a1_column(text: &str) -> Option<A1Column> {
    let caps = a1_column_regex().captures(text)?;
    let abs_col = caps.get(1).map(|m| m.as_str()) == Some("$");
    let col = letters_to_number(caps.get(2)?.as_str())?;
    Some(A1Column { col, abs_col })
}

fn parse_a1_row(text: &str) -> Option<A1Row> {
    let caps = a1_row_regex().captures(text)?;
    let abs_row = caps.get(1).map(|m| m.as_str()) == Some("$");
    let row = caps.get(2)?.as_str().parse::<i32>().ok()?;
    if row < 1 {
        return None;
    }
    Some(A1Row { row, abs_row })
}

fn letters_to_number(letters: &str) -> Option<i32> {
    let mut result: i32 = 0;
    for ch in letters.chars() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        let value = (ch.to_ascii_uppercase() as u8 - b'A' + 1) as i32;
        result = result * 26 + value;
    }
    Some(result)
}

fn number_to_letters(mut number: i32) -> String {
    let mut result = String::new();
    if number < 1 {
        return "A".to_string();
    }
    while number > 0 {
        let mut remainder = number % 26;
        if remainder == 0 {
            remainder = 26;
            number = (number / 26) - 1;
        } else {
            number /= 26;
        }
        result.push((b'A' + (remainder - 1) as u8) as char);
    }
    result.chars().rev().collect()
}

fn mode_from_ref(reference: Option<A1Ref>) -> ReferenceMode {
    match reference {
        Some(A1Ref::Cell(cell)) => match (cell.abs_col, cell.abs_row) {
            (true, true) => ReferenceMode::Absolute,
            (false, false) => ReferenceMode::Relative,
            _ => ReferenceMode::Mixed,
        },
        Some(A1Ref::Column(col)) => {
            if col.abs_col {
                ReferenceMode::Absolute
            } else {
                ReferenceMode::Relative
            }
        }
        Some(A1Ref::Row(row)) => {
            if row.abs_row {
                ReferenceMode::Absolute
            } else {
                ReferenceMode::Relative
            }
        }
        None => ReferenceMode::Relative,
    }
}

fn combine_modes(left: Option<A1Ref>, right: Option<A1Ref>) -> ReferenceMode {
    match (mode_from_ref(left), mode_from_ref(right)) {
        (ReferenceMode::Absolute, ReferenceMode::Absolute) => ReferenceMode::Absolute,
        (ReferenceMode::Relative, ReferenceMode::Relative) => ReferenceMode::Relative,
        _ => ReferenceMode::Mixed,
    }
}

fn r1c1_mode(text: &str) -> ReferenceMode {
    let upper = text.to_ascii_uppercase();
    let mut abs = 0;
    let mut rel = 0;
    if upper.contains('R') {
        if upper.contains("R[") || upper == "RC" || upper == "R" {
            rel += 1;
        } else {
            abs += 1;
        }
    }
    if upper.contains('C') {
        if upper.contains("C[") || upper == "RC" || upper == "C" {
            rel += 1;
        } else {
            abs += 1;
        }
    }
    match (abs > 0, rel > 0) {
        (true, true) => ReferenceMode::Mixed,
        (true, false) => ReferenceMode::Absolute,
        (false, true) => ReferenceMode::Relative,
        _ => ReferenceMode::Relative,
    }
}

fn cell_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"((?:'[^']+'|[^'!,]+)!)?(\$?[A-Za-z]+\$?\d+)(?::(\$?[A-Za-z]+\$?\d+))?")
            .expect("valid regex")
    })
}

fn column_range_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"((?:'[^']+'|[^'!,]+)!)?(\$?[A-Za-z]+):(\$?[A-Za-z]+)").expect("valid regex")
    })
}

fn row_range_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"((?:'[^']+'|[^'!,]+)!)?(\$?\d+):(\$?\d+)").expect("valid regex"))
}

fn r1c1_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"((?:'[^']+'|[^'!,]+)!)?(R(?:\[[-+]?\d+\]|\d+)?C(?:\[[-+]?\d+\]|\d+)?)(?::(R(?:\[[-+]?\d+\]|\d+)?C(?:\[[-+]?\d+\]|\d+)?))?")
            .expect("valid regex")
    })
}

fn a1_cell_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\$?)([A-Za-z]+)(\$?)(\d+)$").expect("valid regex"))
}

fn a1_column_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\$?)([A-Za-z]+)$").expect("valid regex"))
}

fn a1_row_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\$?)(\d+)$").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formula_to_relative_reference_cells() {
        let source = CellAddress::new(0, 0);
        let dest = CellAddress::new(1, 1);
        let out = formula_to_relative_reference("=A1+B1", source, dest, None);
        assert_eq!(out, "=B2+C2");
    }

    #[test]
    fn test_formula_to_relative_reference_absolute() {
        let source = CellAddress::new(0, 0);
        let dest = CellAddress::new(2, 0);
        let out = formula_to_relative_reference("=$A$1+A1", source, dest, None);
        assert_eq!(out, "=$A$1+A3");
    }

    #[test]
    fn test_extract_references() {
        let refs = extract_references("SUM(A1:B2, Sheet1!$C$3)");
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].kind, ReferenceKind::Range);
        assert_eq!(refs[1].mode, ReferenceMode::Absolute);
    }
}
