# Installation

PipTable can be installed in several ways depending on your needs.

## Using Cargo (Recommended)

The easiest way to install PipTable is using Cargo, Rust's package manager:

```bash
cargo install piptable
```

This will install the `pip` command globally on your system.

## Building from Source

For the latest development version or to contribute:

```bash
# Clone the repository
git clone https://github.com/bwalkt/piptable.git
cd piptable

# Build release version
cargo build --release

# Run tests to verify
cargo test

# Install locally
cargo install --path crates/cli
```

The binary will be available at `target/release/pip`.

## Platform-Specific Instructions

### macOS

```bash
# Using Homebrew (coming soon)
# brew install piptable

# Or use Cargo
cargo install piptable
```

### Linux

```bash
# Ensure you have Rust installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install PipTable
cargo install piptable
```

### Windows

```powershell
# Install Rust from https://rustup.rs/
# Then install PipTable
cargo install piptable
```

## Verifying Installation

After installation, verify PipTable is working:

```bash
# Check version
pip --version

# Show help
pip --help

# Run interactive mode
pip -i
```

## System Requirements

- **Rust**: 1.70 or later
- **Memory**: 512MB minimum, 2GB recommended
- **Disk**: 100MB for installation
- **OS**: macOS, Linux, Windows 10+

## Optional Dependencies

For full functionality, you may want:

- **Python 3.8+**: For Python UDF support
- **Excel**: For viewing `.xlsx` exports

## Development Setup

For development, you'll also need:

```bash
# Install development tools
cargo install cargo-watch cargo-edit

# Install mdBook for documentation
cargo install mdbook

# Run development server
cargo watch -x "run --bin pip"
```

## Troubleshooting

### Command Not Found

If `pip` is not found after installation:

```bash
# Add Cargo bin to PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Build Errors

If you encounter build errors:

```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

### Python Integration Issues

For Python UDF support:

```bash
# Ensure Python is available
python3 --version

# Install with Python feature
cargo install piptable --features python
```

## Next Steps

Now that PipTable is installed, proceed to:
- [Quick Start](quick-start.md) - Run your first commands
- [First Script](first-script.md) - Write a complete script