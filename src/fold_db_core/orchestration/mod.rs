//! Orchestration components for coordinating system operations
//!
//! This module contains orchestration components that coordinate
//! complex operations across multiple system components:
//! - Transform orchestration and coordination
//! - Event-driven FoldDB orchestration
//! - Event-driven database operations

pub mod transform_orchestrator;

// New decomposed orchestration components
pub mod event_monitor;
pub mod execution_coordinator;
pub mod persistence_manager;
pub mod queue_manager;

pub use event_monitor::EventMonitor;
pub use execution_coordinator::{ExecutionCoordinator, ExecutionStats};
pub use persistence_manager::PersistenceManager;
pub use queue_manager::{QueueItem, QueueManager, QueueState};
pub use transform_orchestrator::{TransformOrchestrator, TransformQueue};
