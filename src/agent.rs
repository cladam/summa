//! LLM agent module for structured summarization.
//!
//! Uses rstructor for structured output from LLMs.

pub use crate::summary::Summary;

use crate::config::Config;
use rstructor::{GeminiClient, GeminiModel, LLMClient};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("LLM request failed: {0}")]
    RequestFailed(String),
    #[error("failed to parse response: {0}")]
    ParseError(String),
    #[error("configuration error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
}

/// Run the summarization agent on the provided text
pub async fn summarize(text: &str, config: &Config) -> Result<Summary, AgentError> {
    let api_key = config.api_key()?;

    // Parse the model from config
    let model = parse_gemini_model(&config.agent.model);

    // Build the client
    let client = GeminiClient::new(api_key)
        .map_err(|e| AgentError::RequestFailed(e.to_string()))?
        .model(model);

    // Build the prompt including persona, schema, and text
    let prompt = format!(
        r#"{}

{}

You MUST respond with valid JSON matching this exact schema:
{{
  "title": "string - a concise title for the content",
  "conclusion": "string - the main takeaway or conclusion of the article in 1-2 sentences",
  "key_points": ["array of key takeaways"],
  "entities": ["array of named entities like people, organizations, technologies"],
  "action_items": ["array of actionable items or next steps, can be empty"]
}}

Do not include any markdown formatting, code blocks, or explanations. Only output the raw JSON object.

---

{}"#,
        config.agent.persona, config.agent.prompt, text
    );

    // Get structured output using the Instructor trait
    let result = client
        .generate_with_metadata(&prompt)
        .await
        .map_err(|e| AgentError::RequestFailed(e.to_string()))?;

    // Debug: print raw response
    // eprintln!("--- Raw LLM Response ---");
    // eprintln!("{}", result.text);
    // eprintln!("--- End Response ---");

    // Clean the response (strip markdown code blocks if present)
    let cleaned = strip_markdown_json(&result.text);

    // Parse the JSON response into Summary
    let summary: Summary = serde_json::from_str(&cleaned)
        .map_err(|e| AgentError::ParseError(format!("{}: {}", e, cleaned)))?;

    Ok(summary)
}

/// Strip markdown code block wrappers from JSON response
fn strip_markdown_json(text: &str) -> String {
    let trimmed = text.trim();

    // Remove ```json ... ``` or ``` ... ```
    if trimmed.starts_with("```") {
        let without_prefix = if trimmed.starts_with("```json") {
            &trimmed[7..]
        } else {
            &trimmed[3..]
        };

        if let Some(end_idx) = without_prefix.rfind("```") {
            return without_prefix[..end_idx].trim().to_string();
        }
    }

    trimmed.to_string()
}

/// Parse a model string into a GeminiModel
fn parse_gemini_model(model: &str) -> GeminiModel {
    match model {
        "gemini-2.0-flash" => GeminiModel::Gemini20Flash,
        "gemini-2.5-flash" => GeminiModel::Gemini25Flash,
        "gemini-2.5-pro" => GeminiModel::Gemini25Pro,
        _ => GeminiModel::Gemini20Flash, // Default
    }
}
