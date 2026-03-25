//! # Enhanced Logging System
//!
//! This module provides enhanced logging capabilities for the FoldDB project.
//! It extends the existing web_logger with configuration management and
//! feature-specific logging support.

pub mod config;
pub mod core;
pub mod features;
pub mod outputs;

use crate::logging::core::{LogBridge, Logger, MultiAsyncLogger};
#[cfg(feature = "aws-backend")]
use crate::logging::outputs::dynamodb::DynamoDbLogger;
use crate::logging::outputs::web::WebOutput;
use config::LogConfig;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global logging configuration instance
static LOGGING_CONFIG: OnceCell<Arc<RwLock<LogConfig>>> = OnceCell::new();

/// Global logger instance for querying
static GLOBAL_LOGGER: OnceCell<Arc<dyn crate::logging::core::Logger>> = OnceCell::new();

/// Global WebOutput instance for streaming
static GLOBAL_WEB_OUTPUT: OnceCell<Arc<WebOutput>> = OnceCell::new();

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

/// Enhanced logging system
pub struct LoggingSystem;

impl LoggingSystem {
    /// Initialize the logging system with default configuration
    pub async fn init_default() -> Result<(), LoggingError> {
        let config = LogConfig::default();
        Self::init_with_config(config).await
    }

    /// Initialize logging with explicit Cloud configuration
    pub async fn init_with_cloud(
        dynamo_config: Option<(String, String, Option<String>)>,
    ) -> Result<(), LoggingError> {
        #[allow(unused_mut)]
        let mut config = LogConfig::default();

        // Enable DynamoDB logging if config is provided
        #[cfg(feature = "aws-backend")]
        if let Some((table_name, region, user_id)) = dynamo_config {
            config.outputs.dynamodb.enabled = true;
            config.outputs.dynamodb.table_name = table_name;
            config.outputs.dynamodb.region = Some(region);

            // Set default user ID for the logger if provided
            if let Some(uid) = user_id {
                config.general.app_id = Some(uid);
            }
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

        // Store configuration globally
        let config_arc = Arc::new(RwLock::new(config.clone()));
        LOGGING_CONFIG
            .set(config_arc.clone())
            .map_err(|_| LoggingError::AlreadyInitialized)?;

        // Prepare sync loggers
        let mut sync_loggers: Vec<Box<dyn log::Log>> = Vec::new();

        // Prepare async loggers
        let mut async_loggers: Vec<Arc<dyn crate::logging::core::Logger>> = Vec::new();

        // 1. Web Logger
        if config.outputs.web.enabled {
            match WebOutput::new(&config.outputs.web) {
                Ok(web_output) => {
                    let web_arc = Arc::new(web_output);
                    let _ = GLOBAL_WEB_OUTPUT.set(web_arc.clone());
                    async_loggers.push(web_arc);
                }
                Err(e) => eprintln!("Failed to initialize web logger: {}", e),
            }
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

            async_loggers.push(Arc::new(dynamodb_logger));
        }

        // Connect async loggers via MultiAsyncLogger and LogBridge if any exist
        if !async_loggers.is_empty() {
            let multi_async = MultiAsyncLogger::new(async_loggers);
            let multi_arc = Arc::new(multi_async);

            // Set as global logger for querying
            let _ = GLOBAL_LOGGER.set(multi_arc.clone());

            // Bridge to sync world
            let bridge = LogBridge::new(multi_arc);
            sync_loggers.push(Box::new(bridge));
        }

        // 3. Console Logger
        if config.outputs.console.enabled {
            match outputs::ConsoleOutput::new(&config.outputs.console) {
                Ok(logger) => sync_loggers.push(Box::new(logger)),
                Err(e) => eprintln!("Failed to initialize console logger: {}", e),
            }
        }

        // 4. File Logger
        if config.outputs.file.enabled {
            match outputs::FileOutput::new(&config.outputs.file) {
                Ok(logger) => sync_loggers.push(Box::new(logger)),
                Err(e) => eprintln!("Failed to initialize file logger: {}", e),
            }
        }

        // 5. Structured Logger
        if config.outputs.structured.enabled {
            match outputs::StructuredOutput::new(&config.outputs.structured) {
                Ok(logger) => sync_loggers.push(Box::new(logger)),
                Err(e) => eprintln!("Failed to initialize structured logger: {}", e),
            }
        }

        // Initialize MultiLogger
        let multi_logger = MultiLogger::new(sync_loggers);
        log::set_logger(Box::leak(Box::new(multi_logger)))
            .map_err(|_| LoggingError::AlreadyInitialized)?;

        log::set_max_level(level_filter);

        Ok(())
    }

    /// Initialize logging with automatic fallback on failure.
    ///
    /// Attempts cloud logging first (if config provided), falls back to default
    /// logging on error. Silently succeeds if logging is already initialized.
    pub async fn init_with_fallback(cloud_config: Option<(String, String, Option<String>)>) {
        match Self::init_with_cloud(cloud_config).await {
            Ok(_) => {}
            Err(LoggingError::AlreadyInitialized) => {}
            Err(e) => {
                eprintln!("Cloud logging init failed ({}), using default", e);
                let _ = Self::init_default().await;
            }
        }
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
    ) -> Result<Vec<crate::logging::core::LogEntry>, LoggingError> {
        // Try to get user_id from config, but allow fallback for WebOutput which doesn't need it
        let user_id = if let Some(config_arc) = LOGGING_CONFIG.get() {
            config_arc.read().await.general.app_id.clone()
        } else {
            None
        };

        // If we have a user_id, try GLOBAL_LOGGER first (e.g., DynamoDB)
        if let Some(uid) = &user_id {
            if let Some(logger) = GLOBAL_LOGGER.get() {
                if let Ok(entries) = logger.query(uid, limit, from_timestamp).await {
                    if !entries.is_empty() {
                        return Ok(entries);
                    }
                }
            }
        }

        // Fallback to GLOBAL_WEB_OUTPUT which stores logs in memory
        // Note: WebOutput doesn't actually use user_id for log storage - it's an in-memory buffer.
        // The "unknown" value is just a placeholder for the Logger trait interface.
        // This is NOT user data isolation - just internal logging.
        if let Some(web_output) = GLOBAL_WEB_OUTPUT.get() {
            let dummy_user = user_id.as_deref().unwrap_or("unknown");
            if let Ok(entries) = web_output.query(dummy_user, limit, from_timestamp).await {
                return Ok(entries);
            }
        }

        Ok(Vec::new())
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

/// Convenience function to subscribe to web logs (backward compatibility)
pub fn subscribe() -> Option<tokio::sync::broadcast::Receiver<String>> {
    GLOBAL_WEB_OUTPUT
        .get()
        .map(|web_output| web_output.subscribe())
}
