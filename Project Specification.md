## 1. The Tech stack

| Layer             | Crate                   | Why?                                                                                                                                                            |
|-------------------|-------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **TUI**           | `ratatui` + `crossterm` | The gold standard. Use the **Component-based** pattern for high responsiveness.                                                                                 |
| **CLI / Input**   | `clap` (v4+)            | Industry standard for argument parsing and URL input.                                                                                                           |
| **LLM Interface** | `rstructor`             | The 2026 "Instructor" for Rust. It handles **Structured Output** (forcing Gemini to return your JSON schema) and supports OpenAI/Anthropic/Gemini with one API. |
| **Database**      | `sled`                  | A high-performance, embedded K/V store. Use it for raw summary storage.                                                                                         |
| **Search Engine** | `tantivy`               | "Lucene in Rust." It will index your `sled` data for full-text search.                                                                                          |
| **Web Scraping**  | `reqwest` + `scraper`   | `reqwest` for the fetch, `scraper` for the CSS-based data extraction.                                                                                           |

---

## 2. Project Specification: `summa`

### Core Features

* **Structured Intelligence:** The agent won't just dump text; it will return a strictly typed `Summary` struct (Key Points, Entities, Actions).
* **Hybrid Storage:** `sled` stores the full JSON objects by URL hash; `tantivy` indexes the `text` fields for instant TUI search.
* **Provider Agnostic:** Uses the `LLMClient` trait from `rstructor` so you can swap `GeminiClient` for `OpenAIClient` in your config.

### Project Structure

```text
summa/
├── Cargo.toml
├── summa.toml          # Config: API keys, Persona, Default Model
├── src/
│   ├── main.rs         # Entry point, Clap setup, TUI loop
│   ├── agent/          # LLM logic & Structured Output schemas
│   ├── db/             # Sled & Tantivy integration (Search & Storage)
│   ├── ui/             # Ratatui components (Input, Loading, Results)
│   └── scraper/        # Web content extraction logic

```

---

## 3. Implementation Blueprint

### A. The Config (`summa.toml`)

We'll use `serde` and `toml` to load this.

```toml
[agent]
provider = "gemini" # or "openai"
model = "gemini-3-flash"
persona = "You are a senior research assistant specializing in technical synthesis."

[api]
gemini_key = "AIza..."

[storage]
path = "./data"

```

### B. The Structured Agent (using `rstructor`)

This ensures the LLM output is valid and searchable.

```rust
use rstructor::{Instructor, GeminiClient};
use serde::{Serialize, Deserialize};

#[derive(Instructor, Serialize, Deserialize, Debug)]
pub struct Summary {
    pub title: String,
    pub key_points: Vec<String>,
    pub entities: Vec<String>,
    pub action_items: Vec<String>,
}

pub async fn run_agent(text: String, config: Config) -> Result<Summary, Error> {
    let client = GeminiClient::from_env()?
        .model(config.agent.model)
        .persona(config.agent.persona);
        
    // rstructor forces the LLM to follow the Summary schema
    let summary: Summary = client.materialize(&text).await?;
    Ok(summary)
}

```

### C. Search & Storage (Sled + Tantivy)

You can use `typed-sled` to make `sled` feel more like a native Rust collection.

---

## 4. Why this stack?

1. **Searchability:** By using **Tantivy**, your TUI can have a "Search" mode where you type "Rust" and it instantly pulls up every webpage summary mentioning the language.
2. **Type Safety:** `rstructor` solves the biggest problem with LLMs—malformed JSON. If Gemini sends back a bad field, `rstructor` automatically catches it (and can even retry).
3. **Local First:** Since `sled` and `tantivy` are embedded, your research history is private and stays on your machine.

