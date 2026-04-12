//! Infrastructure components for system foundation
//!
//! This module contains core infrastructure components:
//! - System initialization utilities
//! - Async API for async operations
//! - Event monitoring and observability
//!
//! Note: the event/message bus lives in the top-level `crate::messaging`
//! module so both the domain (`schema`) and coordinator (`fold_db_core`)
//! layers can depend on it without a circular reference.

pub mod event_monitor;
pub mod event_statistics;
pub mod pending_task_tracker;
pub mod process_results_subscriber;

pub use crate::messaging::{AsyncMessageBus, Event};
pub use event_monitor::EventMonitor;
pub use process_results_subscriber::ProcessResultsSubscriber;
