//! # Enhanced Logging System
//!
//! This module provides enhanced logging capabilities for the datafold project.
//! It extends the existing web_logger with configuration management and
//! feature-specific logging support.

pub mod config;
pub mod features;
pub mod util;
pub mod outputs;

use config::LogConfig;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global logging configuration instance
static LOGGING_CONFIG: OnceCell<Arc<RwLock<LogConfig>>> = OnceCell::new();

/// Enhanced logging system that works alongside the existing web_logger
pub struct LoggingSystem;

impl LoggingSystem {
    /// Initialize the logging system with default configuration
    pub async fn init_default() -> Result<(), LoggingError> {
        let config = LogConfig::default();
        Self::init_with_config(config).await
    }

    /// Initialize the logging system with a custom configuration
    pub async fn init_with_config(config: LogConfig) -> Result<(), LoggingError> {
        // Set up global log level based on configuration
        let level_filter = match config.general.default_level.as_str() {
            "TRACE" => log::LevelFilter::Trace,
            "DEBUG" => log::LevelFilter::Debug,
            "INFO" => log::LevelFilter::Info,
            "WARN" => log::LevelFilter::Warn,
            "ERROR" => log::LevelFilter::Error,
            _ => log::LevelFilter::Info,
        };
        log::set_max_level(level_filter);
        
        // Also set the web logger level dynamically
        crate::web_logger::set_log_level(level_filter);

        // Store configuration globally
        let config_arc = Arc::new(RwLock::new(config));
        LOGGING_CONFIG
            .set(config_arc.clone())
            .map_err(|_| LoggingError::AlreadyInitialized)?;

        // Web logger is already initialized in main, no need to reinitialize

        Ok(())
    }

    /// Get the global logging configuration
    pub async fn get_config() -> Option<LogConfig> {
        if let Some(config_arc) = LOGGING_CONFIG.get() {
            let config_guard = config_arc.read().await;
            Some(config_guard.clone())
        } else {
            None
        }
    }

    /// Update feature-specific log level
    pub async fn update_feature_level(feature: &str, level: &str) -> Result<(), LoggingError> {
        if let Some(config_arc) = LOGGING_CONFIG.get() {
            let mut config_guard = config_arc.write().await;
            config_guard
                .features
                .insert(feature.to_string(), level.to_string());

            // Update global log level if this affects the general level
            let level_filter = match level {
                "TRACE" => log::LevelFilter::Trace,
                "DEBUG" => log::LevelFilter::Debug,
                "INFO" => log::LevelFilter::Info,
                "WARN" => log::LevelFilter::Warn,
                "ERROR" => log::LevelFilter::Error,
                _ => {
                    return Err(LoggingError::Config(format!(
                        "Invalid log level: {}",
                        level
                    )))
                }
            };
            log::set_max_level(level_filter);

            Ok(())
        } else {
            Err(LoggingError::Config(
                "Logging system not initialized".to_string(),
            ))
        }
    }

    /// Get available features and their current levels
    pub async fn get_features() -> Option<std::collections::HashMap<String, String>> {
        if let Some(config_arc) = LOGGING_CONFIG.get() {
            let config_guard = config_arc.read().await;
            Some(config_guard.features.clone())
        } else {
            None
        }
    }

    /// Reload configuration from file
    pub async fn reload_config_from_file(path: &str) -> Result<(), LoggingError> {
        let new_config = LogConfig::from_file(path)
            .map_err(|e| LoggingError::Config(format!("Failed to load config: {}", e)))?;

        if let Some(config_arc) = LOGGING_CONFIG.get() {
            let mut config_guard = config_arc.write().await;
            *config_guard = new_config;
            Ok(())
        } else {
            Err(LoggingError::Config(
                "Logging system not initialized".to_string(),
            ))
        }
    }
}

/// Logging system errors
#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("Logging system already initialized")]
    AlreadyInitialized,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Config error: {0}")]
    ConfigError(#[from] crate::logging::config::ConfigError),
}

/// Convenience function to get web logs (backward compatibility)
pub fn get_logs() -> Vec<String> {
    crate::web_logger::get_logs()
}

/// Convenience function to subscribe to web logs (backward compatibility)
pub fn subscribe() -> Option<tokio::sync::broadcast::Receiver<String>> {
    crate::web_logger::subscribe()
}

/// Initialize logging with backward compatibility
pub fn init() -> Result<(), log::SetLoggerError> {
    crate::web_logger::init()
}
