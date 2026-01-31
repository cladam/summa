//! # Summa
//!
//! A TUI application for intelligent webpage summarization using LLMs.
//!
//! ## Features
//!
//! - **Structured Intelligence**: Returns typed `Summary` structs with key points, entities, and actions
//! - **Hybrid Storage**: sled for raw storage, tantivy for full-text search
//! - **Provider Agnostic**: Supports Gemini and OpenAI via rstructor

pub mod agent;
pub mod config;
pub mod db;
pub mod scraper;
pub mod search;
pub mod storage;
pub mod summary;
pub mod ui;

pub use config::Config;
pub use db::{SearchIndex, Storage};
pub use summary::Summary;
