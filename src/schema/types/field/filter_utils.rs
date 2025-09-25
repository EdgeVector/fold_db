//! Shared utilities for filter application across different field types
//!
//! This module provides common filter logic that can be reused by RangeField,
//! HashRangeField, and other field types to eliminate code duplication.

use std::collections::HashMap;
use crate::schema::types::field::{HashRangeFilter, HashRangeFilterResult, create_composite_key};
use crate::atom::{MoleculeRange, MoleculeHashRange};

/// Common filter application utilities
pub struct FilterUtils;

impl FilterUtils {
    /// Creates a prefix end boundary for efficient range queries
    /// This is used for BTree range operations to find all keys with a given prefix
    pub fn create_prefix_end(prefix: &str) -> String {
        let mut prefix_end = prefix.to_string();
        if let Some(last_char) = prefix_end.chars().last() {
            if let Some(next_char) = char::from_u32(last_char as u32 + 1) {
                prefix_end.pop();
                prefix_end.push(next_char);
            } else {
                // If we can't increment the last character, append a null character
                prefix_end.push('\0');
            }
        } else {
            // Empty prefix case - search all keys starting with empty string
            prefix_end = "\0".to_string();
        }
        prefix_end
    }

}

/// Trait for fields that can apply HashRangeFilter
/// This allows different field types to implement their own filter logic
/// while sharing common utilities
pub trait FilterApplicator {
    /// Apply a HashRangeFilter and return the results
    fn apply_filter(&self, filter: &HashRangeFilter) -> HashRangeFilterResult;
    
    /// Apply a filter from JSON Value
    fn apply_json_filter(&self, filter_value: &serde_json::Value) -> Result<HashRangeFilterResult, String> {
        let filter: HashRangeFilter = serde_json::from_value(filter_value.clone())
            .map_err(|e| format!("Invalid filter format: {}", e))?;
        Ok(self.apply_filter(&filter))
    }
}

/// Helper trait for range-based operations
/// Used by fields that work with range keys (like RangeField)
pub trait RangeOperations {
    /// Get a single atom UUID by key
    fn get_atom_uuid(&self, key: &str) -> Option<String>;
    
    /// Get all atom UUIDs as key-value pairs
    fn get_all_atoms(&self) -> Vec<(String, String)>;
    
    /// Get atoms in a range (start..end)
    fn get_atoms_in_range(&self, start: &str, end: &str) -> Vec<(String, String)>;
    
    /// Get atoms matching a prefix
    fn get_atoms_with_prefix(&self, prefix: &str) -> Vec<(String, String)>;
    
}

/// Helper trait for hash-range operations
/// Used by fields that work with hash-range combinations (like HashRangeField)
pub trait HashRangeOperations {
    /// Get a single atom UUID by hash and range
    fn get_atom_uuid(&self, hash: &str, range: &str) -> Option<String>;
    
    /// Get all atoms as (hash, range, uuid) tuples
    fn get_all_atoms(&self) -> Vec<(String, String, String)>;
    
    /// Get atoms for a specific hash
    fn get_atoms_for_hash(&self, hash: &str) -> Option<Vec<(String, String)>>;
    
    /// Get atoms in a range for a specific hash
    fn get_atoms_in_range_for_hash(&self, hash: &str, start: &str, end: &str) -> Vec<(String, String)>;
    
    /// Get atoms with prefix for a specific hash
    fn get_atoms_with_prefix_for_hash(&self, hash: &str, prefix: &str) -> Vec<(String, String)>;
    
    
    /// Get all hash values
    fn get_hash_values(&self) -> Vec<String>;
    
    /// Get atoms in hash range
    fn get_atoms_in_hash_range(&self, start: &str, end: &str) -> Vec<(String, String, String)>;
}

