//! Utility helpers for the logging system

use crate::logging::LoggingError;

/// Parse a string log level into a [`tracing::Level`].
///
/// Returns an error if the provided level is not one of the
/// standard log level strings (TRACE, DEBUG, INFO, WARN, ERROR).
pub fn parse_log_level(level: &str) -> Result<tracing::Level, LoggingError> {
    match level {
        "TRACE" => Ok(tracing::Level::TRACE),
        "DEBUG" => Ok(tracing::Level::DEBUG),
        "INFO" => Ok(tracing::Level::INFO),
        "WARN" => Ok(tracing::Level::WARN),
        "ERROR" => Ok(tracing::Level::ERROR),
        _ => Err(LoggingError::Config(format!(
            "Invalid log level: {}",
            level
        ))),
    }
}
