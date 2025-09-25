//! HashRange field type for schema indexing iterator stack model
//!
//! Provides a field type that combines hash and range functionality for
//! efficient indexing with complex fan-out operations.

use crate::fees::types::config::FieldPaymentConfig;
use crate::impl_field;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::common::FieldCommon;
use crate::schema::types::field::hash_range_filter::{HashRangeFilter, HashRangeFilterResult, create_composite_key, parse_composite_key};
use crate::schema::types::field::{FilterApplicator, HashRangeOperations, apply_hash_range_filter};
use crate::schema::types::SchemaError;
use crate::db_operations::DbOperations;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use crate::atom::MoleculeHashRange;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use log::{info, error};

/// Field that combines hash and range functionality for indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRangeField {
    pub inner: FieldCommon,
    pub molecule: Option<MoleculeHashRange>,
}

/// Configuration for HashRange field indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRangeConfig {
    /// Maximum iterator depth allowed
    pub max_depth: usize,
    /// Whether to enable caching of parsed chains
    pub enable_caching: bool,
}

impl Default for HashRangeConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            enable_caching: true,
        }
    }
}

impl HashRangeField {
    /// Creates a new HashRange field
    #[must_use]
    pub fn new(
        field_mappers: HashMap<String, String>,
        molecule: Option<MoleculeHashRange>,
    ) -> Self {
        Self {
            inner: FieldCommon::new(field_mappers),
            molecule,
        }
    }
}

impl crate::schema::types::field::Field for HashRangeField {
    fn common(&self) -> &crate::schema::types::field::FieldCommon {
        &self.inner
    }
    
    fn common_mut(&mut self) -> &mut crate::schema::types::field::FieldCommon {
        &mut self.inner
    }

    fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations) {
        // If we have a molecule_uuid, look up the corresponding MoleculeHashRange
        if let Some(molecule_uuid) = self.inner.molecule_uuid() {
            let ref_key = format!("ref:{}", molecule_uuid);
            if let Ok(Some(molecule_hash_range)) = db_ops.get_item::<MoleculeHashRange>(&ref_key) {
                self.molecule = Some(molecule_hash_range);
            }
        }
    }

    fn write_mutation(&mut self, key_config: &crate::schema::types::key_config::KeyConfig, atom: crate::atom::Atom, pub_key: String) {
        // Initialize molecule if needed
        if self.molecule.is_none() {
            self.molecule = Some(crate::atom::MoleculeHashRange::new(pub_key.clone()));
        }
        
        // For HashRangeField, we use both hash and range keys to store the atom
        if let (Some(hash_key), Some(range_key)) = (&key_config.hash_field, &key_config.range_field) {
            if let Some(molecule) = &mut self.molecule {
                molecule.set_atom_uuid(key_config, atom.uuid().to_string());
                log::debug!("Writing atom to HashRangeField with pub_key '{}', hash '{}' and range '{}': {:?}", pub_key, hash_key, range_key, atom);
            }
        }
    }

    fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        filter: Option<HashRangeFilter>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🔍 HashRangeField: Resolving hash-range values with filter: {:?}", filter);

        // Refresh field data from database first
        self.refresh_from_db(db_ops);

        // Apply filters to get matching atom UUIDs
        let filter_result = if let Some(ref filter) = filter {
            self.apply_filter(filter)
        } else {
            // No filter - return all hash-range keys
            let mut matches = HashMap::new();
            if let Some(molecule) = &self.molecule {
                for (hash_value, range_key, atom_uuid) in molecule.iter_all_atoms() {
                    let composite_key = format!("{}:{}", hash_value, range_key);
                    matches.insert(composite_key, atom_uuid.clone());
                }
            }
            HashRangeFilterResult::new(matches)
        };

        info!("🔍 HashRangeField: Filter applied, found {} matches", filter_result.matches.len());

        // Fetch actual atom content from database
        let mut resolved_values = serde_json::Map::new();

        for (key, atom_uuid) in filter_result.matches {
            info!("🔍 HashRangeField: Fetching atom content for key '{}', UUID '{}'", key, atom_uuid);
            
            match db_ops.get_item::<crate::atom::Atom>(&format!("atom:{}", atom_uuid)) {
                Ok(Some(atom)) => {
                    info!("✅ HashRangeField: Successfully fetched atom for key '{}'", key);
                    resolved_values.insert(key, atom.content().clone());
                }
                Ok(None) => {
                    error!("❌ HashRangeField: Atom '{}' not found for key '{}'", atom_uuid, key);
                    resolved_values.insert(key, JsonValue::Null);
                }
                Err(e) => {
                    error!("❌ HashRangeField: Failed to fetch atom '{}' for key '{}': {}", atom_uuid, key, e);
                    return Err(SchemaError::InvalidField(format!(
                        "Failed to fetch atom '{}' for key '{}': {}",
                        atom_uuid, key, e
                    )));
                }
            }
        }

        info!("✅ HashRangeField: Value resolution completed successfully");
        Ok(JsonValue::Object(resolved_values))
    }
}