/// Generic filter application for RangeField
pub fn apply_range_filter<T: RangeOperations>(operations: &T, filter: &HashRangeFilter) -> HashRangeFilterResult {
    let mut matches = HashMap::new();

    match filter {
        HashRangeFilter::HashKey(key) => {
            if let Some(atom_uuid) = operations.get_atom_uuid(key) {
                matches.insert(key.clone(), atom_uuid);
            }
        }
        HashRangeFilter::RangePrefix(prefix) => {
            for (key, atom_uuid) in operations.get_atoms_with_prefix(prefix) {
                matches.insert(key, atom_uuid);
            }
        }
        HashRangeFilter::RangeRange { start, end } => {
            for (key, atom_uuid) in operations.get_atoms_in_range(start, end) {
                matches.insert(key, atom_uuid);
            }
        }
        HashRangeFilter::Value(target_value) => {
            for (key, atom_uuid) in operations.get_all_atoms() {
                if atom_uuid == *target_value {
                    matches.insert(key, atom_uuid);
                }
            }
        }
        HashRangeFilter::HashRangeKeys(keys) => {
            for (_hash, range) in keys {
                if let Some(atom_uuid) = operations.get_atom_uuid(range) {
                    matches.insert(range.clone(), atom_uuid);
                }
            }
        }
        HashRangeFilter::RangePattern(_pattern) => {
            // Pattern matching not supported - return empty results
        }
        // Hash-range specific filters - RangeField only handles range keys, so ignore hash components
        HashRangeFilter::HashRangeKey { range, .. } => {
            if let Some(atom_uuid) = operations.get_atom_uuid(range) {
                matches.insert(range.clone(), atom_uuid);
            }
        }
        HashRangeFilter::HashRangePrefix { prefix, .. } => {
            for (key, atom_uuid) in operations.get_atoms_with_prefix(prefix) {
                matches.insert(key, atom_uuid);
            }
        }
        HashRangeFilter::HashRangeRange { start, end, .. } => {
            for (key, atom_uuid) in operations.get_atoms_in_range(start, end) {
                matches.insert(key, atom_uuid);
            }
        }
        HashRangeFilter::HashRangePattern { pattern: _pattern, .. } => {
            // Pattern matching not supported - return empty results
        }
        // Hash-only filters - RangeField doesn't handle hash keys, return empty
        HashRangeFilter::HashPattern(_) => {
            // RangeField doesn't handle hash patterns
        }
        HashRangeFilter::HashRange { .. } => {
            // RangeField doesn't handle hash ranges
        }
    }

    HashRangeFilterResult::new(matches)
}

/// Generic filter application for HashRangeField
pub fn apply_hash_range_filter<T: HashRangeOperations>(operations: &T, filter: &HashRangeFilter) -> HashRangeFilterResult {
    let mut matches = HashMap::new();

    match filter {
        HashRangeFilter::HashRangeKey { hash, range } => {
            if let Some(atom_uuid) = operations.get_atom_uuid(hash, range) {
                let composite_key = create_composite_key(hash, range);
                matches.insert(composite_key, atom_uuid);
            }
        }
        HashRangeFilter::HashKey(hash) => {
            if let Some(range_atoms) = operations.get_atoms_for_hash(hash) {
                for (range_key, atom_uuid) in range_atoms {
                    let composite_key = create_composite_key(hash, &range_key);
                    matches.insert(composite_key, atom_uuid);
                }
            }
        }
        HashRangeFilter::HashRangePrefix { hash, prefix } => {
            for (range_key, atom_uuid) in operations.get_atoms_with_prefix_for_hash(hash, prefix) {
                let composite_key = create_composite_key(hash, &range_key);
                matches.insert(composite_key, atom_uuid);
            }
        }
        HashRangeFilter::RangePrefix(prefix) => {
            for hash_value in operations.get_hash_values() {
                for (range_key, atom_uuid) in operations.get_atoms_with_prefix_for_hash(&hash_value, prefix) {
                    let composite_key = create_composite_key(&hash_value, &range_key);
                    matches.insert(composite_key, atom_uuid);
                }
            }
        }
        HashRangeFilter::HashRangeRange { hash, start, end } => {
            for (range_key, atom_uuid) in operations.get_atoms_in_range_for_hash(hash, start, end) {
                let composite_key = create_composite_key(hash, &range_key);
                matches.insert(composite_key, atom_uuid);
            }
        }
        HashRangeFilter::RangeRange { start, end } => {
            for hash_value in operations.get_hash_values() {
                for (range_key, atom_uuid) in operations.get_atoms_in_range_for_hash(&hash_value, start, end) {
                    let composite_key = create_composite_key(&hash_value, &range_key);
                    matches.insert(composite_key, atom_uuid);
                }
            }
        }
        HashRangeFilter::Value(target_value) => {
            for (hash_value, range_key, atom_uuid) in operations.get_all_atoms() {
                if atom_uuid == *target_value {
                    let composite_key = create_composite_key(&hash_value, &range_key);
                    matches.insert(composite_key, atom_uuid);
                }
            }
        }
        HashRangeFilter::HashRangeKeys(keys) => {
            for (hash, range) in keys {
                if let Some(atom_uuid) = operations.get_atom_uuid(hash, range) {
                    let composite_key = create_composite_key(hash, range);
                    matches.insert(composite_key, atom_uuid);
                }
            }
        }
        HashRangeFilter::HashRangePattern { hash: _, pattern: _pattern } => {
            // Pattern matching not supported - return empty results
        }
        HashRangeFilter::RangePattern(_pattern) => {
            // Pattern matching not supported - return empty results
        }
        HashRangeFilter::HashPattern(_pattern) => {
            // Pattern matching not supported - return empty results
        }
        HashRangeFilter::HashRange { start, end } => {
            for (hash_value, range_key, atom_uuid) in operations.get_atoms_in_hash_range(start, end) {
                let composite_key = create_composite_key(&hash_value, &range_key);
                matches.insert(composite_key, atom_uuid);
            }
        }
    }

    HashRangeFilterResult::new(matches)
}

