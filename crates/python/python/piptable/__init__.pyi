"""Type stubs for piptable Python bindings."""

from typing import Any, Optional

class Sheet:
    """A sheet representing a 2D grid of cells."""

    def __init__(self) -> None:
        """Create a new empty sheet."""
        ...

    @staticmethod
    def from_data(data: list[list[Any]]) -> Sheet:
        """Create a sheet from a 2D list of values."""
        ...

    @staticmethod
    def from_csv(
        path: str,
        has_headers: bool = False,
        delimiter: Optional[str] = None,
    ) -> Sheet:
        """Load a sheet from a CSV file."""
        ...

    @staticmethod
    def from_xlsx(path: str, has_headers: bool = False) -> Sheet:
        """Load a sheet from an Excel file (first sheet)."""
        ...

    @staticmethod
    def from_xlsx_sheet(
        path: str,
        sheet_name: str,
        has_headers: bool = False,
    ) -> Sheet:
        """Load a specific sheet from an Excel file by name."""
        ...

    def name(self) -> str:
        """Get the sheet name."""
        ...

    def set_name(self, name: str) -> None:
        """Set the sheet name."""
        ...

    def row_count(self) -> int:
        """Get the number of rows."""
        ...

    def col_count(self) -> int:
        """Get the number of columns."""
        ...

    def is_empty(self) -> bool:
        """Check if the sheet is empty."""
        ...

    def get(self, row: int, col: int) -> Any:
        """Get a cell value by row and column index (0-based)."""
        ...

    def set(self, row: int, col: int, value: Any) -> None:
        """Set a cell value by row and column index (0-based)."""
        ...

    def get_by_name(self, row: int, col_name: str) -> Any:
        """Get a cell value by row index and column name."""
        ...

    def row(self, index: int) -> list[Any]:
        """Get an entire row by index (0-based)."""
        ...

    def column(self, index: int) -> list[Any]:
        """Get an entire column by index (0-based)."""
        ...

    def column_by_name(self, name: str) -> list[Any]:
        """Get an entire column by name."""
        ...

    def name_columns_by_row(self, row_index: int) -> None:
        """Use the specified row as column headers."""
        ...

    def column_names(self) -> Optional[list[str]]:
        """Get column names (if set)."""
        ...

    def row_append(self, data: list[Any]) -> None:
        """Append a row to the end of the sheet."""
        ...

    def row_insert(self, index: int, data: list[Any]) -> None:
        """Insert a row at a specific index."""
        ...

    def row_delete(self, index: int) -> None:
        """Delete a row at a specific index."""
        ...

    def column_append(self, data: list[Any]) -> None:
        """Append a column to the end of each row."""
        ...

    def column_delete(self, index: int) -> None:
        """Delete a column at a specific index."""
        ...

    def column_delete_by_name(self, name: str) -> None:
        """Delete a column by name."""
        ...

    def to_list(self) -> list[list[Any]]:
        """Convert to a 2D list."""
        ...

    def to_dict(self) -> dict[str, list[Any]]:
        """Convert to a dictionary (column name -> values)."""
        ...

    def save_as_csv(self, path: str, delimiter: Optional[str] = None) -> None:
        """Save the sheet to a CSV file."""
        ...

    def save_as_xlsx(self, path: str) -> None:
        """Save the sheet to an Excel file."""
        ...

    def to_csv_string(self) -> str:
        """Get CSV string representation."""
        ...

    def __repr__(self) -> str: ...
    def __len__(self) -> int: ...


class Book:
    """A book containing multiple sheets."""

    def __init__(self) -> None:
        """Create a new empty book."""
        ...

    @staticmethod
    def from_xlsx(path: str, has_headers: bool = False) -> Book:
        """Load a book from an Excel file (all sheets)."""
        ...

    @staticmethod
    def from_csv_dir(path: str, has_headers: bool = False) -> Book:
        """Load a book from a directory of CSV files."""
        ...

    @staticmethod
    def xlsx_sheet_names(path: str) -> list[str]:
        """Get sheet names from an Excel file without loading data."""
        ...

    def name(self) -> str:
        """Get the book name."""
        ...

    def sheet_count(self) -> int:
        """Get the number of sheets."""
        ...

    def is_empty(self) -> bool:
        """Check if the book is empty."""
        ...

    def sheet_names(self) -> list[str]:
        """Get all sheet names."""
        ...

    def has_sheet(self, name: str) -> bool:
        """Check if a sheet exists."""
        ...

    def get_sheet(self, name: str) -> Sheet:
        """Get a sheet by name (returns a copy).

        Note: This returns a copy of the sheet. Modifications to the returned
        sheet will not affect the book. Use `add_sheet` to replace a sheet
        after modifications.
        """
        ...

    def add_sheet(self, name: str, sheet: Sheet) -> None:
        """Add a sheet to the book."""
        ...

    def remove_sheet(self, name: str) -> Sheet:
        """Remove a sheet by name."""
        ...

    def rename_sheet(self, old_name: str, new_name: str) -> None:
        """Rename a sheet."""
        ...

    def save_as_xlsx(self, path: str) -> None:
        """Save the book to an Excel file."""
        ...

    def __repr__(self) -> str: ...
    def __len__(self) -> int: ...
