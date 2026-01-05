// Core database operations
pub mod core;
pub mod error_utils;
pub mod sync_wrapper;
// Legacy v1 - Disabled to avoid conflicts with v2
mod atom_operations;
mod metadata_operations;
pub mod native_index;
mod native_index_ai_classifier;
mod native_index_classification;
mod public_key_operations;
mod schema_operations;
mod transform_operations;

// Re-export the main DbOperations struct and error utilities
pub use core::DbOperations;
pub use error_utils::ErrorUtils;
pub use native_index::{BatchIndexOperation, IndexResult, NativeIndexManager};
pub use native_index_ai_classifier::NativeIndexAIClassifier;
pub use native_index_classification::{
    ClassificationCacheKey, ClassificationRequest, ClassificationType, ExtractedEntity,
    FieldClassification, SplitStrategy,
};
pub use sync_wrapper::DbOperationsSync;
