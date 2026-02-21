//! Configuration for the ingestion module

use serde::{Deserialize, Serialize};
use std::env;

/// Specifies the AI provider to use for ingestion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, utoipa::ToSchema)]
pub enum AIProvider {
    #[default]
    OpenRouter,
    Ollama,
}

/// Configuration for the OpenRouter AI provider.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "anthropic/claude-3.5-sonnet".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }
}

impl OpenRouterConfig {
    pub fn validate(&self) -> Result<(), crate::ingestion::IngestionError> {
        if self.api_key.is_empty() {
            return Err(crate::ingestion::IngestionError::configuration_error(
                "OpenRouter API key is required",
            ));
        }
        if self.model.is_empty() {
            return Err(crate::ingestion::IngestionError::configuration_error(
                "OpenRouter model is required",
            ));
        }
        if self.base_url.is_empty() {
            return Err(crate::ingestion::IngestionError::configuration_error(
                "OpenRouter base URL is required",
            ));
        }
        Ok(())
    }
}

/// Configuration for the Ollama AI provider.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OllamaConfig {
    pub model: String,
    pub base_url: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            model: "llama3.3".to_string(),
            base_url: "http://localhost:11434".to_string(),
        }
    }
}

impl OllamaConfig {
    pub fn validate(&self) -> Result<(), crate::ingestion::IngestionError> {
        if self.model.is_empty() {
            return Err(crate::ingestion::IngestionError::configuration_error(
                "Ollama model is required",
            ));
        }
        if self.base_url.is_empty() {
            return Err(crate::ingestion::IngestionError::configuration_error(
                "Ollama base URL is required",
            ));
        }
        Ok(())
    }
}

/// Configuration for the ingestion module.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestionConfig {
    /// The AI provider to use.
    pub provider: AIProvider,
    /// OpenRouter specific configuration.
    pub openrouter: OpenRouterConfig,
    /// Ollama specific configuration.
    pub ollama: OllamaConfig,
    /// Whether ingestion is enabled.
    pub enabled: bool,
    /// Maximum number of retries for API calls.
    pub max_retries: u32,
    /// Timeout for API calls in seconds.
    pub timeout_seconds: u64,
    /// Whether to auto-execute mutations after generation.
    pub auto_execute_mutations: bool,
    /// Default trust distance for mutations.
    pub default_trust_distance: u32,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            provider: AIProvider::default(),
            openrouter: OpenRouterConfig::default(),
            ollama: OllamaConfig::default(),
            enabled: false,
            max_retries: 3,
            timeout_seconds: 300,
            auto_execute_mutations: true,
            default_trust_distance: 0,
        }
    }
}

impl IngestionConfig {
    /// Return a copy with sensitive values (API keys) masked for safe display.
    pub fn redacted(&self) -> Self {
        let mut copy = self.clone();
        if !copy.openrouter.api_key.is_empty() {
            copy.openrouter.api_key = "***configured***".to_string();
        }
        copy
    }

    /// Create a new ingestion config from environment variables and saved config file.
    pub fn from_env() -> Result<Self, crate::ingestion::IngestionError> {
        let config = Self::from_env_allow_empty();

        match config.provider {
            AIProvider::OpenRouter => {
                if config.openrouter.api_key.is_empty() {
                    return Err(crate::ingestion::IngestionError::configuration_error(
                        "OpenRouter API key is required. Set FOLD_OPENROUTER_API_KEY or configure in the UI.",
                    ));
                }
            }
            AIProvider::Ollama => {
                // No specific required fields for Ollama at the moment,
                // as it often runs without an API key.
            }
        }

        Ok(config)
    }

