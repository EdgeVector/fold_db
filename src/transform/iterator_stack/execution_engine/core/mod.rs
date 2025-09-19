//! Core execution engine types and main execution methods
//!
//! Provides the main ExecutionEngine for processing field expressions and
//! coordinating execution across different alignment types.

pub mod engine;
pub mod scope_creation;
pub mod statistics;
pub mod types;

pub use engine::*;
pub use scope_creation::*;
pub use statistics::*;
pub use types::*;
