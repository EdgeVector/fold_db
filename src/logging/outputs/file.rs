//! File output handler with rotation support

use crate::logging::config::FileConfig;
use crate::logging::utils::parse_file_size;
use crate::logging::LoggingError;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_appender::{non_blocking, rolling};
use std::path::Path;

/// File output handler that provides file logging with rotation
pub struct FileOutput {
    config: FileConfig,
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

impl FileOutput {
    /// Create a new file output handler
    pub async fn new(config: &FileConfig) -> Result<Self, LoggingError> {
        // Create parent directories if they don't exist
        let path = Path::new(&config.path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| LoggingError::Io(e))?;
        }

        // Validate configured max file size
        parse_file_size(&config.max_size)?;

        // Set up file appender with rotation
        let file_appender = if config.max_files > 1 {
            // Use rolling file appender
            let directory = path.parent().unwrap_or(Path::new("."));
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("datafold.log");
            
            rolling::RollingFileAppender::new(
                rolling::Rotation::DAILY,
                directory,
                filename
            )
        } else {
            // Use simple file appender
            rolling::RollingFileAppender::new(
                rolling::Rotation::NEVER,
                path.parent().unwrap_or(Path::new(".")),
                path.file_name().and_then(|n| n.to_str()).unwrap_or("datafold.log")
            )
        };

        let (non_blocking, guard) = non_blocking(file_appender);

        Ok(Self {
            config: config.clone(),
            _guard: guard,
        })
    }

    /// Create a tracing layer for file output
    pub fn create_layer(&self) -> Result<impl Layer<Registry> + Send + Sync, LoggingError> {
        let mut layer = fmt::Layer::default()
            .with_ansi(false) // No colors in file output
            .with_filter(self.parse_level_filter()?);

        if !self.config.include_timestamp {
            layer = layer.without_time();
        }

        if self.config.include_module {
            layer = layer.with_target(true);
        } else {
            layer = layer.with_target(false);
        }

        if self.config.include_thread {
            layer = layer.with_thread_ids(true).with_thread_names(true);
        }

        Ok(layer)
    }

    /// Parse the log level filter from configuration
    fn parse_level_filter(&self) -> Result<tracing::Level, LoggingError> {
        match self.config.level.as_str() {
            "TRACE" => Ok(tracing::Level::TRACE),
            "DEBUG" => Ok(tracing::Level::DEBUG),
            "INFO" => Ok(tracing::Level::INFO),
            "WARN" => Ok(tracing::Level::WARN),
            "ERROR" => Ok(tracing::Level::ERROR),
            _ => Err(LoggingError::Config(format!("Invalid log level: {}", self.config.level))),
        }
    }

}