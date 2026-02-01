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

    // Build the prompt including persona as context
    let prompt = format!(
        "{}\n\n{}\n\n---\n\n{}",
        config.agent.persona, config.agent.prompt, text
    );

    // Get structured output using the Instructor trait
    let result = client
        .generate_with_metadata(&prompt)
        .await
        .map_err(|e| AgentError::RequestFailed(e.to_string()))?;

    // Parse the JSON response into Summary
    let summary: Summary = serde_json::from_str(&result.text)
        .map_err(|e| AgentError::ParseError(e.to_string()))?;

    Ok(summary)
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