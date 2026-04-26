//! Structured JSON output handler
use crate::logging::config::StructuredConfig;
use crate::logging::LoggingError;
use log::{LevelFilter, Metadata, Record};
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct StructuredOutput {
    config: StructuredConfig,
    file: Mutex<Option<File>>,
}

#[derive(Serialize)]
struct JsonLogEntry<'a> {
    timestamp: u64,
    level: String,
    module: &'a str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
}

impl StructuredOutput {
    pub fn new(config: &StructuredConfig) -> Result<Self, LoggingError> {
        let file = if config.enabled {
            if let Some(path_str) = &config.path {
                let path = std::path::Path::new(path_str);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(LoggingError::Io)?;
                }
                Some(
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .map_err(LoggingError::Io)?,
                )
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            config: config.clone(),
            file: Mutex::new(file),
        })
    }

    fn should_log(&self, metadata: &Metadata) -> bool {
        if !self.config.enabled {
            return false;
        }

        let filter = match self.config.level.as_str() {
            "TRACE" => LevelFilter::Trace,
            "DEBUG" => LevelFilter::Debug,
            "INFO" => LevelFilter::Info,
            "WARN" => LevelFilter::Warn,
            "ERROR" => LevelFilter::Error,
            _ => LevelFilter::Info,
        };

        metadata.level() <= filter
    }
}

impl log::Log for StructuredOutput {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.should_log(metadata)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let entry = JsonLogEntry {
                timestamp,
                level: record.level().to_string(),
                module: record.module_path().unwrap_or(""),
                message: record.args().to_string(),
                file: record.file(),
                line: record.line(),
            };

            if let Ok(json) = serde_json::to_string(&entry) {
                if let Ok(mut file_guard) = self.file.lock() {
                    if let Some(file) = file_guard.as_mut() {
                        let _ = writeln!(file, "{}", json);
                    } else if self.config.path.is_none() {
                        // If enabled but no file path, perhaps write to stdout/stderr?
                        // But ConsoleOutput already handles stdout.
                        // We will just do nothing if no file is configured,
                        // unless requirements say otherwise.
                        // Previous code had: println!("{}", json);
                        // Let's keep it if no file path is specified but it is enabled.
                        println!("{}", json);
                    }
                }
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(file) = file_guard.as_mut() {
                let _ = file.flush();
            }
        }
    }
}
