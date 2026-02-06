use piptable_primitives::address::{
    address_to_cell, cell_range_to_address, cell_to_address, convert_cell_to_range,
    desanitize_sheet_name, escape_characters, sanitize_sheet_name, selection_bounds_from_cells,
    supplant, AddressPosition, AddressToCellOptions,
};
use piptable_primitives::{CellAddress, CellRange};
use std::collections::HashMap;

#[test]
fn test_address_to_cell_basic() {
    let cell = address_to_cell("B2", AddressToCellOptions::default()).expect("cell");
    assert_eq!(cell, CellAddress::new(1, 1));
}

#[test]
fn test_address_to_cell_with_sheet_prefix() {
    let cell = address_to_cell("Sheet1!C3", AddressToCellOptions::default()).expect("cell");
    assert_eq!(cell, CellAddress::new(2, 2));
}

#[test]
fn test_address_to_cell_row_only_and_column_only() {
    let opts = AddressToCellOptions {
        position: AddressPosition::End,
        row_count: Some(10),
        column_count: Some(5),
    };

    let col_only = address_to_cell("D", opts).expect("column only");
    assert_eq!(col_only, CellAddress::new(9, 3));

    let row_only = address_to_cell(
        "4",
        AddressToCellOptions {
            position: AddressPosition::Start,
            row_count: Some(10),
            column_count: Some(5),
        },
    )
    .expect("row only");
    assert_eq!(row_only, CellAddress::new(3, 0));
}

#[test]
fn test_cell_to_address_and_range() {
    let cell = CellAddress::new(0, 0);
    assert_eq!(
        cell_to_address(Some(cell), false, false, false, false).unwrap(),
        "A1"
    );

    let range = convert_cell_to_range(CellAddress::new(1, 2));
    assert_eq!(range.start, CellAddress::new(1, 2));
}

#[test]
fn test_cell_range_to_address_with_sheet_name() {
    let range = CellRange::new(CellAddress::new(0, 0), CellAddress::new(1, 1));
    let address = cell_range_to_address(Some("Sales Data"), range);
    assert_eq!(address, "'Sales Data'!A1:B2");
}

#[test]
fn test_sanitize_and_desanitize_sheet_names() {
    let name = sanitize_sheet_name(Some("Sheet 1")).expect("sanitize");
    assert_eq!(name, "'Sheet 1'");
    let desanitized = desanitize_sheet_name(Some(&name)).expect("desanitize");
    assert_eq!(desanitized, "Sheet 1");
}

#[test]
fn test_escape_characters() {
    let input = r".*?+[]()";
    let escaped = escape_characters(input);
    assert_eq!(escaped, r"\.\*\?\+\[\]\(\)");
}

#[test]
fn test_selection_bounds() {
    let start = CellAddress::new(5, 2);
    let end = CellAddress::new(3, 4);
    let range = selection_bounds_from_cells(start, Some(end));
    assert_eq!(range.start, CellAddress::new(3, 2));
    assert_eq!(range.end, CellAddress::new(5, 4));
}

#[test]
fn test_supplant_placeholders() {
    let mut values = HashMap::new();
    values.insert("name".to_string(), "Ada".to_string());
    let out = supplant("Hello {name}!", &values);
    assert_eq!(out, "Hello Ada!");

    let out = supplant("Unclosed {name", &values);
    assert_eq!(out, "Unclosed {name");
}
