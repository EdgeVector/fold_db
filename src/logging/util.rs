//! Utility helpers for the logging system

use crate::logging::LoggingError;

/// Parse a string log level into a [`tracing::Level`].
///
/// Returns an error if the provided level is not one of the
/// standard log level strings (TRACE, DEBUG, INFO, WARN, ERROR).
pub fn parse_log_level(level: &str) -> Result<tracing::level_filters::LevelFilter, LoggingError> {
    match level {
        "TRACE" => Ok(tracing::level_filters::LevelFilter::TRACE),
        "DEBUG" => Ok(tracing::level_filters::LevelFilter::DEBUG),
        "INFO" => Ok(tracing::level_filters::LevelFilter::INFO),
        "WARN" => Ok(tracing::level_filters::LevelFilter::WARN),
        "ERROR" => Ok(tracing::level_filters::LevelFilter::ERROR),
        _ => Err(LoggingError::Config(format!(
            "Invalid log level: {}",
            level
        ))),
    }
}
