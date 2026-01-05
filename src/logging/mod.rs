//! # Enhanced Logging System
//!
//! This module provides enhanced logging capabilities for the datafold project.
//! It extends the existing web_logger with configuration management and
//! feature-specific logging support.

pub mod config;
pub mod core;
pub mod features;
pub mod outputs;
pub mod util;

#[cfg(feature = "aws-backend")]
use crate::logging::core::LogBridge;
#[cfg(feature = "aws-backend")]
use crate::logging::outputs::dynamodb::DynamoDbLogger;
use config::LogConfig;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global logging configuration instance
static LOGGING_CONFIG: OnceCell<Arc<RwLock<LogConfig>>> = OnceCell::new();

/// Global logger instance for querying
static GLOBAL_LOGGER: OnceCell<Arc<dyn crate::logging::core::Logger>> = OnceCell::new();

/// Multi-logger implementation that broadcasts logs to multiple backends
struct MultiLogger {
    loggers: Vec<Box<dyn log::Log>>,
}

impl MultiLogger {
    fn new(loggers: Vec<Box<dyn log::Log>>) -> Self {
        Self { loggers }
    }
}

impl log::Log for MultiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.loggers.iter().any(|l| l.enabled(metadata))
    }

    fn log(&self, record: &log::Record) {
        for logger in &self.loggers {
            logger.log(record);
        }
    }

    fn flush(&self) {
        for logger in &self.loggers {
            logger.flush();
        }
    }
}

/// Enhanced logging system that works alongside the existing web_logger
pub struct LoggingSystem;

impl LoggingSystem {
    /// Initialize the logging system with default configuration
    pub async fn init_default() -> Result<(), LoggingError> {
        let config = LogConfig::default();
        Self::init_with_config(config).await
    }

    /// Initialize logging with explicit DynamoDB configuration
    ///
    /// This is the preferred method for Lambda/serverless environments where
    /// configuration should be derived from the node's database config rather
    /// than environment variables.
    ///
    /// # Arguments
    /// * `dynamo_config` - Optional (table_name, region) tuple. If provided,
    ///                     DynamoDB logging will be automatically enabled.
    pub async fn init_with_dynamodb(
        dynamo_config: Option<(String, String)>,
    ) -> Result<(), LoggingError> {
        #[allow(unused_mut)]
        let mut config = LogConfig::default();

        // Enable DynamoDB logging if config is provided
        #[cfg(feature = "aws-backend")]
        if let Some((table_name, region)) = dynamo_config {
            config.outputs.dynamodb.enabled = true;
            config.outputs.dynamodb.table_name = table_name;
            config.outputs.dynamodb.region = Some(region);
        }

        #[cfg(not(feature = "aws-backend"))]
        let _ = dynamo_config; // Suppress unused variable warning

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

        // Also set the web logger level dynamically
        crate::web_logger::set_log_level(level_filter);

        // Store configuration globally
        let config_arc = Arc::new(RwLock::new(config.clone()));
        LOGGING_CONFIG
            .set(config_arc.clone())
            .map_err(|_| LoggingError::AlreadyInitialized)?;

        // Prepare loggers
        let mut loggers: Vec<Box<dyn log::Log>> = Vec::new();

        // 1. Web Logger (always included for now, or configurable?)
        if config.outputs.web.enabled {
            loggers.push(Box::new(crate::web_logger::get_instance()));
        }

        // 2. DynamoDB Logger
        #[cfg(feature = "aws-backend")]
        if config.outputs.dynamodb.enabled {
            let table_name = config.outputs.dynamodb.table_name.clone();

            // Create DynamoDB logger
            let dynamodb_logger = if let Some(region) = &config.outputs.dynamodb.region {
                let region_provider = aws_config::Region::new(region.clone());
                let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(region_provider)
                    .load()
                    .await;
                DynamoDbLogger::with_config(table_name, &sdk_config).await
            } else {
                DynamoDbLogger::new(table_name).await
            };

            // Bridge it to log::Log
            let logger_arc = Arc::new(dynamodb_logger);
            let bridge = LogBridge::new(logger_arc.clone());
            loggers.push(Box::new(bridge));

            // Set global logger for querying
            let _ = GLOBAL_LOGGER.set(logger_arc);
        }

        // 3. File Logger (Placeholder logic, assuming FileOutput implements Log or similar?)
        // Currently FileOutput is independent or not fully wired as log::Log.
        // For this task, we focus on DynamoDB.

        // Initialize MultiLogger
        let multi_logger = MultiLogger::new(loggers);
        log::set_logger(Box::leak(Box::new(multi_logger)))
            .map_err(|_| LoggingError::AlreadyInitialized)?;

        log::set_max_level(level_filter);

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

    /// Query logs with pagination support
    pub async fn query_logs(
        limit: Option<usize>,
        from_timestamp: Option<i64>,
    ) -> Vec<crate::logging::core::LogEntry> {
        if let Some(logger) = GLOBAL_LOGGER.get() {
            if let Ok(entries) = logger.query("anonymous", limit, from_timestamp).await {
                // DynamoDB logger returns most recent first, so we reverse to get chronological
                return entries.into_iter().rev().collect();
            }
        }

        // Fallback to in-memory web logger
        let raw_entries = crate::web_logger::get_entries();
        let from_ts = from_timestamp.unwrap_or(0);

        raw_entries
            .into_iter()
            .filter(|e| e.timestamp > from_ts)
            .map(|e| {
                let level = match e.level.as_str() {
                    "TRACE" => crate::logging::core::LogLevel::Trace,
                    "DEBUG" => crate::logging::core::LogLevel::Debug,
                    "INFO" => crate::logging::core::LogLevel::Info,
                    "WARN" => crate::logging::core::LogLevel::Warn,
                    "ERROR" => crate::logging::core::LogLevel::Error,
                    _ => crate::logging::core::LogLevel::Info,
                };

                crate::logging::core::LogEntry {
                    timestamp: e.timestamp,
                    level,
                    event_type: "web_logger".to_string(),
                    message: e.message,
                    user_id: None,
                    metadata: None,
                }
            })
            .collect()
    }

    /// Query recent logs from the active backend (legacy support)
    pub async fn query_recent_logs(limit: usize) -> Vec<String> {
        let entries = Self::query_logs(Some(limit), None).await;
        entries
            .into_iter()
            .map(|entry| format!("{} - {}", entry.level.as_str(), entry.message))
            .collect()
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
/// This calls init_default() but blocks on async execution
pub fn init() -> Result<(), LoggingError> {
    // We cannot easily run async here without a runtime.
    // This is for backward compatibility where sync initialization was expected.
    // However, DynamoDB requires async.
    // We'll just fallback to web_logger::init() if we can't do async properties
    crate::web_logger::init().map_err(|_| LoggingError::AlreadyInitialized)
}
