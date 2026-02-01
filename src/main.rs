//! Summa CLI - Intelligent webpage summarisation
//!
//! The application logic is contained in lib.rs, and this file is responsible
//! for parsing arguments and handling top-level errors.

use clap::{Parser, Subcommand};
use summa::{agent, scraper, ui, Config};

#[derive(Parser)]
#[command(name = "summa")]
#[command(author, version, about = "TUI for intelligent webpage summarisation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Summarise a webpage by URL
    Summarise {
        /// URL to summarize
        url: String,
        /// Show raw extracted text instead of summary
        #[arg(long)]
        raw: bool,
    },
    /// Search stored summaries
    Search {
        /// Search query
        query: String,
    },
    /// List all stored summaries
    List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Summarise { url, raw }) => {
            println!("Fetching: {}", url);

            // Scrape the content
            let content = scraper::fetch_content(&url).await?;
            let title = content
                .title
                .clone()
                .unwrap_or_else(|| "No title".to_string());

            if raw {
                // Just show raw extracted text
                println!("\n=== {} ===\n", title);
                println!("{}", content.text);
                println!("\n--- Extracted {} characters ---", content.text.len());
            } else {
                // Summarise using LLM
                println!("Summarising {} characters...\n", content.text.len());

                let config = Config::load()?;
                let summary = agent::summarize(&content.text, &config).await?;

                println!("=== {} ===\n", summary.title);

                println!("💡 Conclusion:");
                println!("  {}\n", summary.conclusion);

                println!("📌 Key Points:");
                for point in &summary.key_points {
                    println!("  • {}", point);
                }

                if !summary.entities.is_empty() {
                    println!("\n🏷️  Entities:");
                    println!("  {}", summary.entities.join(", "));
                }

                if !summary.action_items.is_empty() {
                    println!("\n✅ Action Items:");
                    for item in &summary.action_items {
                        println!("  • {}", item);
                    }
                }
            }
        }
        Some(Commands::Search { query }) => {
            println!("Searching for: {}", query);
            // TODO: Implement search
        }
        Some(Commands::List) => {
            println!("Listing stored summaries...");
            // TODO: Implement listing
        }
        None => {
            // Default: Launch the TUI
            ui::run().await?;
        }
    }

    Ok(())
}
