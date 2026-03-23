//! summera CLI - Intelligent webpage summarisation
//!
//! The application logic is contained in lib.rs, and this file is responsible
//! for parsing arguments and handling top-level errors.

use clap::{Parser, Subcommand};
use summera::{agent, reader, scraper, ui, Config, SearchIndex, Storage};

#[derive(Parser)]
#[command(name = "summera")]
#[command(author, version, about = "TUI for intelligent webpage summarisation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Summarise a webpage by URL or a local file (PDF, PPTX)
    Summarise {
        /// URL or local file path to summarise
        source: String,
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
    /// Update summera to the latest version.
    #[command(name = "update", hide = true)] // Hidden from help
    Update,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Summarise { source, raw }) => {
            // Detect whether the source is a URL or a local file
            let (title, text, source_key) = if reader::is_url(&source) {
                println!("Fetching: {}", source);
                let content = scraper::fetch_content(&source).await?;
                let title = content.title.unwrap_or_else(|| "No title".to_string());
                (title, content.text, source.clone())
            } else {
                println!("Reading: {}", source);
                let content = reader::extract_from_file(&source)?;
                let title = content.title.unwrap_or_else(|| "No title".to_string());
                // Use absolute path as the storage key for local files
                let abs_path = std::fs::canonicalize(&source)
                    .unwrap_or_else(|_| std::path::PathBuf::from(&source));
                let key = format!("file://{}", abs_path.display());
                (title, content.text, key)
            };

            if raw {
                // Just show raw extracted text
                println!("\n=== {} ===\n", title);
                println!("{}", text);
                println!("\n--- Extracted {} characters ---", text.len());
            } else {
                // Summarise using LLM
                println!("Summarising {} characters...\n", text.len());

                let config = Config::load()?;
                let summary = agent::summarize(&text, &config).await?;

                // Persist the summary to sled storage
                let storage = Storage::open(&config.storage.path)?;
                storage.store(&source_key, &summary)?;

                // Index in tantivy for full-text search
                let search_path = config.storage.path.join("search_index");
                if let Ok(search_index) = SearchIndex::open(&search_path) {
                    if let Err(e) = search_index.index_summary(&source_key, &summary) {
                        eprintln!("Warning: Failed to index summary: {}", e);
                    }
                }

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
            let config = Config::load()?;
            let storage = Storage::open(&config.storage.path)?;

            // Try tantivy first, fall back to simple search
            let search_path = config.storage.path.join("search_index");
            let results = if let Ok(search_index) = SearchIndex::open(&search_path) {
                match search_index.search(&query, 20) {
                    Ok(urls) if !urls.is_empty() => urls,
                    _ => simple_search(&storage, &query)?,
                }
            } else {
                simple_search(&storage, &query)?
            };

            if results.is_empty() {
                println!("No results found for: {}", query);
            } else {
                println!("Search results for '{}':\n", query);
                for url in &results {
                    if let Ok(Some(stored)) = storage.get(url) {
                        println!(
                            "📄 {} ({})",
                            stored.summary.title,
                            stored.created_at.format("%Y-%m-%d %H:%M")
                        );
                        println!("   {}", stored.url);
                        println!("   {}\n", stored.summary.conclusion);
                    }
                }
            }
        }
        Some(Commands::List) => {
            let config = Config::load()?;
            let storage = Storage::open(&config.storage.path)?;
            let summaries = storage.list_all()?;

            if summaries.is_empty() {
                println!("No stored summaries found.");
            } else {
                println!("Stored summaries ({}):\n", summaries.len());
                for stored in summaries {
                    println!(
                        "📄 {} ({})",
                        stored.summary.title,
                        stored.created_at.format("%Y-%m-%d %H:%M")
                    );
                    println!("   {}", stored.url);
                    println!("   {}\n", stored.summary.conclusion);
                }
            }
        }
        Some(Commands::Update) => {
            println!("--- Checking for updates ---");
            let status = self_update::backends::github::Update::configure()
                .repo_owner("cladam")
                .repo_name("summa") // name of the repo, I haven't changed it....
                .bin_name("summera")
                .show_download_progress(true)
                .current_version(self_update::cargo_crate_version!())
                .build()?
                .update()?;
            println!("Update status: `{}`!", status.version());
        }
        None => {
            // Default: Launch the TUI
            ui::run().await?;
        }
    }

    Ok(())
}

/// Simple text-based search fallback when tantivy index is not available
fn simple_search(storage: &Storage, query: &str) -> anyhow::Result<Vec<String>> {
    let query_lower = query.to_lowercase();
    let all_summaries = storage.list_all()?;

    let results: Vec<String> = all_summaries
        .into_iter()
        .filter(|stored| {
            let summary = &stored.summary;
            summary.title.to_lowercase().contains(&query_lower)
                || summary.conclusion.to_lowercase().contains(&query_lower)
                || summary
                    .key_points
                    .iter()
                    .any(|p| p.to_lowercase().contains(&query_lower))
                || summary
                    .entities
                    .iter()
                    .any(|e| e.to_lowercase().contains(&query_lower))
        })
        .map(|stored| stored.url)
        .collect();

    Ok(results)
}
