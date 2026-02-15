//! Logging abstraction for Lambda deployments
//!
//! Provides a trait that users can implement with their choice of backend
//! (DynamoDB, CloudWatch, S3, custom databases, etc.)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

tokio::task_local! {
    /// Task-local storage for the current user ID
    /// This allows implicit propagation of user context through the async call stack.
    static CURRENT_USER_ID: String;
}

/// Execute a future within the context of a specific user.
///
/// Any logs generated within this future (including standard `log::*` macros)
/// will automatically have the user_id attached.
///
/// # Example
///
/// /// Example
///
/// (Example removed as it was ignored)
pub async fn run_with_user<F>(user_id: &str, f: F) -> F::Output
where
    F: Future,
{
    CURRENT_USER_ID.scope(user_id.to_string(), f).await
}

/// Get the current user ID from task-local storage, if set.
pub fn get_current_user_id() -> Option<String> {
    CURRENT_USER_ID.try_with(|id| id.clone()).ok()
}

/// Log entry structure
///
/// Note: user_id is not stored in LogEntry. The Logger implementation
/// manages user_id internally based on how it was initialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: i64,
    pub level: LogLevel,
    pub event_type: String,
    pub message: String,
    pub user_id: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Trait for logging implementations
///
/// Users implement this trait with their choice of backend.
///
/// # Multi-Tenant Pattern
///
/// For multi-tenant deployments, create a logger instance per request with the user_id:
///
/// /// Example
///
/// (Example removed as it was ignored)
///
/// See `examples/lambda_dynamodb_logger.rs` for a complete implementation.
#[async_trait]
pub trait Logger: Send + Sync {
    /// Log an event
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Query logs for a user (optional - not all backends support this)
    ///
    /// Default implementation returns empty vector for write-only loggers.
    async fn query(
        &self,
        user_id: &str,
        limit: Option<usize>,
        from_timestamp: Option<i64>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let _ = (user_id, limit, from_timestamp);
        Ok(vec![])
    }
}

/// No-op logger (default)
///
/// Use this when you don't need logging or want to disable it.
pub struct NoOpLogger;

impl Default for NoOpLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl NoOpLogger {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Logger for NoOpLogger {
    async fn log(&self, _entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

/// Stdout logger (for development/debugging)
///
/// Logs to stderr in a structured format.
///
/// # Example
///
/// /// Example
///
/// (Example removed as it was ignored)
pub struct StdoutLogger;

impl Default for StdoutLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl StdoutLogger {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Logger for StdoutLogger {
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let metadata_str = if let Some(meta) = &entry.metadata {
            format!(" {:?}", meta)
        } else {
            String::new()
        };

        eprintln!(
            "[{}] [{}] - {}{}",
            entry.level.as_str(),
            entry.event_type,
            entry.message,
            metadata_str
        );
        Ok(())
    }
}

/// User-scoped logger wrapper
///
/// Provides convenience methods for logging with a specific user_id.
/// Automatically sets the user_id in all log entries.
///
/// # Example
///
/// /// Example
///
/// (Example removed as it was ignored)
pub struct UserLogger {
    user_id: String,
    logger: Arc<dyn Logger>,
}

impl UserLogger {
    /// Create a new user-scoped logger
    pub fn new(user_id: String, logger: Arc<dyn Logger>) -> Self {
        Self { user_id, logger }
    }

    /// Get the user_id for this logger
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// Log with custom level and metadata
    ///
    /// # Example
    ///
    /// /// Example
    ///
    /// (Example removed as it was ignored)
    pub async fn log(
        &self,
        level: LogLevel,
        event_type: &str,
        message: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64;

        let entry = LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp,
            level,
            event_type: event_type.to_string(),
            message: message.to_string(),
            user_id: Some(self.user_id.clone()),
            metadata,
        };

        self.logger.log(entry).await
    }

    /// Log info message
    ///
    /// # Example
    ///
    /// /// Example
    ///
    /// (Example removed as it was ignored)
    pub async fn info(
        &self,
        event_type: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Info, event_type, message, None).await
    }

