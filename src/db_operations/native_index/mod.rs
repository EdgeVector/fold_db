mod classification;
mod extraction;
mod indexing;
mod search;
mod types;

#[cfg(test)]
mod tests;

pub use classification::ClassificationType;
pub use types::{BatchIndexOperation, IndexEntry, IndexResult};

use crate::storage::traits::KvStore;
use sled::Tree;
use std::sync::Arc;

use types::EXCLUDED_FIELDS;

#[derive(Clone)]
pub struct NativeIndexManager {
    tree: Option<Tree>,
    store: Option<Arc<dyn KvStore>>,
}

impl NativeIndexManager {
    /// Create with Sled Tree (backward compatible)
    pub fn new(tree: Tree) -> Self {
        Self {
            tree: Some(tree),
            store: None,
        }
    }

    /// Create with KvStore (works with any backend)
    pub fn new_with_store(store: Arc<dyn KvStore>) -> Self {
        Self {
            tree: None,
            store: Some(store),
        }
    }

    /// Check if this manager uses async storage (DynamoDB) vs sync (Sled)
    pub fn is_async(&self) -> bool {
        self.store.is_some()
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
