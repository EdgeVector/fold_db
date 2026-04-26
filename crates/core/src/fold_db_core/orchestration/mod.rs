//! Orchestration components for coordinating system operations
//!
//! This module contains orchestration components that coordinate
//! complex operations across multiple system components.

pub mod index_status;

pub use index_status::{IndexStatusTracker, IndexingState, IndexingStatus};
