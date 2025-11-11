//! Query Module - Dedicated query processing for FoldDB
//!
//! This module contains all query-related functionality extracted from the main FoldDB core,
//! providing a clean separation of concerns for query operations.

pub mod hash_range_query;
pub mod query_executor;
pub mod formatter;

// Re-export main query functionality
pub use hash_range_query::HashRangeQueryProcessor;
pub use query_executor::QueryExecutor;
pub use formatter::{format_hash_range_fields, records_from_field_map, FieldMetadata, Record, QueryResultRecord};
