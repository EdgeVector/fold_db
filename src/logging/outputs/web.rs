//! Web streaming output handler (maintains backward compatibility)

use crate::logging::config::WebConfig;
use crate::logging::util::parse_log_level;
use crate::logging::LoggingError;
use std::collections::VecDeque;
use std::sync::Mutex;
use tokio::sync::broadcast;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{fmt, Layer, Registry};
use std::io::{self, Write};

/// Web output handler that provides streaming log output for web interfaces
pub struct WebOutput {
    config: WebConfig,
    buffer: std::sync::Arc<Mutex<VecDeque<String>>>,
    sender: broadcast::Sender<String>,
}

impl WebOutput {
    /// Create a new web output handler
    pub fn new(config: &WebConfig) -> Result<Self, LoggingError> {
        let (sender, _) = broadcast::channel(config.buffer_size);
        
        Ok(Self {
            config: config.clone(),
            buffer: std::sync::Arc::new(Mutex::new(VecDeque::with_capacity(config.max_logs))),
            sender,
        })
    }

    /// Create a tracing layer for web output
    pub fn create_layer(&self) -> Result<impl Layer<Registry> + Send + Sync, LoggingError> {
        let writer = WebWriter::new(
            self.buffer.clone(),
            self.sender.clone(),
            self.config.max_logs,
        );

        let layer = fmt::Layer::default()
            .with_writer(writer)
            .with_ansi(false); // No ANSI colors for web
            
        let layer = layer.with_filter(parse_log_level(&self.config.level)?);

        Ok(layer)
    }

    /// Get current logs in buffer
    pub fn get_logs(&self) -> Vec<String> {
        self.buffer
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect()
    }

    /// Subscribe to log stream
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

}

/// Custom writer that sends logs to both buffer and broadcast channel
#[derive(Clone)]
struct WebWriter {
    buffer: std::sync::Arc<Mutex<VecDeque<String>>>,
    sender: broadcast::Sender<String>,
    max_logs: usize,
}

impl WebWriter {
    fn new(
        buffer: std::sync::Arc<Mutex<VecDeque<String>>>,
        sender: broadcast::Sender<String>,
        max_logs: usize,
    ) -> Self {
        Self {
            buffer,
            sender,
            max_logs,
        }
    }
}

impl Write for WebWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let msg = String::from_utf8_lossy(buf).trim().to_string();
        
        if !msg.is_empty() {
            // Add to buffer
            if let Ok(mut buffer) = self.buffer.lock() {
                buffer.push_back(msg.clone());
                if buffer.len() > self.max_logs {
                    buffer.pop_front();
                }
            }
            
            // Send to broadcast channel
            let _ = self.sender.send(msg);
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for WebWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}