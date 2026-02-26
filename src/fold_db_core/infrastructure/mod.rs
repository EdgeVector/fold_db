//! Infrastructure components for system foundation
//!
//! This module contains core infrastructure components:
//! - Message bus for event-driven communication
//! - System initialization utilities
//! - Async API for async operations
//! - Event monitoring and observability
//! - Backfill tracking for transform operations

pub mod backfill_tracker;
pub mod event_monitor;
pub mod event_statistics;
// init module removed
pub mod message_bus;
pub mod pending_task_tracker;
pub mod process_results_subscriber;
pub mod schema_approval_handler;

pub use event_monitor::EventMonitor;
pub use process_results_subscriber::ProcessResultsSubscriber;
pub use message_bus::{
    schema_events::{SchemaChanged, TransformExecuted, TransformTriggered},
    AsyncMessageBus, Event,
};
