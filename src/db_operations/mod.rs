// Core database operations
pub mod core;
pub mod error_utils;
// Atom and molecule operations
pub mod atom_operations;
mod metadata_operations;
pub mod native_index;
mod public_key_operations;
mod schema_operations;
mod transform_operations;

// Re-export the main DbOperations struct and error utilities
pub use core::DbOperations;
pub use error_utils::ErrorUtils;
pub use native_index::{IndexResult, NativeIndexManager};
pub use atom_operations::MoleculeData;
