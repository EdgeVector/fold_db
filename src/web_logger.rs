use log::{LevelFilter, Metadata, Record, SetLoggerError};
use once_cell::sync::OnceCell;
use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};
use tokio::sync::broadcast;

pub struct WebLogger {
    buffer: Mutex<VecDeque<String>>,
    sender: broadcast::Sender<String>,
    level: RwLock<LevelFilter>,
}

impl WebLogger {
    fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            buffer: Mutex::new(VecDeque::with_capacity(1000)),
            sender,
            level: RwLock::new(LevelFilter::Warn),
        }
    }
    
    pub fn set_level(&self, level: LevelFilter) {
        if let Ok(mut current_level) = self.level.write() {
            *current_level = level;
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
            let msg = format!("{} - {}", record.level(), record.args());
            println!("{}", msg);
            if let Ok(mut buf) = self.buffer.lock() {
                buf.push_back(msg.clone());
                if buf.len() > 1000 {
                    buf.pop_front();
                }
            }
            let _ = self.sender.send(msg);
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    let logger = LOGGER.get_or_init(WebLogger::new);
    log::set_logger(logger)?;
    log::set_max_level(LevelFilter::Warn);
    Ok(())
}

pub fn get_logs() -> Vec<String> {
    LOGGER
        .get()
        .map(|l| l.buffer.lock().unwrap().iter().cloned().collect())
        .unwrap_or_default()
}

pub fn subscribe() -> Option<broadcast::Receiver<String>> {
    LOGGER.get().map(|l| l.sender.subscribe())
}

pub fn set_log_level(level: LevelFilter) {
    if let Some(logger) = LOGGER.get() {
        logger.set_level(level);
    }
}
