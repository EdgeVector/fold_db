//! Runtime execution engine for schema indexing iterator stack
//!
//! Handles the actual execution of iterator stacks, broadcasting of values across
//! iterations, and proper index entry emission at the correct depth.

pub mod core;
pub mod field_evaluation;
pub mod field_execution;
pub mod iterator_management;
pub mod tests;

// Re-export main types and functions
pub use core::{
    ExecutionContext, ExecutionEngine, ExecutionResult, ExecutionStatistics, ExecutionWarning,
    ExecutionWarningType, IndexEntry,
};
pub use field_evaluation::FieldEvaluationError;
pub use field_execution::FieldExecutionResult;

// Re-export all public functionality
