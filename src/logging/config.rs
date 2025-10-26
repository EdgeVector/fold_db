//! Configuration management for the logging system
//!
//! This module handles loading and managing logging configuration from TOML files,
//! environment variables, and runtime updates.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main logging configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// General logging settings
    pub general: GeneralConfig,
    /// Output-specific configurations
    pub outputs: OutputsConfig,
    /// Feature-specific log levels
    pub features: HashMap<String, String>,
}

/// General logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Default log level for all modules
    pub default_level: String,
    /// Enable colored output
    pub enable_colors: bool,
    /// Enable correlation IDs for request tracking
    pub enable_correlation_ids: bool,
    /// Maximum correlation ID length
    pub max_correlation_id_length: usize,
}

/// Configuration for all output types
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputsConfig {
    /// Console output configuration
    pub console: ConsoleConfig,
    /// File output configuration
    pub file: FileConfig,
    /// Web streaming output configuration
    pub web: WebConfig,
    /// Structured JSON output configuration
    pub structured: StructuredConfig,
}

/// Console output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleConfig {
    /// Enable console output
    pub enabled: bool,
    /// Log level for console output
    pub level: String,
    /// Enable colors in console output
    pub colors: bool,
    /// Include timestamps
    pub include_timestamp: bool,
    /// Include module path
    pub include_module: bool,
    /// Include thread information
    pub include_thread: bool,
}

/// File output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    /// Enable file output
    pub enabled: bool,
    /// Log file path
    pub path: String,
    /// Log level for file output
    pub level: String,
    /// Maximum file size before rotation (e.g., "10MB")
    pub max_size: String,
    /// Maximum number of log files to keep
    pub max_files: u32,
    /// Include timestamps
    pub include_timestamp: bool,
    /// Include module path
    pub include_module: bool,
    /// Include thread information
    pub include_thread: bool,
}

/// Web streaming output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Enable web streaming output
    pub enabled: bool,
    /// Log level for web output
    pub level: String,
    /// Buffer size for web streaming
    pub buffer_size: usize,
    /// Enable filtering in web interface
    pub enable_filtering: bool,
    /// Maximum number of logs to keep in memory
    pub max_logs: usize,
}

/// Structured JSON output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredConfig {
    /// Enable structured JSON output
    pub enabled: bool,
    /// Log level for structured output
    pub level: String,
    /// Output file path for structured logs
    pub path: Option<String>,
    /// Include additional context fields
    pub include_context: bool,
    /// Include performance metrics
    pub include_metrics: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            outputs: OutputsConfig::default(),
            features: Self::default_features(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_level: "INFO".to_string(),
            enable_colors: true,
            enable_correlation_ids: true,
            max_correlation_id_length: 64,
        }
    }
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "INFO".to_string(),
            colors: true,
            include_timestamp: true,
            include_module: true,
            include_thread: false,
        }
    }
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: "logs/datafold.log".to_string(),
            level: "DEBUG".to_string(),
            max_size: "10MB".to_string(),
            max_files: 5,
            include_timestamp: true,
            include_module: true,
            include_thread: true,
        }
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "INFO".to_string(),
            buffer_size: 1000,
            enable_filtering: true,
            max_logs: 5000,
        }
    }
}

impl Default for StructuredConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level: "DEBUG".to_string(),
            path: Some("logs/datafold-structured.json".to_string()),
            include_context: true,
            include_metrics: false,
        }
    }
}

