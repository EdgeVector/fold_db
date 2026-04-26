//! Web streaming output handler
use crate::logging::config::WebConfig;
use crate::logging::core::{LogEntry, LogLevel, Logger};
use crate::logging::LoggingError;
use async_trait::async_trait;
use log::LevelFilter;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast;

/// Web output handler that provides streaming logs via broadcast channel
pub struct WebOutput {
    config: WebConfig,
    buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    sender: broadcast::Sender<String>, // Broadcasts JSON string of LogEntry
    level_filter: RwLock<LevelFilter>,
}

impl WebOutput {
    pub fn new(config: &WebConfig) -> Result<Self, LoggingError> {
        let (sender, _) = broadcast::channel(config.buffer_size);

        let filter = match config.level.as_str() {
            "TRACE" => LevelFilter::Trace,
            "DEBUG" => LevelFilter::Debug,
            "INFO" => LevelFilter::Info,
            "WARN" => LevelFilter::Warn,
            "ERROR" => LevelFilter::Error,
            _ => LevelFilter::Info,
        };

        Ok(Self {
            config: config.clone(),
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_logs))),
            sender,
            level_filter: RwLock::new(filter),
        })
    }

    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.buffer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    fn should_log(&self, level: LogLevel) -> bool {
        if !self.config.enabled {
            return false;
        }

        if let Ok(filter) = self.level_filter.read() {
            let log_level = match level {
                LogLevel::Trace => log::Level::Trace,
                LogLevel::Debug => log::Level::Debug,
                LogLevel::Info => log::Level::Info,
                LogLevel::Warn => log::Level::Warn,
                LogLevel::Error => log::Level::Error,
            };
            log_level <= *filter
        } else {
            false
        }
    }
}

#[async_trait]
impl Logger for WebOutput {
    async fn log(&self, entry: LogEntry) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.should_log(entry.level.clone()) {
            // Add to buffer
            if let Ok(mut buffer) = self.buffer.lock() {
                buffer.push_back(entry.clone());
                if buffer.len() > self.config.max_logs {
                    buffer.pop_front();
                }
            }

            // Send to broadcast channel
            // Serialize LogEntry to JSON string
            if let Ok(msg) = serde_json::to_string(&entry) {
                let _ = self.sender.send(msg);
            }
        }
        Ok(())
    }

    async fn query(
        &self,
        _user_id: &str,
        _limit: Option<usize>,
        from_timestamp: Option<i64>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let entries = self.get_logs();
        let from_ts = from_timestamp.unwrap_or(0);

        // Filter by timestamp
        Ok(entries
            .into_iter()
            .filter(|e| e.timestamp > from_ts)
            .collect())
    }
}