    /// Create a new ingestion config allowing empty API key (for configuration endpoints).
    pub fn from_env_allow_empty() -> Self {
        // Load provider from environment, default to OpenRouter
        let provider = env::var("AI_PROVIDER")
            .ok()
            .map(|p| match p.to_lowercase().as_str() {
                "ollama" => AIProvider::Ollama,
                _ => AIProvider::OpenRouter,
            })
            .unwrap_or_default();

        let mut config = IngestionConfig {
            provider,
            ..Default::default()
        };

        // Load saved config if it exists
        if let Ok(saved_config) = Self::load_saved_config() {
            config.provider = saved_config.provider;
            config.openrouter = saved_config.openrouter;
            config.ollama = saved_config.ollama;
        }

        // Override with environment variables if they are set
        if let Ok(key) = env::var("FOLD_OPENROUTER_API_KEY") {
            config.openrouter.api_key = key;
        }
        if let Ok(model) = env::var("OPENROUTER_MODEL") {
            config.openrouter.model = model;
        }
        if let Ok(url) = env::var("OPENROUTER_BASE_URL") {
            config.openrouter.base_url = url;
        }
        if let Ok(model) = env::var("OLLAMA_MODEL") {
            config.ollama.model = model;
        }
        if let Ok(url) = env::var("OLLAMA_BASE_URL") {
            config.ollama.base_url = url;
        }

        config.enabled = env::var("INGESTION_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        config.max_retries = env::var("INGESTION_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .unwrap_or(3);

        config.timeout_seconds = env::var("INGESTION_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .unwrap_or(300);

        config.auto_execute_mutations = env::var("INGESTION_AUTO_EXECUTE")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        config.default_trust_distance = env::var("INGESTION_DEFAULT_TRUST_DISTANCE")
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .unwrap_or(0);

        config
    }

    /// Load saved configuration from file.
    fn load_saved_config() -> Result<SavedConfig, Box<dyn std::error::Error>> {
        use std::fs;
        use std::path::Path;

        let config_dir = env::var("FOLD_CONFIG_DIR").unwrap_or_else(|_| "./config".to_string());
        let config_path = Path::new(&config_dir).join("ingestion_config.json");

        if !config_path.exists() {
            return Err("Config file does not exist".into());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: SavedConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Validate the configuration based on the selected provider.
    pub fn validate(&self) -> Result<(), crate::ingestion::IngestionError> {
        match self.provider {
            AIProvider::OpenRouter => self.openrouter.validate(),
            AIProvider::Ollama => self.ollama.validate(),
        }
    }

    /// Check if ingestion is enabled and properly configured.
    pub fn is_ready(&self) -> bool {
        self.enabled && self.validate().is_ok()
    }

    /// Save configuration to file.
    ///
    /// If the incoming `api_key` is empty and an existing config has one,
    /// the existing key is preserved (prevents accidental clearing).
    pub fn save_to_file(config: &SavedConfig) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use std::io::Write;

        let config_path = Self::get_config_file_path();

        // Merge: preserve existing api_key when the incoming value is empty
        let merged = if config.openrouter.api_key.is_empty() {
            if let Ok(existing) = Self::load_saved_config() {
                if !existing.openrouter.api_key.is_empty() {
                    let mut merged = config.clone();
                    merged.openrouter.api_key = existing.openrouter.api_key;
                    merged
                } else {
                    config.clone()
                }
            } else {
                config.clone()
            }
        } else {
            config.clone()
        };

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&merged)?;
        let mut file = fs::File::create(&config_path)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    /// Get the path to the ingestion configuration file
    pub fn get_config_file_path() -> std::path::PathBuf {
        let config_dir =
            std::env::var("FOLD_CONFIG_DIR").unwrap_or_else(|_| "./config".to_string());

        std::path::Path::new(&config_dir).join("ingestion_config.json")
    }
}

/// Structure for saving AI provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct SavedConfig {
    pub provider: AIProvider,
    pub openrouter: OpenRouterConfig,
    pub ollama: OllamaConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IngestionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.provider, AIProvider::OpenRouter);
        assert_eq!(config.openrouter.model, "anthropic/claude-3.5-sonnet");
        assert_eq!(config.openrouter.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(config.ollama.model, "llama3.3");
        assert_eq!(config.ollama.base_url, "http://localhost:11434");
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_seconds, 300);
        assert!(config.auto_execute_mutations);
        assert_eq!(config.default_trust_distance, 0);
    }

    #[test]
    fn test_validation_openrouter_fails_without_api_key() {
        let config = IngestionConfig {
            provider: AIProvider::OpenRouter,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_openrouter_succeeds_with_api_key() {
        let mut config = IngestionConfig {
            provider: AIProvider::OpenRouter,
            ..Default::default()
        };
        config.openrouter.api_key = "test-key".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_ollama_succeeds_by_default() {
        let config = IngestionConfig {
            provider: AIProvider::Ollama,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_is_ready() {
        let mut config = IngestionConfig {
            provider: AIProvider::OpenRouter,
            ..Default::default()
        };
        assert!(!config.is_ready());

        config.enabled = true;
        config.openrouter.api_key = "test-key".to_string();
        assert!(config.is_ready());

        config.provider = AIProvider::Ollama;
        assert!(config.is_ready());
    }
}