impl LogConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(ConfigError::Io)?;

        let mut config: LogConfig =
            toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;

        // Apply environment variable overrides
        config.apply_env_overrides()?;

        Ok(config)
    }

    /// Load configuration from environment variables only
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Self::default();
        config.apply_env_overrides()?;
        Ok(config)
    }

    /// Apply environment variable overrides to the configuration
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        // General settings
        if let Ok(level) = std::env::var("DATAFOLD_LOG_LEVEL") {
            self.general.default_level = level;
        }
        if let Ok(colors) = std::env::var("DATAFOLD_LOG_COLORS") {
            self.general.enable_colors = colors.parse().unwrap_or(true);
        }

        // Console settings
        if let Ok(enabled) = std::env::var("DATAFOLD_LOG_CONSOLE_ENABLED") {
            self.outputs.console.enabled = enabled.parse().unwrap_or(true);
        }
        if let Ok(level) = std::env::var("DATAFOLD_LOG_CONSOLE_LEVEL") {
            self.outputs.console.level = level;
        }

        // File settings
        if let Ok(enabled) = std::env::var("DATAFOLD_LOG_FILE_ENABLED") {
            self.outputs.file.enabled = enabled.parse().unwrap_or(false);
        }
        if let Ok(path) = std::env::var("DATAFOLD_LOG_FILE_PATH") {
            self.outputs.file.path = path;
        }
        if let Ok(level) = std::env::var("DATAFOLD_LOG_FILE_LEVEL") {
            self.outputs.file.level = level;
        }

        // Web settings
        if let Ok(enabled) = std::env::var("DATAFOLD_LOG_WEB_ENABLED") {
            self.outputs.web.enabled = enabled.parse().unwrap_or(true);
        }
        if let Ok(level) = std::env::var("DATAFOLD_LOG_WEB_LEVEL") {
            self.outputs.web.level = level;
        }

        // Feature-specific overrides
        for (key, value) in std::env::vars() {
            if let Some(feature) = key.strip_prefix("DATAFOLD_LOG_FEATURE_") {
                let feature_name = feature.to_lowercase();
                self.features.insert(feature_name, value);
            }
        }

        Ok(())
    }

    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(ConfigError::Io)?;
        }

        std::fs::write(path, content).map_err(ConfigError::Io)?;
        Ok(())
    }

    /// Get default feature-specific log levels
    fn default_features() -> HashMap<String, String> {
        let mut features = HashMap::new();
        features.insert("query".to_string(), "INFO".to_string());
        features.insert("mutation".to_string(), "INFO".to_string());
        features.insert("schema".to_string(), "INFO".to_string());
        features.insert("ingestion".to_string(), "INFO".to_string());
        features.insert("transform".to_string(), "DEBUG".to_string());
        features.insert("network".to_string(), "INFO".to_string());
        features.insert("permissions".to_string(), "INFO".to_string());
        features.insert("http_server".to_string(), "DEBUG".to_string());
        features.insert("tcp_server".to_string(), "INFO".to_string());
        features.insert("database".to_string(), "WARN".to_string());
        features
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate log levels
        let valid_levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];

        if !valid_levels.contains(&self.general.default_level.as_str()) {
            return Err(ConfigError::InvalidLevel(
                self.general.default_level.clone(),
            ));
        }

        if !valid_levels.contains(&self.outputs.console.level.as_str()) {
            return Err(ConfigError::InvalidLevel(
                self.outputs.console.level.clone(),
            ));
        }

        if !valid_levels.contains(&self.outputs.file.level.as_str()) {
            return Err(ConfigError::InvalidLevel(self.outputs.file.level.clone()));
        }

        if !valid_levels.contains(&self.outputs.web.level.as_str()) {
            return Err(ConfigError::InvalidLevel(self.outputs.web.level.clone()));
        }

        if !valid_levels.contains(&self.outputs.structured.level.as_str()) {
            return Err(ConfigError::InvalidLevel(
                self.outputs.structured.level.clone(),
            ));
        }

        // Validate feature levels
        for (feature, level) in &self.features {
            if !valid_levels.contains(&level.as_str()) {
                return Err(ConfigError::InvalidFeatureLevel(
                    feature.clone(),
                    level.clone(),
                ));
            }
        }

        // Validate file size format
        if self.outputs.file.enabled {
            self.parse_file_size(&self.outputs.file.max_size)?;
        }

        Ok(())
    }

    /// Parse file size string (e.g., "10MB", "1GB") to bytes
    fn parse_file_size(&self, size_str: &str) -> Result<u64, ConfigError> {
        let size_str = size_str.to_uppercase();

        if let Some(num_str) = size_str.strip_suffix("GB") {
            let num: u64 = num_str
                .parse()
                .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))?;
            Ok(num * 1024 * 1024 * 1024)
        } else if let Some(num_str) = size_str.strip_suffix("MB") {
            let num: u64 = num_str
                .parse()
                .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))?;
            Ok(num * 1024 * 1024)
        } else if let Some(num_str) = size_str.strip_suffix("KB") {
            let num: u64 = num_str
                .parse()
                .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))?;
            Ok(num * 1024)
        } else if let Some(num_str) = size_str.strip_suffix("B") {
            num_str
                .parse()
                .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))
        } else {
            // Default to bytes if no suffix
            size_str
                .parse()
                .map_err(|_| ConfigError::InvalidFileSize(size_str.clone()))
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse configuration: {0}")]
    Parse(String),
    #[error("Failed to serialize configuration: {0}")]
    Serialize(String),
    #[error("Invalid log level: {0}")]
    InvalidLevel(String),
    #[error("Invalid log level for feature '{0}': {1}")]
    InvalidFeatureLevel(String, String),
    #[error("Invalid file size format: {0}")]
    InvalidFileSize(String),
}
