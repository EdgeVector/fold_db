//! Infrastructure components for system foundation
//!
//! This module contains core infrastructure components:
//! - Message bus for event-driven communication
//! - System initialization utilities
//! - Async API for async operations
//! - Event monitoring and observability

pub mod event_monitor;
pub mod event_statistics;
pub mod message_bus;
pub mod pending_task_tracker;
pub mod process_results_subscriber;

pub use event_monitor::EventMonitor;
pub use message_bus::{AsyncMessageBus, Event};
pub use process_results_subscriber::ProcessResultsSubscriber;
