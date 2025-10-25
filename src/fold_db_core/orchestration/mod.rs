//! Orchestration components for coordinating system operations
//!
//! This module contains orchestration components that coordinate
//! complex operations across multiple system components:
//! - Transform orchestration and coordination
//! - Event-driven FoldDB orchestration
//! - Event-driven database operations

pub mod transform_orchestrator;

// New decomposed orchestration components
pub mod transform_event_monitor;
pub mod execution_coordinator;
pub mod persistence_manager;
pub mod queue_manager;
pub mod mutation_event_manager;
pub mod index_event_handler;
pub mod index_status;

pub use transform_event_monitor::TransformEventMonitor;
pub use execution_coordinator::{ExecutionCoordinator, ExecutionStats};
pub use persistence_manager::PersistenceManager;
pub use queue_manager::{QueueItem, QueueManager, QueueState};
pub use transform_orchestrator::{TransformOrchestrator, TransformQueue};
pub use mutation_event_manager::MutationEventManager;
pub use index_event_handler::IndexEventHandler;
pub use index_status::{IndexStatusTracker, IndexingStatus, IndexingState};
