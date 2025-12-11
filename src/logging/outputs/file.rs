//! File output handler with rotation support

use crate::logging::config::FileConfig;
use crate::logging::util::parse_log_level;
use crate::logging::LoggingError;
use tracing_subscriber::fmt;

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

        let (_non_blocking, guard) = non_blocking(file_appender);

        Ok(Self {
            config: config.clone(),
            _guard: guard,
        })
    }

    /// Create a tracing layer for file output
    pub fn create_layer(&self) -> Result<Box<dyn Layer<Registry> + Send + Sync>, LoggingError> {
        let mut layer = fmt::Layer::default()
            .with_ansi(false); // No colors in file output

        if self.config.include_module {
            layer = layer.with_target(true);
        } else {
            layer = layer.with_target(false);
        }

        if self.config.include_thread {
            layer = layer.with_thread_ids(true).with_thread_names(true);
        }

        if !self.config.include_timestamp {
            let layer = layer.without_time()
                .with_filter(parse_log_level(&self.config.level)?);
            Ok(Box::new(layer))
        } else {
            let layer = layer
                .with_filter(parse_log_level(&self.config.level)?);
            Ok(Box::new(layer))
        }
    }

    /// Parse file size string to bytes
    fn parse_file_size(size_str: &str) -> Result<u64, LoggingError> {
        let size_str = size_str.to_uppercase();
        
        if let Some(num_str) = size_str.strip_suffix("GB") {
            let num: u64 = num_str.parse()
                .map_err(|_| LoggingError::Config(format!("Invalid file size: {}", size_str)))?;
            Ok(num * 1024 * 1024 * 1024)
        } else if let Some(num_str) = size_str.strip_suffix("MB") {
            let num: u64 = num_str.parse()
                .map_err(|_| LoggingError::Config(format!("Invalid file size: {}", size_str)))?;
            Ok(num * 1024 * 1024)
        } else if let Some(num_str) = size_str.strip_suffix("KB") {
            let num: u64 = num_str.parse()
                .map_err(|_| LoggingError::Config(format!("Invalid file size: {}", size_str)))?;
            Ok(num * 1024)
        } else if let Some(num_str) = size_str.strip_suffix("B") {
            num_str.parse()
                .map_err(|_| LoggingError::Config(format!("Invalid file size: {}", size_str)))
        } else {
            // Default to bytes if no suffix
            size_str.parse()
                .map_err(|_| LoggingError::Config(format!("Invalid file size: {}", size_str)))
        }
    }
}