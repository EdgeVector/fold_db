use crate::schema::types::key_value::KeyValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

/// HashRange filter operations for querying hash-range fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(
        export,
        export_to = "bindings/src/fold_node/static-react/src/types/generated.ts"
    )
)]
pub enum HashRangeFilter {
    /// Filter by exact hash and range key match
    HashRangeKey { hash: String, range: String },
    /// Filter by hash value only (returns all range keys for that hash)
    HashKey(String),
    /// Filter by exact range key value (across all hash groups for HashRange schemas)
    RangeKey(String),
    /// Filter by range key prefix within a specific hash group
    HashRangePrefix { hash: String, prefix: String },
    /// Filter by range key prefix across all hash groups
    RangePrefix(String),
    /// Filter by range key range within a specific hash group
    HashRangeRange {
        hash: String,
        start: String,
        end: String,
    },
    /// Filter by range key range across all hash groups
    RangeRange { start: String, end: String },
    /// Filter by sample size
    SampleN(usize),
    /// Filter by multiple hash-range key pairs
    HashRangeKeys(Vec<(String, String)>),
    /// Filter by range key pattern within a specific hash group
    HashRangePattern { hash: String, pattern: String },
    /// Filter by range key pattern across all hash groups
    RangePattern(String),
    /// Filter by hash key pattern (supports glob-style matching)
    HashPattern(String),
    /// Filter by hash range (inclusive start, exclusive end) - for hash values
    HashRange { start: String, end: String },
}


/// Result of a hash-range filter operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(
    feature = "ts-bindings",
    ts(
        export,
        export_to = "bindings/src/fold_node/static-react/src/types/generated.ts"
    )
)]
pub struct HashRangeFilterResult {
    /// Matches with composite keys in format "KeyValue" -> atom_uuid
    pub matches: HashMap<KeyValue, String>,
    /// Total count of matches found
    pub total_count: usize,
    /// Number of hash groups that had matches
    pub hash_groups_count: usize,
}

impl HashRangeFilterResult {
    /// Creates an empty result
    pub fn empty() -> Self {
        Self {
            matches: HashMap::new(),
            total_count: 0,
            hash_groups_count: 0,
        }
    }

    /// Creates a result with matches
    pub fn new(matches: HashMap<KeyValue, String>) -> Self {
        let hash_groups_count = matches.keys().len();

        Self {
            total_count: matches.len(),
            matches,
            hash_groups_count,
        }
    }
}
