# summera

Intelligent content summarisation powered by LLMs with a beautiful TUI.

Summarise webpages, PDFs, and PowerPoint presentations from the command line or
an interactive terminal UI.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Structured Intelligence**: Returns typed summaries with key points, conclusions, entities, and action items
- **Multiple Sources**: Summarise webpages, local PDF files, and PPTX presentations
- **Hybrid Storage**: sled for persistent storage, tantivy for full-text search
- **Provider Agnostic**: Supports Gemini and OpenAI via rstructor
- **Beautiful TUI**: Split-pane interface with summary list and scrollable detail view
- **CLI Support**: Use from the command line for scripting and automation

## Installation

### Prerequisites

- **Rust** toolchain (`cargo`)

```bash
# Clone and build
git clone https://github.com/cladam/summa.git
cd summa
cargo build --release

# The binary is at target/release/summera
```

#### Installing from crates.io

The easiest way to install `summera` is to download it from [crates.io](https://crates.io/crates/summera). You can do it
using the following command:

```bash
cargo install summera
```

If you want to update `summera` to the latest version, execute the following command:

```bash
summera update
```

## Usage

### TUI Mode (default)

Simply run `summera` to launch the interactive TUI:

```bash
summera
```

**Key bindings:**

- `o` - Open a URL or local file to summarise
- `f` - Search stored summaries
- `↑/↓` or `j/k` - Navigate summary list
- `Tab` - Switch between list and detail panes
- `PageUp/PageDown` - Scroll detail view
- `Esc` - Clear search / Cancel dialogue
- `q` - Quit

### CLI Mode

#### Summarise a webpage

```bash
summera summarise <URL>
```

Example:

```bash
summera summarise https://cladam.github.io/2025/12/22/lewin-and-devops/
```

#### Summarise a local file

Summera can extract text from **PDF** and **PPTX** files and summarise them
just like a webpage:

```bash
# PDF
summera summarise ~/Documents/quarterly-report.pdf

# PowerPoint (PPTX)
summera summarise ./slides/architecture-overview.pptx
```

> **Note:** The legacy binary `.ppt` format is not supported, only `.pptx`
> (Office Open XML).

#### View raw extracted text

Useful for inspecting what the LLM will actually see:

```bash
summera summarise <URL-or-FILE> --raw
```

#### Search stored summaries

```bash
summera search <QUERY>
```

Example:

```bash
summera search "DevOps"
```

#### List all stored summaries

```bash
summera list
```

## Configuration

On the first run, `summera` will automatically create a default configuration file at the standard location for your
operating system:

* **macOS**: `~/Library/Application Support/summera/summera.toml`
* **Linux**: `~/.config/summera/summera.toml`
* **Windows**: `%APPDATA%\summera\summera.toml`

Summera looks for configuration in the following locations (in order):

1. `./summera.toml` (current directory)
2. `summera.toml` in default config directory

If no config file exists, a default one is created automatically.

### Example `summera.toml`

```toml
[agent]
provider = "gemini"           # "gemini" or "openai"
model = "gemini-2.0-flash"    # Model identifier
persona = "You are a senior research assistant specialising in technical synthesis."
prompt = "Can you provide a comprehensive summary of the given text? ..."

[storage]
path = "/path/to/data"        # Where to store summaries

[api]
gemini_key = "AIza..."
```

### API Keys

Use the section in `summera.toml` or set your API key as an environment variable:

```bash
# For Gemini
export GEMINI_API_KEY="your-api-key"

# For OpenAI
export OPENAI_API_KEY="your-api-key"
```

## Data Storage

Summera stores data in two locations within the configured storage path:

- **sled database**: Stores full summary data with timestamps
- **tantivy index**: Full-text search index for fast querying

Default location: `~/.local/share/summera_data/`

## Architecture

```
src/
├── main.rs      # CLI entry point and argument parsing
├── lib.rs       # Library exports
├── agent.rs     # LLM integration via rstructor
├── config.rs    # Configuration loading and management
├── reader.rs    # Local file text extraction (PDF, PPTX)
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
- **pdf-extract**: PDF text extraction
- **zip** / **quick-xml**: PPTX (Office Open XML) parsing
- **tokio**: Async runtime
- **clap**: CLI argument parsing

## Supported Formats

| Format              | Extension             | Support         |
|---------------------|-----------------------|-----------------|
| Webpage             | `http://`, `https://` | ✅ Full          |
| PDF                 | `.pdf`                | ✅ Full          |
| PowerPoint (OOXML)  | `.pptx`               | ✅ Full          |
| PowerPoint (legacy) | `.ppt`                | ❌ Not supported |

## License

MIT

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.