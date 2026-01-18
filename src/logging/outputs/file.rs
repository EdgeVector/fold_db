//! File output handler
use crate::logging::config::FileConfig;
use crate::logging::LoggingError;
use log::{LevelFilter, Metadata, Record};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

pub struct FileOutput {
    config: FileConfig,
    file: Mutex<Option<std::fs::File>>,
}

impl FileOutput {
    pub fn new(config: &FileConfig) -> Result<Self, LoggingError> {
        let file = if config.enabled {
            let path = std::path::Path::new(&config.path);
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

impl log::Log for FileOutput {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.should_log(metadata)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if let Ok(mut file_guard) = self.file.lock() {
                if let Some(file) = file_guard.as_mut() {
                    let _ = writeln!(
                        file,
                        "[{}] [{}] - {}",
                        record.level(),
                        record.module_path().unwrap_or("unknown"),
                        record.args()
                    );
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
