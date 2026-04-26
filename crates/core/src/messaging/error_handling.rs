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