impl HashRangeField {
    /// Applies a filter from a JSON Value (for use with Operation::Query filter)
    pub fn apply_json_filter(&self, filter_value: &JsonValue) -> Result<HashRangeFilterResult, String> {
        let filter: HashRangeFilter = serde_json::from_value(filter_value.clone())
            .map_err(|e| format!("Invalid hash-range filter format: {}", e))?;
        Ok(self.apply_filter(&filter))
    }

    /// Gets all keys in the hash range (useful for pagination or listing)
    /// Returns composite keys in the format "hash_value:range_value"
    pub fn get_all_keys(&self) -> Vec<String> {
        self.molecule
            .as_ref()
            .map(|molecule| {
                molecule
                    .iter_all_atoms()
                    .map(|(hash_value, range_key, _)| create_composite_key(hash_value, range_key))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets a subset of keys within a range for a specific hash group (useful for pagination)
    /// Returns composite keys in the format "hash_value:range_value"
    pub fn get_keys_in_range(&self, hash_value: &str, start: &str, end: &str) -> Vec<String> {
        self.molecule
            .as_ref()
            .and_then(|molecule| molecule.get_atoms_for_hash(hash_value))
            .map(|range_map| {
                // Leverage BTree's efficient range operations
                range_map
                    .range(start.to_string()..end.to_string())
                    .map(|(range_key, _)| create_composite_key(hash_value, range_key))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets all range keys for a specific hash group
    pub fn get_range_keys_for_hash(&self, hash_value: &str) -> Vec<String> {
        self.molecule
            .as_ref()
            .and_then(|molecule| molecule.range_values_for_hash(hash_value))
            .map(|iter| iter.cloned().collect())
            .unwrap_or_default()
    }

    /// Gets the total count of items in the hash range
    pub fn count(&self) -> usize {
        self.molecule
            .as_ref()
            .map(|molecule| molecule.atom_count())
            .unwrap_or(0)
    }

    /// Gets the count of items for a specific hash group
    pub fn count_for_hash(&self, hash_value: &str) -> usize {
        self.molecule
            .as_ref()
            .and_then(|molecule| molecule.get_atoms_for_hash(hash_value))
            .map(|range_map| range_map.len())
            .unwrap_or(0)
    }

    /// Gets all hash values in the molecule
    pub fn get_hash_values(&self) -> Vec<String> {
        self.molecule
            .as_ref()
            .map(|molecule| molecule.hash_values().cloned().collect())
            .unwrap_or_default()
    }
}

impl FilterApplicator for HashRangeField {
    fn apply_filter(&self, filter: &HashRangeFilter) -> HashRangeFilterResult {
        let Some(molecule) = &self.molecule else {
            return HashRangeFilterResult::empty();
        };

        apply_hash_range_filter(molecule, filter)
    }
}