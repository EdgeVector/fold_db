// Core database operations
mod atom_operations;
pub mod core;
pub mod error_utils;
mod metadata_operations;
mod native_index;
mod orchestrator_operations;
mod public_key_operations;
mod schema_operations;
mod transform_operations;
mod utility_operations;

// Tests module

// Re-export the main DbOperations struct and error utilities
pub use core::DbOperations;
pub use error_utils::ErrorUtils;
pub use native_index::{IndexResult, NativeIndexConfig, NativeIndexManager};
