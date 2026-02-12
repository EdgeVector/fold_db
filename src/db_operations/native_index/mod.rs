mod extraction;
mod indexing;
mod search;
mod types;

#[cfg(test)]
mod tests;

pub use types::{BatchIndexOperation, IndexEntry, IndexResult};

use crate::storage::traits::KvStore;
use std::sync::Arc;

use types::EXCLUDED_FIELDS;

#[derive(Clone)]
pub struct NativeIndexManager {
    store: Arc<dyn KvStore>,
}

impl NativeIndexManager {
    /// Create with KvStore (works with any backend)
    pub fn new(store: Arc<dyn KvStore>) -> Self {
        Self { store }
    }

    pub fn should_index_field(field_name: &str) -> bool {
        !EXCLUDED_FIELDS
            .iter()
            .any(|excluded| excluded.eq_ignore_ascii_case(field_name))
    }

    /// Convert IndexEntry results to IndexResult for backward compatibility
    pub fn entries_to_results(&self, entries: Vec<IndexEntry>) -> Vec<IndexResult> {
        entries
            .into_iter()
            .map(|e| e.to_index_result(None))
            .collect()
    }
}
