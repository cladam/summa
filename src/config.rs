//! Configuration loading and management for summa.
//!
//! Loads settings from `summa.toml` with environment variable overrides for sensitive data.
//! If no config file exists, creates a default one in `~/.config/summa/summa.toml`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Default persona for the LLM agent
const DEFAULT_PERSONA: &str =
    "You are a senior research assistant specializing in technical synthesis.";

/// Default prompt for summarisation
const DEFAULT_PROMPT: &str = "Can you provide a comprehensive summary of the given text? The summary should cover all the key points and main ideas presented in the original text, while also condensing the information into a concise and easy-to-understand format. Please ensure that the summary includes relevant details and examples that support the main ideas, while avoiding any unnecessary information or repetition. The length of the summary should be appropriate for the length and complexity of the original text, providing a clear and accurate overview without omitting any important information.";

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),
    #[error("missing required API key for provider: {0}")]
    MissingApiKey(String),
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// LLM provider: "gemini" or "openai"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Model identifier (e.g., "gemini-2.0-flash")
    #[serde(default = "default_model")]
    pub model: String,
    /// System persona for the agent
    #[serde(default = "default_persona")]
    pub persona: String,
    /// Prompt template for summarisation
    #[serde(default = "default_prompt")]
    pub prompt: String,
}

fn default_provider() -> String {
    "gemini".to_string()
}

fn default_model() -> String {
    "gemini-2.0-flash".to_string()
}

fn default_persona() -> String {
    DEFAULT_PERSONA.to_string()
}

fn default_prompt() -> String {
    DEFAULT_PROMPT.to_string()
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            persona: default_persona(),
            prompt: default_prompt(),
        }
    }
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

impl Default for StorageConfig {
    fn default() -> Self {
        let default_path = dirs::data_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".summa")
            })
            .join("summa_data");

        Self { path: default_path }
    }
}

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub storage: StorageConfig,
}

impl Config {
    /// Get the default config directory path
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config")
            })
            .join("summa")
    }

    /// Get the default config file path
    pub fn config_file_path() -> PathBuf {
        Self::config_dir().join("summa.toml")
    }

    /// Load configuration from the default location, creating it if it doesn't exist
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

    /// Find the config file, creating a default one if it doesn't exist
    fn find_config_file() -> Result<PathBuf, ConfigError> {
        // Check current directory first
        let local_config = PathBuf::from("summa.toml");
        if local_config.exists() {
            return Ok(local_config);
        }

        // Check default config directory
        let default_config = Self::config_file_path();
        if default_config.exists() {
            return Ok(default_config);
        }

        // Create default config file
        Self::create_default_config()?;
        Ok(default_config)
    }

    /// Create the default config file with sensible defaults
    fn create_default_config() -> Result<(), ConfigError> {
        let config_dir = Self::config_dir();
        std::fs::create_dir_all(&config_dir)?;

        let default_config = Config::default();
        let config_content = toml::to_string_pretty(&default_config)?;

        let config_path = Self::config_file_path();
        std::fs::write(&config_path, config_content)?;

        eprintln!("Created default config at: {}", config_path.display());

        Ok(())
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

