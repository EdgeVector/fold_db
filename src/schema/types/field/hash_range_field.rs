//! HashRange field type for schema indexing iterator stack model
//!
//! Provides a field type that combines hash and range functionality for
//! efficient indexing with complex fan-out operations.

use crate::fees::types::config::FieldPaymentConfig;
use crate::impl_field;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::common::FieldCommon;
use serde::{Deserialize, Serialize};
use crate::atom::MoleculeHashRange;
use std::collections::{BTreeMap, HashMap};

/// Field that combines hash and range functionality for indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRangeField {
    pub inner: FieldCommon,
    pub molecule_hash_range: Option<MoleculeHashRange>,
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
        permission_policy: PermissionsPolicy,
        payment_config: FieldPaymentConfig,
        field_mappers: HashMap<String, String>,
        molecule_hash_range: Option<MoleculeHashRange>,
    ) -> Self {
        Self {
            inner: FieldCommon::new(permission_policy, payment_config, field_mappers),
            molecule_hash_range,
        }
    }
}

impl_field!(HashRangeField);