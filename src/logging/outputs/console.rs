//! Console output handler with color support

use crate::logging::config::ConsoleConfig;
use crate::logging::util::parse_log_level;
use crate::logging::LoggingError;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use std::io;

/// Console output handler that provides colored terminal output
pub struct ConsoleOutput {
    config: ConsoleConfig,
}

impl ConsoleOutput {
    /// Create a new console output handler
    pub fn new(config: &ConsoleConfig) -> Result<Self, LoggingError> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// Create a tracing layer for console output
    pub fn create_layer(&self) -> Result<impl Layer<Registry> + Send + Sync, LoggingError> {
        let mut layer = fmt::Layer::default()
            .with_writer(io::stdout)
            .with_filter(parse_log_level(&self.config.level)?);

        // Configure formatting based on config
        if self.config.colors {
            layer = layer.with_ansi(true);
        } else {
            layer = layer.with_ansi(false);
        }

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

}