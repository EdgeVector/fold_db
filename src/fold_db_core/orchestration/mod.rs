//! Orchestration components for coordinating system operations
//!
//! This module contains orchestration components that coordinate
//! complex operations across multiple system components:
//! - Transform orchestration and coordination
//! - Event-driven FoldDB orchestration
//! - Event-driven database operations

pub mod transform_orchestrator;

// New decomposed orchestration components
pub mod backfill_manager;
pub mod execution_coordinator;
pub mod index_orchestrator;
pub mod index_status;
pub mod persistence_manager;
pub mod progress_store;
pub mod queue_manager;

pub use execution_coordinator::{ExecutionCoordinator, ExecutionStats};
pub use index_status::{IndexStatusTracker, IndexingState, IndexingStatus};
pub use persistence_manager::PersistenceManager;
#[cfg(feature = "aws-backend")]
pub use progress_store::DynamoDbProgressStore;
pub use progress_store::{InMemoryProgressStore, ProgressStore};
pub use queue_manager::{QueueItem, QueueManager, QueueState};
pub use transform_orchestrator::{TransformOrchestrator, TransformQueue};