/// Implementation of RangeOperations for MoleculeRange
impl RangeOperations for MoleculeRange {
    fn get_atom_uuid(&self, key: &str) -> Option<String> {
        self.get_atom_uuid(key).cloned()
    }
    
    fn get_all_atoms(&self) -> Vec<(String, String)> {
        self.atom_uuids.iter()
            .map(|(key, uuid)| (key.clone(), uuid.clone()))
            .collect()
    }
    
    fn get_atoms_in_range(&self, start: &str, end: &str) -> Vec<(String, String)> {
        self.atom_uuids.range(start.to_string()..end.to_string())
            .map(|(key, uuid)| (key.clone(), uuid.clone()))
            .collect()
    }
    
    fn get_atoms_with_prefix(&self, prefix: &str) -> Vec<(String, String)> {
        let prefix_end = FilterUtils::create_prefix_end(prefix);
        self.atom_uuids.range(prefix.to_string()..prefix_end)
            .map(|(key, uuid)| (key.clone(), uuid.clone()))
            .collect()
    }
    
}

/// Implementation of HashRangeOperations for MoleculeHashRange
impl HashRangeOperations for MoleculeHashRange {
    fn get_atom_uuid(&self, hash: &str, range: &str) -> Option<String> {
        self.get_atom_uuid(hash, range).cloned()
    }
    
    fn get_all_atoms(&self) -> Vec<(String, String, String)> {
        self.iter_all_atoms()
            .map(|(hash, range, uuid)| (hash.clone(), range.clone(), uuid.clone()))
            .collect()
    }
    
    fn get_atoms_for_hash(&self, hash: &str) -> Option<Vec<(String, String)>> {
        self.get_atoms_for_hash(hash)
            .map(|range_map| {
                range_map.iter()
                    .map(|(range, uuid)| (range.clone(), uuid.clone()))
                    .collect()
            })
    }
    
    fn get_atoms_in_range_for_hash(&self, hash: &str, start: &str, end: &str) -> Vec<(String, String)> {
        self.get_atoms_for_hash(hash)
            .map(|range_map| {
                range_map.range(start.to_string()..end.to_string())
                    .map(|(range, uuid)| (range.clone(), uuid.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    fn get_atoms_with_prefix_for_hash(&self, hash: &str, prefix: &str) -> Vec<(String, String)> {
        let prefix_end = FilterUtils::create_prefix_end(prefix);
        self.get_atoms_for_hash(hash)
            .map(|range_map| {
                range_map.range(prefix.to_string()..prefix_end)
                    .map(|(range, uuid)| (range.clone(), uuid.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    
    fn get_hash_values(&self) -> Vec<String> {
        self.hash_values().cloned().collect()
    }
    
    fn get_atoms_in_hash_range(&self, start: &str, end: &str) -> Vec<(String, String, String)> {
        self.hash_values()
            .flat_map(|hash| {
                self.get_atoms_for_hash(hash)
                    .map(|range_map| {
                        range_map.range(start.to_string()..end.to_string())
                            .map(|(range, uuid)| (hash.clone(), range.clone(), uuid.clone()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect()
    }
}

// TODO: Add tests for these utilities