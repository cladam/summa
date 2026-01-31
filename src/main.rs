//! Summa CLI - Intelligent webpage summarisation
//!
//! The application logic is contained in lib.rs, and this file is responsible
//! for parsing arguments and handling top-level errors.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "summa")]
#[command(author, version, about = "TUI for intelligent webpage summarisation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Summarize a webpage by URL
    Summarize {
        /// URL to summarize
        url: String,
    },
    /// Search stored summaries
    Search {
        /// Search query
        query: String,
    },
    /// List all stored summaries
    List,
    /// Launch the interactive TUI
    Tui,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Summarize { url }) => {
            println!("Summarizing: {}", url);
            // TODO: Implement summarization flow
        }
        Some(Commands::Search { query }) => {
            println!("Searching for: {}", query);
            // TODO: Implement search
        }
        Some(Commands::List) => {
            println!("Listing stored summaries...");
            // TODO: Implement listing
        }
        Some(Commands::Tui) => {
            println!("Launching TUI...");
            // TODO: Implement TUI
        }
        None => {
            println!("Launching TUI...");
            // Default to TUI mode
            // TODO: Implement TUI
        }
    }

    Ok(())
}
