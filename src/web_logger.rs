use log::{LevelFilter, Metadata, Record, SetLoggerError};
use once_cell::sync::OnceCell;
use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub struct WebLogEntry {
    pub timestamp: i64,
    pub level: String,
    pub message: String,
}

pub struct WebLogger {
    buffer: Mutex<VecDeque<WebLogEntry>>,
    sender: broadcast::Sender<String>,
    level: RwLock<LevelFilter>,
}

impl WebLogger {
    fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(1000)),
            sender,
            level: RwLock::new(LevelFilter::Info),
        }
    }

    pub fn set_level(&self, level: LevelFilter) {
        if let Ok(mut current_level) = self.level.write() {
            *current_level = level;
        }
    }

    pub fn get_entries(&self) -> Vec<WebLogEntry> {
        if let Ok(buf) = self.buffer.lock() {
            buf.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

static LOGGER: OnceCell<WebLogger> = OnceCell::new();

impl log::Log for WebLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        if let Ok(current_level) = self.level.read() {
            metadata.level() <= *current_level
        } else {
            false
        }
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let msg = format!("{}", record.args());
            let full_msg = format!("{} - {}", record.level(), record.args());

            if let Ok(mut buf) = self.buffer.lock() {
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;

                buf.push_back(WebLogEntry {
                    timestamp,
                    level: record.level().to_string(),
                    message: msg,
                });

                if buf.len() > 1000 {
                    buf.pop_front();
                }
            }
            let _ = self.sender.send(full_msg);
        }
    }

    fn flush(&self) {}
}

pub fn get_instance() -> &'static WebLogger {
    LOGGER.get_or_init(WebLogger::new)
}

pub fn init() -> Result<(), SetLoggerError> {
    let logger = get_instance();
    log::set_logger(logger)?;
    log::set_max_level(LevelFilter::Info);
    Ok(())
}

pub fn get_logs() -> Vec<String> {
    LOGGER
        .get()
        .map(|l| {
            l.buffer
                .lock()
                .unwrap()
                .iter()
                .map(|e| format!("{} - {}", e.level, e.message))
                .collect()
        })
        .unwrap_or_default()
}

pub fn get_entries() -> Vec<WebLogEntry> {
    if let Some(logger) = LOGGER.get() {
        logger.get_entries()
    } else {
        Vec::new()
    }
}

pub fn subscribe() -> Option<broadcast::Receiver<String>> {
    LOGGER.get().map(|l| l.sender.subscribe())
}

pub fn set_log_level(level: LevelFilter) {
    if let Some(logger) = LOGGER.get() {
        logger.set_level(level);
    }
}
