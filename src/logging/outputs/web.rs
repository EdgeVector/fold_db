//! Web streaming output handler
use crate::logging::config::WebConfig;
use crate::logging::LoggingError;
use log::{LevelFilter, Metadata, Record};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

/// Web output handler that provides streaming logs via broadcast channel
pub struct WebOutput {
    config: WebConfig,
    buffer: Arc<Mutex<VecDeque<String>>>,
    sender: broadcast::Sender<String>,
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

    pub fn get_logs(&self) -> Vec<String> {
        self.buffer.lock().unwrap().iter().cloned().collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    fn should_log(&self, metadata: &Metadata) -> bool {
        if !self.config.enabled {
            return false;
        }

        if let Ok(filter) = self.level_filter.read() {
            metadata.level() <= *filter
        } else {
            false
        }
    }
}

impl log::Log for WebOutput {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.should_log(metadata)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();

            let msg = format!("[{}][{}] - {}", timestamp, record.level(), record.args());

            // Add to buffer
            if let Ok(mut buffer) = self.buffer.lock() {
                buffer.push_back(msg.clone());
                if buffer.len() > self.config.max_logs {
                    buffer.pop_front();
                }
            }

            // Send to broadcast channel
            let _ = self.sender.send(msg);
        }
    }

    fn flush(&self) {}
}
