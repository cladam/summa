# summa

Intelligent webpage summarisation powered by LLMs with a beautiful TUI.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Structured Intelligence**: Returns typed summaries with key points, conclusions, entities, and action items
- **Hybrid Storage**: sled for persistent storage, tantivy for full-text search
- **Provider Agnostic**: Supports Gemini and OpenAI via rstructor
- **Beautiful TUI**: Split-pane interface with summary list and scrollable detail view
- **CLI Support**: Use from the command line for scripting and automation

## Installation

```bash
cargo install --path .
```

## Usage

### TUI Mode (default)

Simply run `summa` to launch the interactive TUI:

```bash
summa
```

**Key bindings:**

- `o` - Open a URL to summarise
- `f` - Search stored summaries
- `↑/↓` or `j/k` - Navigate summary list
- `Tab` - Switch between list and detail panes
- `PageUp/PageDown` - Scroll detail view
- `Esc` - Clear search / Cancel dialogue
- `q` - Quit

### CLI Mode

#### Summarise a webpage

```bash
summa summarise <URL>
```

Example:

```bash
summa summarise https://cladam.github.io/2025/12/22/lewin-and-devops/
```

#### View raw extracted text

```bash
summa summarise <URL> --raw
```

#### Search stored summaries

```bash
summa search <QUERY>
```

Example:

```bash
summa search "DevOps"
```

#### List all stored summaries

```bash
summa list
```

## Configuration

On the first run, `summa` will automatically create a default configuration file at the standard location for your
operating system:

* **macOS**: `~/Library/Application Support/summa/summa.toml`
* **Linux**: `~/.config/summa/summa.toml`
* **Windows**: `%APPDATA%\summa\summa.toml`

Summa looks for configuration in the following locations (in order):

1. `./summa.toml` (current directory)
2. `summa.toml` in default config directory

If no config file exists, a default one is created automatically.

### Example `summa.toml`

```toml
[agent]
provider = "gemini"           # "gemini" or "openai"
model = "gemini-2.0-flash"    # Model identifier
persona = "You are a senior research assistant specialising in technical synthesis."
prompt = "Can you provide a comprehensive summary of the given text? The summary should cover all the key points and main ideas presented in the original text, while also condensing the information into a concise and easy-to-understand format. Please ensure that the summary includes relevant details and examples that support the main ideas, while avoiding any unnecessary information or repetition. The length of the summary should be appropriate for the length and complexity of the original text, providing a clear and accurate overview without omitting any important information. Use British English spelling and conventions throughout your response."                # Customise the summarisation prompt

[storage]
path = "/path/to/data"        # Where to store summaries

[api]
gemini_key = "AIza..."
```

### API Keys

Use the section in `summa.toml` or set your API key as an environment variable:

```bash
# For Gemini
export GEMINI_API_KEY="your-api-key"

# For OpenAI
export OPENAI_API_KEY="your-api-key"
```

## Data Storage

Summa stores data in two locations within the configured storage path:

- **sled database**: Stores full summary data with timestamps
- **tantivy index**: Full-text search index for fast querying

Default location: `~/.local/share/summa_data/`

## Architecture

```
src/
├── main.rs      # CLI entry point and argument parsing
├── lib.rs       # Library exports
├── agent.rs     # LLM integration via rstructor
├── config.rs    # Configuration loading and management
├── scraper.rs   # Web content extraction
├── search.rs    # Tantivy full-text search
├── storage.rs   # Sled persistent storage
├── summary.rs   # Summary data structure
└── ui.rs        # Ratatui TUI implementation
```

## Dependencies

- **rstructor**: Structured LLM outputs with schema enforcement
- **ratatui**: Terminal UI framework
- **sled**: Embedded database for storage
- **tantivy**: Full-text search engine
- **reqwest**: HTTP client for web scraping
- **scraper**: HTML parsing and content extraction
- **tokio**: Async runtime
- **clap**: CLI argument parsing

## License

MIT

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.