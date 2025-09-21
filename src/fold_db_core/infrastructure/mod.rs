//! Infrastructure components for system foundation
//!
//! This module contains core infrastructure components:
//! - Message bus for event-driven communication
//! - System initialization utilities
//! - Async API for async operations
//! - Event monitoring and observability

pub mod event_monitor;
pub mod factory;
pub mod init;
pub mod message_bus;

pub use event_monitor::EventMonitor;
pub use message_bus::{
    schema_events::{SchemaChanged, TransformExecuted, TransformTriggered},
    MessageBus,
};
