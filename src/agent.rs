//! LLM agent module for structured summarization.
//!
//! Uses rstructor for structured output from LLMs.

pub use crate::summary::Summary;

use crate::config::Config;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("LLM request failed: {0}")]
    RequestFailed(String),
    #[error("failed to parse LLM response: {0}")]
    ParseError(String),
    #[error("configuration error: {0}")]
    ConfigError(#[from] crate::config::ConfigError),
}

/// Run the summarization agent on the provided text
pub async fn summarize(_text: &str, _config: &Config) -> Result<Summary, AgentError> {
    // TODO: Implement with rstructor once added to dependencies
    // let client = GeminiClient::from_env()?
    //     .model(&config.agent.model)
    //     .persona(&config.agent.persona);
    // let summary: Summary = client.materialize(text).await?;

    todo!("implement LLM summarization with rstructor")
}
