//! Shared utilities for filter application across different field types
//!
//! This module provides common filter logic that can be reused by RangeField,
//! HashRangeField, and other field types to eliminate code duplication.

use crate::atom::{MoleculeHash, MoleculeHashRange, MoleculeRange};
use crate::db_operations::DbOperations;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::{HashRangeFilter, HashRangeFilterResult};
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;
use std::collections::HashMap;
use std::sync::Arc;

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

/// Resolve atom UUID matches into concrete FieldValue map by fetching atoms (async version)
pub async fn fetch_atoms_for_matches_async(
    db_ops: &Arc<DbOperations>,
    matches: impl IntoIterator<Item = (KeyValue, String)>,
) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
    fetch_atoms_for_matches_async_with_org(db_ops, matches, None).await
}

/// Resolve atom UUID matches with org_hash prefix support.
pub async fn fetch_atoms_for_matches_async_with_org(
    db_ops: &Arc<DbOperations>,
    matches: impl IntoIterator<Item = (KeyValue, String)>,
    org_hash: Option<&str>,
) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
    fetch_atoms_with_key_metadata_async_with_org(
        db_ops,
        matches.into_iter().map(|(kv, uuid)| (kv, uuid, None)),
        org_hash,
    )
    .await
}

/// Resolve atom UUID matches into concrete FieldValue map, preferring molecule
/// per-key metadata over atom metadata for source_file_name and metadata fields.
/// Falls back to atom metadata for backward compatibility with pre-existing data.
pub async fn fetch_atoms_with_key_metadata_async(
    db_ops: &Arc<DbOperations>,
    matches: impl IntoIterator<Item = (KeyValue, String, Option<crate::atom::KeyMetadata>)>,
) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
    fetch_atoms_with_key_metadata_async_with_org(db_ops, matches, None).await
}

/// Resolve atom UUID matches into concrete FieldValue map with org_hash prefix support.
pub async fn fetch_atoms_with_key_metadata_async_with_org(
    db_ops: &Arc<DbOperations>,
    matches: impl IntoIterator<Item = (KeyValue, String, Option<crate::atom::KeyMetadata>)>,
    org_hash: Option<&str>,
) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
    let mut resolved_values: HashMap<KeyValue, FieldValue> = HashMap::new();

    use crate::storage::traits::TypedStore;
    for (key, atom_uuid, key_meta) in matches.into_iter() {
        let base_key = format!("atom:{}", atom_uuid);
        let storage_key = super::build_storage_key(org_hash, &base_key);
        match db_ops
            .atoms()
            .raw()
            .get_item::<crate::atom::Atom>(&storage_key)
            .await
        {
            Ok(Some(atom)) => {
                // Prefer molecule per-key metadata, fall back to atom metadata
                let (source_file_name, metadata) = match key_meta {
                    Some(km) => (
                        km.source_file_name
                            .or_else(|| atom.source_file_name().cloned()),
                        km.metadata.or_else(|| atom.metadata().cloned()),
                    ),
                    None => (atom.source_file_name().cloned(), atom.metadata().cloned()),
                };
                resolved_values.insert(
                    key,
                    FieldValue {
                        value: atom.content().clone(),
                        atom_uuid: atom_uuid.clone(),
                        source_file_name,
                        metadata,
                        molecule_uuid: None,
                        molecule_version: None,
                    },
                );
            }
            Ok(None) => {
                let key_str = key.to_string();
                if key_str.is_empty() {
                    return Err(SchemaError::InvalidField(format!(
                        "Atom '{}' not found",
                        atom_uuid
                    )));
                } else {
                    return Err(SchemaError::InvalidField(format!(
                        "Atom '{}' not found for key '{}'",
                        atom_uuid, key
                    )));
                }
            }
            Err(e) => {
                let key_str = key.to_string();
                if key_str.is_empty() {
                    return Err(SchemaError::InvalidField(format!(
                        "Failed to fetch atom '{}': {}",
                        atom_uuid, e
                    )));
                } else {
                    return Err(SchemaError::InvalidField(format!(
                        "Failed to fetch atom '{}' for key '{}': {}",
                        atom_uuid, key, e
                    )));
                }
            }
        }
    }

    Ok(resolved_values)
}

