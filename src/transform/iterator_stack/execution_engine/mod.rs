//! Runtime execution engine for schema indexing iterator stack
//!
//! Handles the actual execution of iterator stacks, broadcasting of values across
//! iterations, and proper index entry emission at the correct depth.

pub mod core;
pub mod field_execution;
pub mod field_evaluation;
pub mod iterator_management;
pub mod tests;

// Re-export main types and functions
pub use core::{ExecutionEngine, ExecutionContext, ExecutionResult, IndexEntry, ExecutionStatistics, ExecutionWarning, ExecutionWarningType};
pub use field_execution::{FieldExecutionResult};
pub use field_evaluation::{FieldEvaluationError};

// Re-export all public functionality
