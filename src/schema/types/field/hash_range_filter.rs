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

impl HashRangeFilter {
    /// Create a unified filter from a KeyConfig
    /// This converts KeyConfig field names to appropriate HashRangeFilter variants
    pub fn from_key_config(
        key_config: Option<crate::schema::types::key_config::KeyConfig>,
    ) -> Option<Self> {
        match key_config {
            Some(config) => {
                match (&config.hash_field, &config.range_field) {
                    (Some(hash), Some(range)) => {
                        // Both hash and range fields - create a combined filter
                        Some(Self::HashRangeKey {
                            hash: hash.clone(),
                            range: range.clone(),
                        })
                    }
                    (Some(hash), None) => {
                        // Only hash field
                        Some(Self::HashKey(hash.clone()))
                    }
                    (None, Some(range)) => {
                        // Only range field - convert to range prefix filter
                        Some(Self::RangePrefix(range.clone()))
                    }
                    (None, None) => {
                        // No fields specified
                        None
                    }
                }
            }
            None => None,
        }
    }

    /// Create a unified filter from JSON values (for use with query filters)
    pub fn from_json_values(
        hash_filter_value: Option<serde_json::Value>,
        range_filter_value: Option<serde_json::Value>,
    ) -> Option<Self> {
        let hash_filter = hash_filter_value.and_then(|v| serde_json::from_value::<Self>(v).ok());
        let range_filter = range_filter_value.and_then(|v| serde_json::from_value::<Self>(v).ok());

        match (hash_filter, range_filter) {
            (Some(hash), Some(range)) => {
                // Both hash and range filters - combine them
                Some(Self::combine_filters(hash, range))
            }
            (Some(hash), None) => Some(hash),
            (None, Some(range)) => Some(range),
            (None, None) => None,
        }
    }

    /// Combine two HashRangeFilter instances into a single filter
    pub fn combine_filters(hash_filter: Self, range_filter: Self) -> Self {
        match (hash_filter, range_filter) {
            // HashKey + RangePrefix = HashRangePrefix
            (Self::HashKey(hash), Self::RangePrefix(prefix)) => {
                Self::HashRangePrefix { hash, prefix }
            }
            // HashKey + RangeRange = HashRangeRange
            (Self::HashKey(hash), Self::RangeRange { start, end }) => {
                Self::HashRangeRange { hash, start, end }
            }
            // HashKey + RangePattern = HashRangePattern
            (Self::HashKey(hash), Self::RangePattern(pattern)) => {
                Self::HashRangePattern { hash, pattern }
            }
            // For other combinations, prefer the hash filter and log a warning
            (hash_filter, _) => {
                log::warn!("⚠️ Combining filters: using hash filter, ignoring range filter");
                hash_filter
            }
        }
    }
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
