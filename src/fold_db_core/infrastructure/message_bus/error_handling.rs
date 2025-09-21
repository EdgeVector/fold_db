//! Error types and handling for the message bus system

use thiserror::Error;

/// Errors that can occur within the message bus system
#[derive(Error, Debug)]
pub enum MessageBusError {
    /// Failed to send a message to subscribers
    #[error("Failed to send message: {reason}")]
    SendFailed { reason: String },

    /// Failed to register a consumer
    #[error("Failed to register consumer for event type: {event_type}")]
    RegistrationFailed { event_type: String },

    /// Channel is disconnected
    #[error("Channel disconnected for event type: {event_type}")]
    ChannelDisconnected { event_type: String },
}

/// Result type for message bus operations
pub type MessageBusResult<T> = Result<T, MessageBusError>;

/// Errors for async message reception
#[derive(Error, Debug, Clone)]
pub enum AsyncRecvError {
    #[error("Timeout while waiting for message")]
    Timeout,
    #[error("Channel disconnected")]
    Disconnected,
}

/// Errors for async try_recv
#[derive(Error, Debug, Clone)]
pub enum AsyncTryRecvError {
    #[error("No message available")]
    Empty,
    #[error("Channel disconnected")]
    Disconnected,
}

/// Event with retry metadata for error recovery
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RetryableEvent {
    /// The original event
    pub event: super::events::Event,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Maximum retries allowed
    pub max_retries: u32,
    /// Timestamp of original event
    pub timestamp: std::time::SystemTime,
    /// Error from last failure (if any)
    pub last_error: Option<String>,
}

impl RetryableEvent {
    /// Create a new retryable event
    pub fn new(event: super::events::Event, max_retries: u32) -> Self {
        Self {
            event,
            retry_count: 0,
            max_retries,
            timestamp: std::time::SystemTime::now(),
            last_error: None,
        }
    }

    /// Check if event can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Increment retry count and set error
    pub fn increment_retry(&mut self, error: String) {
        self.retry_count += 1;
        self.last_error = Some(error);
    }

    /// Check if event should go to dead letter queue
    pub fn is_dead_letter(&self) -> bool {
        self.retry_count >= self.max_retries
    }
}

/// Dead letter event for events that couldn't be processed
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DeadLetterEvent {
    /// The original retryable event
    pub retryable_event: RetryableEvent,
    /// Timestamp when moved to dead letter queue
    pub dead_letter_timestamp: std::time::SystemTime,
    /// Reason for dead lettering
    pub reason: String,
}

impl DeadLetterEvent {
    /// Create a new dead letter event
    pub fn new(retryable_event: RetryableEvent, reason: String) -> Self {
        Self {
            retryable_event,
            dead_letter_timestamp: std::time::SystemTime::now(),
            reason,
        }
    }
}

/// Event history entry for event sourcing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct EventHistoryEntry {
    /// Unique event ID
    pub event_id: String,
    /// The event data
    pub event: super::events::Event,
    /// Timestamp when event occurred
    pub timestamp: std::time::SystemTime,
    /// Source component that generated the event
    pub source: String,
    /// Event sequence number (for ordering)
    pub sequence_number: u64,
}

impl EventHistoryEntry {
    /// Create a new event history entry
    pub fn new(event: super::events::Event, source: String, sequence_number: u64) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            event,
            timestamp: std::time::SystemTime::now(),
            source,
            sequence_number,
        }
    }
}
