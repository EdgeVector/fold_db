use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HashRange filter operations for querying hash-range fields
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        end: String 
    },
    /// Filter by range key range across all hash groups
    RangeRange { 
        start: String, 
        end: String 
    },
    /// Filter by value match (searches across all hash groups)
    Value(String),
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
    pub fn from_key_config(key_config: Option<crate::schema::types::key_config::KeyConfig>) -> Option<Self> {
        match key_config {
            Some(config) => {
                match (&config.hash_field, &config.range_field) {
                    (Some(hash), Some(range)) => {
                        // Both hash and range fields - create a combined filter
                        Some(Self::HashRangeKey { hash: hash.clone(), range: range.clone() })
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
    pub fn from_json_values(hash_filter_value: Option<serde_json::Value>, range_filter_value: Option<serde_json::Value>) -> Option<Self> {
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
pub struct HashRangeFilterResult {
    /// Matches with composite keys in format "hash:range" -> atom_uuid
    pub matches: HashMap<String, String>,
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
    pub fn new(matches: HashMap<String, String>) -> Self {
        let hash_groups_count = matches
            .keys()
            .map(|key| key.split(':').next().unwrap_or(""))
            .collect::<std::collections::HashSet<_>>()
            .len();
        
        Self {
            total_count: matches.len(),
            matches,
            hash_groups_count,
        }
    }
}

/// Helper function to create composite keys
pub fn create_composite_key(hash: &str, range: &str) -> String {
    format!("{}:{}", hash, range)
}

/// Helper function to parse composite keys
pub fn parse_composite_key(composite_key: &str) -> Option<(String, String)> {
    if let Some(colon_pos) = composite_key.find(':') {
        let hash = composite_key[..colon_pos].to_string();
        let range = composite_key[colon_pos + 1..].to_string();
        Some((hash, range))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_composite_key() {
        assert_eq!(create_composite_key("user123", "post456"), "user123:post456");
        assert_eq!(create_composite_key("", "range"), ":range");
        assert_eq!(create_composite_key("hash", ""), "hash:");
    }

    #[test]
    fn test_parse_composite_key() {
        assert_eq!(
            parse_composite_key("user123:post456"),
            Some(("user123".to_string(), "post456".to_string()))
        );
        assert_eq!(
            parse_composite_key(":range"),
            Some(("".to_string(), "range".to_string()))
        );
        assert_eq!(
            parse_composite_key("hash:"),
            Some(("hash".to_string(), "".to_string()))
        );
        assert_eq!(parse_composite_key("invalid"), None);
    }

    #[test]
    fn test_hash_range_filter_result() {
        let mut matches = HashMap::new();
        matches.insert("user1:post1".to_string(), "atom1".to_string());
        matches.insert("user1:post2".to_string(), "atom2".to_string());
        matches.insert("user2:post1".to_string(), "atom3".to_string());

        let result = HashRangeFilterResult::new(matches);
        assert_eq!(result.total_count, 3);
        assert_eq!(result.hash_groups_count, 2); // user1 and user2
    }

    #[test]
    fn test_empty_result() {
        let result = HashRangeFilterResult::empty();
        assert_eq!(result.total_count, 0);
        assert_eq!(result.hash_groups_count, 0);
        assert!(result.matches.is_empty());
    }
}
