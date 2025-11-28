pub mod common;
pub mod filter_utils;
pub mod hash_range_field;
pub mod hash_range_filter;
pub mod range_field;
pub mod single_field;
pub mod variant;

pub use common::{Field, FieldCommon, FieldType};
pub use filter_utils::{
    FilterUtils,
    FilterApplicator,
    RangeOperations,
    HashRangeOperations,
    apply_range_filter,
    apply_hash_range_filter,
    fetch_atoms_for_matches,
};
pub use hash_range_field::HashRangeField;
pub use hash_range_filter::{HashRangeFilter, HashRangeFilterResult};
pub use range_field::RangeField;
// RangeFilter has been unified into HashRangeFilter
pub use single_field::SingleField;
pub use variant::{FieldVariant, FieldValue};

/// Helper to run async code from sync context, handling both cases where we're
/// already in a runtime (use block_in_place) or not (create new runtime)
pub(crate) fn run_async<F, T>(future: F) -> Result<T, crate::schema::SchemaError>
where
    F: std::future::Future<Output = Result<T, crate::schema::SchemaError>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(_handle) => {
            // We're already in a runtime, use block_in_place to avoid nested runtime error
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(future)
            })
        }
        Err(_) => {
            // No runtime, create one
            tokio::runtime::Runtime::new()
                .map_err(|e| crate::schema::SchemaError::InvalidData(format!("Failed to create runtime: {}", e)))?
                .block_on(future)
        }
    }
}
