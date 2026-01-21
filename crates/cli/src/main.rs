//! # piptable-cli
//!
//! Command-line interface for the piptable DSL.

use clap::Parser;

/// piptable - A VBA-like DSL for data processing
#[derive(Parser)]
#[command(name = "pip")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Script file to execute
    #[arg(short, long)]
    file: Option<String>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        println!("piptable CLI v{}", env!("CARGO_PKG_VERSION"));
    }

    if let Some(file) = cli.file {
        println!("Would execute: {file}");
    } else {
        println!("piptable REPL - not yet implemented");
    }

    Ok(())
}
