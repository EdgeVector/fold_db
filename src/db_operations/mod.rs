// Core database operations
pub mod core;
pub mod sync_wrapper;
pub mod error_utils;
// Legacy v1 - Disabled to avoid conflicts with v2
mod metadata_operations;
mod schema_operations;
mod public_key_operations;
mod transform_operations;
mod atom_operations;
mod native_index;
mod native_index_classification;
mod native_index_ai_classifier;

// Re-export the main DbOperations struct and error utilities
pub use core::DbOperations;
pub use sync_wrapper::DbOperationsSync;
pub use error_utils::ErrorUtils;
pub use native_index::{IndexResult, NativeIndexManager};
pub use native_index_classification::{
    ClassificationType, ExtractedEntity, FieldClassification, SplitStrategy,
    ClassificationRequest, ClassificationCacheKey,
};
pub use native_index_ai_classifier::NativeIndexAIClassifier;
