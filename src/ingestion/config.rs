//! Configuration for the ingestion module

use crate::log_feature;
use crate::logging::features::LogFeature;
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
            model: "google/gemini-2.5-flash".to_string(),
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
    pub provider: AIProvider,
    pub openrouter: OpenRouterConfig,
    pub ollama: OllamaConfig,
    pub enabled: bool,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub auto_execute_mutations: bool,
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

    /// Load and fully validate config. Returns an error if any provider field
    /// is invalid (missing API key, empty model, etc.).
    pub fn from_env() -> Result<Self, crate::ingestion::IngestionError> {
        let config = Self::load()?;
        config.validate()?;
        Ok(config)
    }

    /// Load config from the saved file and environment variables.
    ///
    /// Precedence (highest to lowest):
    /// - `FOLD_OPENROUTER_API_KEY` env var (secrets never live in files)
    /// - Saved config file (UI choices)
    /// - Other env vars (only when no saved config)
    /// - Compiled-in defaults
    ///
    /// Returns an error if the config file exists but cannot be read or parsed.
    pub fn load() -> Result<Self, crate::ingestion::IngestionError> {
        let mut config = IngestionConfig::default();

        // Apply saved config (UI choices override defaults).
        // No FOLD_CONFIG_DIR or file not found → silent fallback to defaults.
        // File exists but unreadable/unparseable → fail fast.
        let has_saved = match Self::config_file_path() {
            None => {
                log_feature!(LogFeature::Ingestion, info, "FOLD_CONFIG_DIR not set; using env vars/defaults");
                false
            }
            Some(path) if !path.exists() => {
                log_feature!(LogFeature::Ingestion, info, "No saved ingestion config at {}; using env vars/defaults", path.display());
                false
            }
            Some(path) => {
                let saved = Self::load_from_file(&path)?;
                log_feature!(
                    LogFeature::Ingestion,
                    info,
                    "Loaded saved ingestion config: provider={:?}, model={}",
                    saved.provider,
                    match saved.provider {
                        AIProvider::Ollama => &saved.ollama.model,
                        AIProvider::OpenRouter => &saved.openrouter.model,
                    }
                );
                config.provider = saved.provider;
                config.openrouter = saved.openrouter;
                config.ollama = saved.ollama;
                true
            }
        };

        // API key: env var always wins — secrets shouldn't live in config files
        if let Ok(key) = env::var("FOLD_OPENROUTER_API_KEY") {
            config.openrouter.api_key = key;
        }

        // Provider selection and non-secret model settings only apply when
        // there's no saved config (saved config already has these)
        if !has_saved {
            if let Ok(p) = env::var("AI_PROVIDER") {
                config.provider = if p.to_lowercase() == "ollama" {
                    AIProvider::Ollama
                } else {
                    AIProvider::OpenRouter
                };
            }
            if let Ok(v) = env::var("OPENROUTER_MODEL") { config.openrouter.model = v; }
            if let Ok(v) = env::var("OPENROUTER_BASE_URL") { config.openrouter.base_url = v; }
            if let Ok(v) = env::var("OLLAMA_MODEL") { config.ollama.model = v; }
            if let Ok(v) = env::var("OLLAMA_BASE_URL") { config.ollama.base_url = v; }
        }

        // Runtime settings: env vars override defaults; unset vars keep the compiled default.
        config.enabled = env_bool("INGESTION_ENABLED", config.enabled);
        config.max_retries = env_parse("INGESTION_MAX_RETRIES", config.max_retries);
        config.timeout_seconds = env_parse("INGESTION_TIMEOUT_SECONDS", config.timeout_seconds);
        config.auto_execute_mutations = env_bool("INGESTION_AUTO_EXECUTE", config.auto_execute_mutations);

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

    /// Save provider/model settings to the config file.
    ///
    /// If the incoming api_key is empty or redacted, the existing saved key is
    /// preserved. If the file exists but cannot be read, returns an error rather
    /// than silently clearing the key.
    pub fn save_to_file(config: &SavedConfig) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_file_path().ok_or("FOLD_CONFIG_DIR is not set; cannot save ingestion config")?;

        let mut to_save = config.clone();
        if to_save.openrouter.api_key.is_empty() || to_save.openrouter.api_key == "***configured***" {
            if config_path.exists() {
                let existing = Self::load_from_file(&config_path)
                    .map_err(|e| format!("Failed to read existing config to preserve API key: {e}"))?;
                to_save.openrouter.api_key = existing.openrouter.api_key;
            } else {
                to_save.openrouter.api_key = String::new();
            }
        }

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&to_save)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Path to the ingestion config file, or `None` if `FOLD_CONFIG_DIR` is not set.
    /// Saving config requires `FOLD_CONFIG_DIR`; loading without it silently uses defaults.
    pub fn get_config_file_path() -> Option<std::path::PathBuf> {
        Self::config_file_path()
    }

    fn config_file_path() -> Option<std::path::PathBuf> {
        env::var("FOLD_CONFIG_DIR")
            .ok()
            .map(|dir| std::path::Path::new(&dir).join("ingestion_config.json"))
    }

    fn load_from_file(
        path: &std::path::Path,
    ) -> Result<SavedConfig, crate::ingestion::IngestionError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::ingestion::IngestionError::configuration_error(format!(
                "Failed to read config file {}: {e}",
                path.display()
            ))
        })?;
        let mut saved: SavedConfig = serde_json::from_str(&content).map_err(|e| {
            crate::ingestion::IngestionError::configuration_error(format!(
                "Failed to parse config file {}: {e}",
                path.display()
            ))
        })?;
        // Strip redaction placeholder — treat as "no key configured"
        if saved.openrouter.api_key == "***configured***" {
            saved.openrouter.api_key = String::new();
        }
        Ok(saved)
    }

    // Keep old name as a hidden alias so existing callers compile without changes.
    #[doc(hidden)]
    pub fn from_env_allow_empty() -> Self {
        Self::load().unwrap_or_else(|e| {
            log::error!("Failed to load ingestion config: {e}. Using defaults.");
            IngestionConfig::default()
        })
    }
}

/// Provider/model settings persisted to disk by the UI.
/// Runtime fields (enabled, retries, timeout) are controlled via env vars only.
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct SavedConfig {
    pub provider: AIProvider,
    pub openrouter: OpenRouterConfig,
    pub ollama: OllamaConfig,
}

// ---- env var helpers ----

fn env_bool(name: &str, default: bool) -> bool {
    env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn env_parse<T: std::str::FromStr>(name: &str, default: T) -> T {
    env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IngestionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.provider, AIProvider::OpenRouter);
        assert_eq!(config.openrouter.model, "google/gemini-2.5-flash");
        assert_eq!(config.openrouter.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(config.ollama.model, "llama3.3");
        assert_eq!(config.ollama.base_url, "http://localhost:11434");
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_seconds, 300);
        assert!(config.auto_execute_mutations);
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
