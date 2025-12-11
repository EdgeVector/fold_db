// Core database operations
// Core database operations
pub mod core_refactored;
pub mod sync_wrapper;
pub mod error_utils;
// Legacy v1 - Disabled to avoid conflicts with v2
mod metadata_operations_v2;
mod schema_operations_v2;
mod public_key_operations_v2;
mod transform_operations_v2;
mod atom_operations_v2;
mod native_index;
mod native_index_classification;
mod native_index_ai_classifier;

// Re-export the main DbOperations struct and error utilities
pub use core_refactored::DbOperationsV2;
pub use sync_wrapper::DbOperationsSync;
pub use error_utils::ErrorUtils;
pub use native_index::{IndexResult, NativeIndexManager};
pub use native_index_classification::{
    ClassificationType, ExtractedEntity, FieldClassification, SplitStrategy,
    ClassificationRequest, ClassificationCacheKey,
};
pub use native_index_ai_classifier::NativeIndexAIClassifier;

// Legacy exports (deprecated - use DbOperationsV2 instead)
// Kept for backward compatibility during transition
#[deprecated(since = "0.2.0", note = "Use DbOperationsV2 instead")]
pub use core_refactored::DbOperationsV2 as DbOperations;
