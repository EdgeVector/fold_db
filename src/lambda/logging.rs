//! Logging abstraction for Lambda deployments
//!
//! Provides a trait that users can implement with their choice of backend
//! (DynamoDB, CloudWatch, S3, custom databases, etc.)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Log entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Optional user_id - logger implementation populates this if needed
    pub user_id: Option<String>,
    pub timestamp: i64,
    pub level: LogLevel,
    pub event_type: String,
    pub message: String,
    pub metadata: Option<HashMap<String, String>>,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
/// # Example
///
/// ```ignore
/// use datafold::lambda::{Logger, LogEntry};
/// use async_trait::async_trait;
///
/// pub struct MyLogger {
///     // Your backend
/// }
///
/// #[async_trait]
/// impl Logger for MyLogger {
///     async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///         // Write to your backend
///         Ok(())
///     }
/// }
/// ```
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
/// ```ignore
/// use datafold::lambda::{LambdaConfig, StdoutLogger};
/// use std::sync::Arc;
///
/// let config = LambdaConfig::new()
///     .with_logger(Arc::new(StdoutLogger));
/// ```
pub struct StdoutLogger;

#[async_trait]
impl Logger for StdoutLogger {
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let metadata_str = if let Some(meta) = &entry.metadata {
            format!(" {:?}", meta)
        } else {
            String::new()
        };
        
        let user_id = entry.user_id.as_deref().unwrap_or("system");
        
        eprintln!(
            "[{}] [{}] {} - {}{}",
            user_id,
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
/// ```ignore
/// use datafold::lambda::LambdaContext;
///
/// async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
///     let user_id = event.payload["user_id"].as_str().unwrap_or("anonymous");
///     let logger = LambdaContext::create_logger(user_id)?;
///     
///     logger.info("request_started", "Processing request").await?;
///     // Your business logic...
///     logger.info("request_completed", "Request completed successfully").await?;
///     
///     Ok(json!({ "statusCode": 200 }))
/// }
/// ```
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
    /// ```ignore
    /// use std::collections::HashMap;
    ///
    /// logger.log(
    ///     LogLevel::Info,
    ///     "ingestion_completed",
    ///     "Successfully ingested data",
    ///     Some(HashMap::from([
    ///         ("record_count".to_string(), "100".to_string()),
    ///     ]))
    /// ).await?;
    /// ```
    pub async fn log(
        &self,
        level: LogLevel,
        event_type: &str,
        message: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as i64;
        
        let entry = LogEntry {
            user_id: Some(self.user_id.clone()),
            timestamp,
            level,
            event_type: event_type.to_string(),
            message: message.to_string(),
            metadata,
        };
        
        self.logger.log(entry).await
    }
    
    /// Log info message
    ///
    /// # Example
    ///
    /// ```ignore
    /// logger.info("request_started", "Processing your request").await?;
    /// ```
    pub async fn info(&self, event_type: &str, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Info, event_type, message, None).await
    }
    
    /// Log error message
    ///
    /// # Example
    ///
    /// ```ignore
    /// logger.error("ingestion_failed", "Failed to ingest data").await?;
    /// ```
    pub async fn error(&self, event_type: &str, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Error, event_type, message, None).await
    }
    
    /// Log warning message
    ///
    /// # Example
    ///
    /// ```ignore
    /// logger.warn("schema_mismatch", "Schema validation warning").await?;
    /// ```
    pub async fn warn(&self, event_type: &str, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Warn, event_type, message, None).await
    }
    
    /// Log debug message
    ///
    /// # Example
    ///
    /// ```ignore
    /// logger.debug("cache_hit", "Found in cache").await?;
    /// ```
    pub async fn debug(&self, event_type: &str, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Debug, event_type, message, None).await
    }
    
    /// Log trace message
    ///
    /// # Example
    ///
    /// ```ignore
    /// logger.trace("function_entry", "Entering function").await?;
    /// ```
    pub async fn trace(&self, event_type: &str, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.log(LogLevel::Trace, event_type, message, None).await
    }
}

/// Bridge that forwards Rust's log crate to custom Logger
///
/// This allows all internal datafold logging (using `log::info!()`, etc.)
/// to be captured and sent to your custom logger implementation.
///
/// The logger implementation is responsible for determining the user_id
/// (e.g., via task-local storage in multi-tenant scenarios).
pub struct LogBridge {
    logger: Arc<dyn Logger>,
}

impl LogBridge {
    /// Create a new log bridge
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self { logger }
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
                user_id: None, // Logger implementation populates this
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
                level,
                event_type: record.target().to_string(),
                message: format!("{}", record.args()),
                metadata: None,
            };

            // Fire and forget (don't block on logging)
            let logger = self.logger.clone();
            tokio::spawn(async move {
                let _ = logger.log(entry).await;
            });
        }
    }

    fn flush(&self) {}
}
