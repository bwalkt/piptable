# Getting Started

Welcome to PipTable! This guide will help you get up and running quickly.

## Installation

PipTable can be installed in several ways:

### Using Cargo

```bash
cargo install piptable
```

### From Source

```bash
git clone https://github.com/bwalkt/piptable
cd piptable
cargo build --release
```

## Your First Script

Create a file called `hello.pip`:

```vba
' Hello World example
dim message = "Hello, PipTable!"
print(message)

' Load and process data
dim data = import "sample.csv" into sheet
dim result = query("SELECT * FROM data WHERE value > 100")
export result to "output.csv"
```

Run it with:

```bash
pip hello.pip
```

## Next Steps

- Learn about [Core Concepts](core-concepts.md)
- Explore the [DSL Reference](../reference/dsl/README.md)
- Try examples in the [Cookbook](../cookbook/data-processing.md)