    /// Log error message
    ///
    /// # Example
    ///
    /// /// Example
    ///
    /// (Example removed as it was ignored)
    pub async fn error(
        &self,
        event_type: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Error, event_type, message, None).await
    }

    /// Log warning message
    ///
    /// # Example
    ///
    /// /// Example
    ///
    /// (Example removed as it was ignored)
    pub async fn warn(
        &self,
        event_type: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Warn, event_type, message, None).await
    }

    /// Log debug message
    ///
    /// # Example
    ///
    /// /// Example
    ///
    /// (Example removed as it was ignored)
    pub async fn debug(
        &self,
        event_type: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Debug, event_type, message, None).await
    }

    /// Log trace message
    ///
    /// # Example
    ///
    /// /// Example
    ///
    /// (Example removed as it was ignored)
    pub async fn trace(
        &self,
        event_type: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Trace, event_type, message, None).await
    }
}

/// Multi-logger implementation that broadcasts logs to multiple async loggers
pub struct MultiAsyncLogger {
    loggers: Vec<Arc<dyn Logger>>,
}

impl MultiAsyncLogger {
    pub fn new(loggers: Vec<Arc<dyn Logger>>) -> Self {
        Self { loggers }
    }
}

#[async_trait]
impl Logger for MultiAsyncLogger {
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for logger in &self.loggers {
            // We clone the entry for each logger since they need ownership or a copy
            // LogEntry is Clone, so this is fine.
            if let Err(e) = logger.log(entry.clone()).await {
                // We don't want to fail everything if one logger fails, but maybe we should log it?
                // For now, silently continue or print to stderr
                eprintln!("Error in MultiAsyncLogger: {}", e);
            }
        }
        Ok(())
    }

    async fn query(
        &self,
        user_id: &str,
        limit: Option<usize>,
        from_timestamp: Option<i64>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        // Query the first logger that supports querying?
        // Or aggregate?
        // For now, let's just query the first one that returns results, or just the first one.
        // Typically, we only have one "storage" logger (DynamoDB) and one "stream" logger (Web).
        // Web might hold recent logs in memory.

        for logger in &self.loggers {
            // Try to query
            match logger.query(user_id, limit, from_timestamp).await {
                Ok(logs) if !logs.is_empty() => return Ok(logs),
                Ok(_) => continue, // Try next logger
                Err(_) => continue,
            }
        }
        Ok(vec![])
    }
}

/// Bridge that forwards Rust's log crate to custom Logger
///
/// This allows all internal FoldDB logging (using `log::info!()`, etc.)
/// to be captured and sent to your custom logger implementation.
///
/// **Note**: LogEntry does not contain user_id. Your logger implementation
/// should use its own user_id field (set during logger initialization).
///
/// Bridge that forwards Rust's log crate to custom Logger
///
/// This allows all internal FoldDB logging (using `log::info!()`, etc.)
/// to be captured and sent to your custom logger implementation.
///
/// **Note**: LogEntry does not contain user_id. Your logger implementation
/// should use its own user_id field (set during logger initialization).
///
/// See `examples/lambda_dynamodb_logger.rs` for the recommended pattern.
pub struct LogBridge {
    logger: Arc<dyn Logger>,
    handle: tokio::runtime::Handle,
}

impl LogBridge {
    /// Create a new log bridge
    /// Must be called from within a Tokio runtime
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            handle: tokio::runtime::Handle::current(),
        }
    }
}

impl log::Log for LogBridge {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level = match record.level() {
                log::Level::Error => LogLevel::Error,
                log::Level::Warn => LogLevel::Warn,
                log::Level::Info => LogLevel::Info,
                log::Level::Debug => LogLevel::Debug,
                log::Level::Trace => LogLevel::Trace,
            };

            let entry = LogEntry {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
                level,
                event_type: record.target().to_string(),
                message: record.args().to_string(),
                user_id: get_current_user_id(),
                metadata: None,
            };

            // Fire and forget using the captured handle
            let logger = self.logger.clone();
            self.handle.spawn(async move {
                let _ = logger.log(entry).await;
            });
        }
    }

    fn flush(&self) {}
}
