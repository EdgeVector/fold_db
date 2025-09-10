//! Mutation Module - Dedicated mutation processing for FoldDB
//! 
//! This module contains all mutation-related functionality extracted from the main FoldDB core,
//! providing a clean separation of concerns for mutation operations.

pub mod mutation_executor;
pub mod mutation_processor;

// Re-export main mutation functionality
pub use mutation_executor::MutationExecutor;
pub use mutation_processor::MutationProcessor;
