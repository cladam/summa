//! Web scraping module for content extraction.
//!
//! Uses reqwest for fetching and scraper for HTML parsing.

use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use thiserror::Error;

/// User-Agent string identifying this scraper
const USER_AGENT: &str = concat!("summa/", env!("CARGO_PKG_VERSION"), " (https://github.com/cladam/summa)");

/// Default timeout for HTTP requests
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Error, Debug)]
pub enum ScraperError {
    #[error("failed to fetch URL: {0}")]
    FetchError(#[from] reqwest::Error),
    #[error("no content found at URL")]
    NoContent,
}

/// Extracted content from a webpage
#[derive(Debug, Clone)]
pub struct WebContent {
    /// The original URL
    pub url: String,
    /// Page title
    pub title: Option<String>,
    /// Main text content
    pub text: String,
}

/// Create a configured HTTP client for scraping
fn create_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(REQUEST_TIMEOUT)
        .build()
}

/// Fetch and extract content from a URL
pub async fn fetch_content(url: &str) -> Result<WebContent, ScraperError> {
    let client = create_client()?;

    // Fetch the HTML
    let response = client.get(url).send().await?;
    let html = response.text().await?;
    let document = Html::parse_document(&html);

    // Extract title
    let title = extract_title(&document);

    // Extract main content
    let text = extract_text(&document);

    if text.trim().is_empty() {
        return Err(ScraperError::NoContent);
    }

    Ok(WebContent {
        url: url.to_string(),
        title,
        text,
    })
}

/// Extract the page title from <title> or <h1>
fn extract_title(document: &Html) -> Option<String> {
    // Try <title> first
    let title_selector = Selector::parse("title").unwrap();
    if let Some(element) = document.select(&title_selector).next() {
        let title: String = element.text().collect();
        if !title.trim().is_empty() {
            return Some(title.trim().to_string());
        }
    }

    // Fall back to first <h1>
    let h1_selector = Selector::parse("h1").unwrap();
    if let Some(element) = document.select(&h1_selector).next() {
        let title: String = element.text().collect();
        if !title.trim().is_empty() {
            return Some(title.trim().to_string());
        }
    }

    None
}

/// Extract readable text content from the page
fn extract_text(document: &Html) -> String {
    // Try to find main content areas first
    let main_selectors = ["article", "main", "[role='main']", ".content", "#content"];

    for selector_str in main_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = extract_text_from_element(&Html::parse_fragment(&element.html()));
                if !text.trim().is_empty() {
                    return text;
                }
            }
        }
    }

    // Fall back to extracting from body, excluding scripts/styles
    extract_text_from_element(document)
}

/// Extract text from paragraphs and headings, excluding scripts and styles
fn extract_text_from_element(document: &Html) -> String {
    let content_selector = Selector::parse("p, h1, h2, h3, h4, h5, h6, li").unwrap();

    let mut paragraphs: Vec<String> = Vec::new();

    for element in document.select(&content_selector) {
        let text: String = element.text().collect::<Vec<_>>().join(" ");
        let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");

        if !cleaned.is_empty() && cleaned.len() > 20 {
            paragraphs.push(cleaned);
        }
    }

    paragraphs.join("\n\n")
}
