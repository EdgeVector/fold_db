//! Utility helpers for the logging system

use crate::logging::LoggingError;

/// Parse a string log level into a [`log::LevelFilter`].
///
/// Returns an error if the provided level is not one of the
/// standard log level strings (TRACE, DEBUG, INFO, WARN, ERROR).
pub fn parse_log_level(level: &str) -> Result<log::LevelFilter, LoggingError> {
    match level {
        "TRACE" => Ok(log::LevelFilter::Trace),
        "DEBUG" => Ok(log::LevelFilter::Debug),
        "INFO" => Ok(log::LevelFilter::Info),
        "WARN" => Ok(log::LevelFilter::Warn),
        "ERROR" => Ok(log::LevelFilter::Error),
        _ => Err(LoggingError::Config(format!(
            "Invalid log level: {}",
            level
        ))),
    }
}
