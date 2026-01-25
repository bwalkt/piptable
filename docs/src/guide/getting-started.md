# Getting Started

Welcome to PipTable! This guide will help you get up and running quickly with data processing using our DSL.

## What You'll Learn

- How to install PipTable
- Writing your first script
- Understanding basic concepts
- Common data processing patterns

## Prerequisites

- Rust toolchain (for installation from source)
- Basic command line familiarity
- Text editor of your choice

## Quick Example

Here's what PipTable code looks like:

```vba
' Load sales data from CSV
dim sales = import "sales.csv" into sheet

' Filter high-value transactions  
dim highValue = query("
    SELECT * FROM sales 
    WHERE amount > 1000
")

' Export results
export highValue to "high_value_sales.xlsx"
```

## Next Steps

1. [Install PipTable](installation.md) - Get PipTable on your system
2. [Quick Start](quick-start.md) - Run your first script in minutes
3. [First Script](first-script.md) - Build a complete data pipeline
4. [Core Concepts](core-concepts.md) - Understand the fundamentals

## Getting Help

- **Documentation**: You're reading it!
- **Examples**: Check the [Cookbook](../cookbook/data-processing.md)
- **Issues**: [GitHub Issues](https://github.com/bwalkt/piptable/issues)
- **Source Code**: [GitHub Repository](https://github.com/bwalkt/piptable)