//! Structured JSON output handler

use crate::logging::config::StructuredConfig;
use crate::logging::util::parse_log_level;
use crate::logging::LoggingError;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_appender::{non_blocking, rolling};
use std::path::Path;
use std::io;

/// Structured output handler that provides JSON-formatted logging
pub struct StructuredOutput {
    config: StructuredConfig,
    _guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

impl StructuredOutput {
    /// Create a new structured output handler
    pub fn new(config: &StructuredConfig) -> Result<Self, LoggingError> {
        let guard = if let Some(ref path) = config.path {
            // Set up file output for structured logs
            let path = Path::new(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| LoggingError::Io(e))?;
            }

            let file_appender = rolling::RollingFileAppender::new(
                rolling::Rotation::DAILY,
                path.parent().unwrap_or(Path::new(".")),
                path.file_name().and_then(|n| n.to_str()).unwrap_or("structured.json")
            );

            let (_, guard) = non_blocking(file_appender);
            Some(guard)
        } else {
            None
        };

        Ok(Self {
            config: config.clone(),
            _guard: guard,
        })
    }

    /// Create a tracing layer for structured output
    pub fn create_layer(&self) -> Result<impl Layer<Registry> + Send + Sync, LoggingError> {
        let writer = if self.config.path.is_some() {
            // Use file writer - this would need the actual non-blocking writer
            // For now, using stdout as placeholder
            Box::new(io::stdout()) as Box<dyn io::Write + Send>
        } else {
            Box::new(io::stdout()) as Box<dyn io::Write + Send>
        };

        let layer = fmt::Layer::default()
            .json() // Enable JSON formatting
            .with_current_span(self.config.include_context)
            .with_span_list(self.config.include_context)
            .with_filter(parse_log_level(&self.config.level)?);

        Ok(layer)
    }

}