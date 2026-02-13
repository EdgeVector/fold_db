mod indexing;
mod search;
mod types;

#[cfg(test)]
mod tests;

pub use types::{IndexClassification, IndexEntry, IndexResult};

use crate::crypto::E2eKeys;
use crate::storage::traits::KvStore;
use std::sync::Arc;

use types::EXCLUDED_FIELDS;

#[derive(Clone)]
pub struct NativeIndexManager {
    store: Arc<dyn KvStore>,
    e2e_index_key: Option<[u8; 32]>,
}

impl NativeIndexManager {
    /// Create with KvStore (works with any backend)
    pub fn new(store: Arc<dyn KvStore>, e2e_index_key: Option<[u8; 32]>) -> Self {
        Self {
            store,
            e2e_index_key,
        }
    }

    /// Blind a search/index term using HMAC if an E2E index key is configured.
    /// Returns the term unchanged when no key is present (backward compat).
    fn blind_token(&self, term: &str) -> String {
        match &self.e2e_index_key {
            Some(key) => E2eKeys::blind_token(key, term),
            None => term.to_string(),
        }
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
