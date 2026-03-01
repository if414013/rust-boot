//! rust-boot CLI scaffolding tool entrypoint.

mod commands;
mod templates;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{GenerateCommand, NewCommand};

/// rust-boot CLI - A scaffolding tool for rust-boot framework projects.
#[derive(Parser, Debug)]
#[command(name = "rust-boot")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new rust-boot project
    New(NewCommand),
    /// Generate a model, handler, or other artifact
    #[command(alias = "g")]
    Generate(GenerateCommand),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New(cmd) => {
            let result = cmd.execute()?;
            println!("Generated project: {}", result.project_name);
            println!("\nGenerated files:");
            println!("  - Cargo.toml");
            println!("  - src/main.rs");
            println!("  - src/lib.rs");
            println!("  - .gitignore");
            println!("\nNote: File I/O not implemented yet. Templates rendered in memory.");
        }
        Commands::Generate(cmd) => {
            let result = cmd.execute()?;
            println!("Generated {}: {}", result.file_name, result.model_name);
            println!("\nNote: File I/O not implemented yet. Template rendered in memory.");
        }
    }

    Ok(())
}
