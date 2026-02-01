# PDF Table Import

This recipe shows how to extract tables from PDFs using the DSL.

## Import All Tables

```piptable
dim tables = import "report.pdf" into book
dim first = tables["table_1"]
```

## Notes

- PDF import extracts tables only.
- Use the book keys `table_1`, `table_2`, ... to access tables.
- File access is not available in the playground.
