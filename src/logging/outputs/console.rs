//! Console output handler
use crate::logging::config::ConsoleConfig;
use crate::logging::LoggingError;
use log::{LevelFilter, Metadata, Record};
use colored::*;

pub struct ConsoleOutput {
    config: ConsoleConfig,
}

impl ConsoleOutput {
    pub fn new(config: &ConsoleConfig) -> Result<Self, LoggingError> {
        Ok(Self { config: config.clone() })
    }

    fn should_log(&self, metadata: &Metadata) -> bool {
        if !self.config.enabled {
            return false;
        }
        
        // Parse config level to LevelFilter, default to Info if invalid
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

impl log::Log for ConsoleOutput {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.should_log(metadata)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_str = if self.config.colors {
                match record.level() {
                    log::Level::Error => "ERROR".red().to_string(),
                    log::Level::Warn => "WARN".yellow().to_string(),
                    log::Level::Info => "INFO".green().to_string(),
                    log::Level::Debug => "DEBUG".blue().to_string(),
                    log::Level::Trace => "TRACE".magenta().to_string(),
                }
            } else {
                record.level().to_string()
            };

            let timestamp = if self.config.include_timestamp {
                 // Placeholder for actual timestamp logic
                 "".to_string() 
            } else {
                "".to_string()
            };

            let module = if self.config.include_module {
                record.module_path().unwrap_or("unknown")
            } else {
                ""
            };

            println!("{} {} [{}] {}", timestamp, level_str, module, record.args());
        }
    }

    fn flush(&self) {}
}
