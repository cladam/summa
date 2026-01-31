//! Configuration loading and management for summa.
//!
//! Loads settings from `summa.toml` with environment variable overrides for sensitive data.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("missing required API key for provider: {0}")]
    MissingApiKey(String),
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// LLM provider: "gemini" or "openai"
    pub provider: String,
    /// Model identifier (e.g., "gemini-2.0-flash")
    pub model: String,
    /// System persona for the agent
    pub persona: String,
}

/// API keys configuration (loaded from environment)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    #[serde(default)]
    pub gemini_key: Option<String>,
    #[serde(default)]
    pub openai_key: Option<String>,
}

/// Storage paths configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base path for data storage
    pub path: PathBuf,
}

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    #[serde(default)]
    pub api: ApiConfig,
    pub storage: StorageConfig,
}

impl Config {
    /// Load configuration from the default location (summa.toml in cwd or home)
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::find_config_file()?;
        Self::load_from(&config_path)
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;

        // Override API keys from environment variables
        if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            config.api.gemini_key = Some(key);
        }
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            config.api.openai_key = Some(key);
        }

        Ok(config)
    }

    /// Find the config file in standard locations
    fn find_config_file() -> Result<PathBuf, ConfigError> {
        // Check current directory first
        let local_config = PathBuf::from("summa.toml");
        if local_config.exists() {
            return Ok(local_config);
        }

        // Check home directory
        if let Some(home) = dirs::home_dir() {
            let home_config = home.join(".config").join("summa").join("summa.toml");
            if home_config.exists() {
                return Ok(home_config);
            }
        }

        // Default to local path (will error on read)
        Ok(local_config)
    }

    /// Get the API key for the configured provider
    pub fn api_key(&self) -> Result<&str, ConfigError> {
        match self.agent.provider.as_str() {
            "gemini" => self
                .api
                .gemini_key
                .as_deref()
                .ok_or_else(|| ConfigError::MissingApiKey("gemini".to_string())),
            "openai" => self
                .api
                .openai_key
                .as_deref()
                .ok_or_else(|| ConfigError::MissingApiKey("openai".to_string())),
            other => Err(ConfigError::MissingApiKey(other.to_string())),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./data"),
        }
    }
}