/// Trait for fields that can apply HashRangeFilter
/// This allows different field types to implement their own filter logic
/// while sharing common utilities
pub trait FilterApplicator {
    /// Apply a HashRangeFilter and return the results
    fn apply_filter(&self, filter: Option<HashRangeFilter>) -> HashRangeFilterResult;
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

/// Helper trait for hash-based operations (single hash key, no range)
/// Used by fields that work with hash keys (like HashField)
pub trait HashOperations {
    /// Get a single atom UUID by hash key
    fn get_atom_uuid(&self, key: &str) -> Option<String>;

    /// Get all atom UUIDs as key-value pairs
    fn get_all_atoms(&self) -> Vec<(String, String)>;
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
    fn get_atoms_in_range_for_hash(
        &self,
        hash: &str,
        start: &str,
        end: &str,
    ) -> Vec<(String, String)>;

    /// Get atoms with prefix for a specific hash
    fn get_atoms_with_prefix_for_hash(&self, hash: &str, prefix: &str) -> Vec<(String, String)>;

    /// Get a deterministic sample of n KeyValues from the update order
    fn sample(&self, n: usize) -> Vec<KeyValue>;

    /// Get all hash values
    fn get_hash_values(&self) -> Vec<String>;

    /// Get atoms in hash range
    fn get_atoms_in_hash_range(&self, start: &str, end: &str) -> Vec<(String, String, String)>;
}

fn insert_hash(m: &mut HashMap<KeyValue, String>, key: String, uuid: String) {
    m.insert(KeyValue::new(Some(key), None), uuid);
}

fn extend_hash<I: IntoIterator<Item = (String, String)>>(
    m: &mut HashMap<KeyValue, String>,
    iter: I,
) {
    for (k, v) in iter {
        insert_hash(m, k, v);
    }
}

/// Generic filter application for HashField (single hash key, no range)
pub fn apply_hash_filter<T: HashOperations>(
    operations: &T,
    optional_filter: Option<HashRangeFilter>,
) -> HashRangeFilterResult {
    let filter = optional_filter.unwrap_or(HashRangeFilter::SampleN(100));
    let mut matches = HashMap::new();

    match filter {
        HashRangeFilter::SampleN(n) => {
            extend_hash(&mut matches, operations.get_all_atoms().into_iter().take(n));
        }
        HashRangeFilter::HashKey(key) => {
            if let Some(uuid) = operations.get_atom_uuid(&key) {
                insert_hash(&mut matches, key, uuid);
            }
        }
        // For Hash fields, range filters don't apply — return all matches
        _ => {
            extend_hash(&mut matches, operations.get_all_atoms());
        }
    }

    HashRangeFilterResult::new(matches)
}

fn insert_range(m: &mut HashMap<KeyValue, String>, key: String, uuid: String) {
    m.insert(KeyValue::new(None, Some(key)), uuid);
}

fn extend_range<I: IntoIterator<Item = (String, String)>>(
    m: &mut HashMap<KeyValue, String>,
    iter: I,
) {
    for (k, v) in iter {
        insert_range(m, k, v);
    }
}

fn insert_hash_range(m: &mut HashMap<KeyValue, String>, hash: String, range: String, uuid: String) {
    m.insert(KeyValue::new(Some(hash), Some(range)), uuid);
}

fn extend_hash_range<I: IntoIterator<Item = (String, String)>>(
    m: &mut HashMap<KeyValue, String>,
    hash: &str,
    iter: I,
) {
    for (rk, uuid) in iter {
        insert_hash_range(m, hash.to_string(), rk, uuid);
    }
}

/// Generic filter application for RangeField
pub fn apply_range_filter<T: RangeOperations>(
    operations: &T,
    optional_filter: Option<HashRangeFilter>,
) -> HashRangeFilterResult {
    let filter = optional_filter.unwrap_or(HashRangeFilter::SampleN(100));
    let mut matches = HashMap::new();

    match filter {
        HashRangeFilter::SampleN(n) => {
            extend_range(&mut matches, operations.get_all_atoms().into_iter().take(n));
        }
        HashRangeFilter::HashKey(key) | HashRangeFilter::RangeKey(key) => {
            if let Some(uuid) = operations.get_atom_uuid(&key) {
                insert_range(&mut matches, key, uuid);
            }
        }
        HashRangeFilter::RangePrefix(prefix) => {
            extend_range(&mut matches, operations.get_atoms_with_prefix(&prefix));
        }
        HashRangeFilter::RangeRange { start, end } => {
            extend_range(&mut matches, operations.get_atoms_in_range(&start, &end));
        }
        HashRangeFilter::HashRangeKeys(keys) => {
            for (_hash, range) in keys {
                if let Some(uuid) = operations.get_atom_uuid(&range) {
                    insert_range(&mut matches, range, uuid);
                }
            }
        }
        HashRangeFilter::RangePattern(_pattern) => {
            // Pattern matching not supported - return empty results
        }
        // Hash-range specific filters - RangeField only handles range keys, so ignore hash components
        HashRangeFilter::HashRangeKey { range, .. } => {
            if let Some(uuid) = operations.get_atom_uuid(&range) {
                insert_range(&mut matches, range, uuid);
            }
        }
        HashRangeFilter::HashRangePrefix { prefix, .. } => {
            extend_range(&mut matches, operations.get_atoms_with_prefix(&prefix));
        }
        HashRangeFilter::HashRangeRange { start, end, .. } => {
            extend_range(&mut matches, operations.get_atoms_in_range(&start, &end));
        }
        HashRangeFilter::HashRangePattern {
            pattern: _pattern, ..
        } => {
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
pub fn apply_hash_range_filter<T: HashRangeOperations>(
    operations: &T,
    optional_filter: Option<HashRangeFilter>,
) -> HashRangeFilterResult {
    let filter = optional_filter.unwrap_or(HashRangeFilter::SampleN(100));
    let mut matches = HashMap::new();

    match filter {
        HashRangeFilter::SampleN(n) => {
            for key_value in operations.sample(n) {
                if let (Some(hash), Some(range)) = (&key_value.hash, &key_value.range) {
                    if let Some(atom_uuid) = operations.get_atom_uuid(hash, range) {
                        matches.insert(key_value, atom_uuid);
                    }
                }
            }
        }
        HashRangeFilter::HashRangeKey { hash, range } => {
            if let Some(uuid) = operations.get_atom_uuid(&hash, &range) {
                insert_hash_range(&mut matches, hash, range, uuid);
            }
        }
        HashRangeFilter::HashKey(hash) => {
            if let Some(range_atoms) = operations.get_atoms_for_hash(&hash) {
                extend_hash_range(&mut matches, &hash, range_atoms);
            }
        }
        HashRangeFilter::RangeKey(range) => {
            for hash_value in operations.get_hash_values() {
                if let Some(uuid) = operations.get_atom_uuid(&hash_value, &range) {
                    insert_hash_range(&mut matches, hash_value, range.clone(), uuid);
                }
            }
        }
        HashRangeFilter::HashRangePrefix { hash, prefix } => {
            extend_hash_range(
                &mut matches,
                &hash,
                operations.get_atoms_with_prefix_for_hash(&hash, &prefix),
            );
        }
        HashRangeFilter::RangePrefix(prefix) => {
            for hash_value in operations.get_hash_values() {
                extend_hash_range(
                    &mut matches,
                    &hash_value,
                    operations.get_atoms_with_prefix_for_hash(&hash_value, &prefix),
                );
            }
        }
        HashRangeFilter::HashRangeRange { hash, start, end } => {
            extend_hash_range(
                &mut matches,
                &hash,
                operations.get_atoms_in_range_for_hash(&hash, &start, &end),
            );
        }
        HashRangeFilter::RangeRange { start, end } => {
            for hash_value in operations.get_hash_values() {
                extend_hash_range(
                    &mut matches,
                    &hash_value,
                    operations.get_atoms_in_range_for_hash(&hash_value, &start, &end),
                );
            }
        }
        HashRangeFilter::HashRangeKeys(keys) => {
            for (hash, range) in keys {
                if let Some(uuid) = operations.get_atom_uuid(&hash, &range) {
                    insert_hash_range(&mut matches, hash, range, uuid);
                }
            }
        }
        HashRangeFilter::HashRangePattern {
            hash: _,
            pattern: _pattern,
        } => {
            // Pattern matching not supported - return empty results
        }
        HashRangeFilter::RangePattern(_pattern) => {
            // Pattern matching not supported - return empty results
        }
        HashRangeFilter::HashPattern(_pattern) => {
            // Pattern matching not supported - return empty results
        }
        HashRangeFilter::HashRange { start, end } => {
            for (hash_value, range_key, uuid) in operations.get_atoms_in_hash_range(&start, &end) {
                insert_hash_range(&mut matches, hash_value, range_key, uuid);
            }
        }
    }

    HashRangeFilterResult::new(matches)
}

/// Implementation of HashOperations for MoleculeHash
impl HashOperations for MoleculeHash {
    fn get_atom_uuid(&self, key: &str) -> Option<String> {
        self.get_atom_uuid(key).cloned()
    }

    fn get_all_atoms(&self) -> Vec<(String, String)> {
        self.atom_uuids
            .iter()
            .map(|(key, entry)| (key.clone(), entry.atom_uuid.clone()))
            .collect()
    }
}

/// Implementation of RangeOperations for MoleculeRange
impl RangeOperations for MoleculeRange {
    fn get_atom_uuid(&self, key: &str) -> Option<String> {
        self.get_atom_uuid(key).cloned()
    }

    fn get_all_atoms(&self) -> Vec<(String, String)> {
        self.atom_uuids
            .iter()
            .map(|(key, entry)| (key.clone(), entry.atom_uuid.clone()))
            .collect()
    }

    fn get_atoms_in_range(&self, start: &str, end: &str) -> Vec<(String, String)> {
        self.atom_uuids
            .range(start.to_string()..end.to_string())
            .map(|(key, entry)| (key.clone(), entry.atom_uuid.clone()))
            .collect()
    }

    fn get_atoms_with_prefix(&self, prefix: &str) -> Vec<(String, String)> {
        let prefix_end = FilterUtils::create_prefix_end(prefix);
        self.atom_uuids
            .range(prefix.to_string()..prefix_end)
            .map(|(key, entry)| (key.clone(), entry.atom_uuid.clone()))
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
        self.get_atoms_for_hash(hash).map(|range_map| {
            range_map
                .iter()
                .map(|(range, uuid)| (range.clone(), uuid.clone()))
                .collect()
        })
    }

    fn get_atoms_in_range_for_hash(
        &self,
        hash: &str,
        start: &str,
        end: &str,
    ) -> Vec<(String, String)> {
        self.get_atoms_for_hash(hash)
            .map(|range_map| {
                range_map
                    .range(start.to_string()..end.to_string())
                    .map(|(range, uuid)| (range.clone(), uuid.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_atoms_with_prefix_for_hash(&self, hash: &str, prefix: &str) -> Vec<(String, String)> {
        let prefix_end = FilterUtils::create_prefix_end(prefix);
        self.get_atoms_for_hash(hash)
            .map(|range_map| {
                range_map
                    .range(prefix.to_string()..prefix_end)
                    .map(|(range, uuid)| (range.clone(), uuid.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn sample(&self, n: usize) -> Vec<KeyValue> {
        self.sample(n)
    }

    fn get_hash_values(&self) -> Vec<String> {
        self.hash_values().cloned().collect()
    }

    fn get_atoms_in_hash_range(&self, start: &str, end: &str) -> Vec<(String, String, String)> {
        self.hash_values()
            .flat_map(|hash| {
                self.get_atoms_for_hash(hash)
                    .map(|range_map| {
                        range_map
                            .range(start.to_string()..end.to_string())
                            .map(|(range, uuid)| (hash.clone(), range.clone(), uuid.clone()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect()
    }
}
