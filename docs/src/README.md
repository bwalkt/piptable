# PipTable Documentation

Welcome to **PipTable** - a powerful data processing DSL that combines VBA-like syntax with SQL capabilities.

## What is PipTable?

PipTable is a domain-specific language designed for data manipulation and analysis. It provides an intuitive syntax for working with tabular data, combining the familiarity of VBA-style programming with the power of SQL queries.

### Key Features

- **Familiar Syntax**: VBA-like syntax that's easy to learn
- **SQL Integration**: Embedded SQL queries with `query()` expressions
- **Multiple File Formats**: Support for CSV, JSON, Excel, Parquet, and more
- **Data Operations**: Built-in append, upsert, and join operations
- **HTTP Support**: Fetch data from APIs with `fetch()`
- **AI Integration**: Ask questions about data with `ask()`
- **Type Safety**: Optional type hints for better code clarity

## Quick Example

```vba
' Load sales data
dim sales = import "sales.csv" into sheet

' Filter and transform
dim highValueSales = query("
    SELECT customer, product, amount 
    FROM sales 
    WHERE amount > 1000
    ORDER BY amount DESC
")

' Export results
export highValueSales to "high_value_sales.xlsx"
```

## Who Should Use PipTable?

PipTable is perfect for:

- **Data Analysts** who need a simple scripting language for ETL pipelines
- **Business Users** familiar with VBA who want to process data
- **Developers** building data processing workflows
- **Teams** needing a readable DSL for data operations

## Getting Help

- **Quick Start**: Jump into our [Getting Started](guide/getting-started.md) guide
- **Examples**: Browse practical examples in the [Cookbook](cookbook/data-processing.md)
- **Reference**: Detailed documentation in the [DSL Reference](reference/dsl/README.md)
- **Community**: Join discussions on [GitHub](https://github.com/bwalkt/piptable)

## Documentation Structure

This documentation is organized into several sections:

1. **User Guide**: Step-by-step tutorials and core concepts
2. **Reference**: Complete DSL and API documentation
3. **Cookbook**: Practical examples and patterns
4. **Development**: For contributors and advanced users

Use the navigation sidebar to explore each section.

## Version

This documentation is for PipTable v0.1.0.

## License

PipTable is open source software licensed under the MIT License.