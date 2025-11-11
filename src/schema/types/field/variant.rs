use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::schema::types::field::{
    Field, FieldCommon, HashRangeField, RangeField, SingleField,
    HashRangeFilter, FilterApplicator, fetch_atoms_for_matches,
};
use crate::db_operations::DbOperations;
use crate::schema::types::SchemaError;
use crate::schema::types::key_value::KeyValue;
use serde_json::Value as JsonValue;

/// Enumeration over all field variants.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum FieldVariant {
    /// Single value field
    Single(SingleField),
    /// Range of values
    Range(RangeField),
    /// Hash-range field for complex indexing
    HashRange(HashRangeField),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    pub value: JsonValue,
    pub atom_uuid: String,
    pub source_file_name: Option<String>,
}

// Macro to reduce boilerplate for Field trait implementation
macro_rules! delegate_field_method {
    ($self:ident, $method:ident) => {
        match $self {
            Self::Single(f) => f.$method(),
            Self::Range(f) => f.$method(),
            Self::HashRange(f) => f.$method(),
        }
    };
    ($self:ident, $method:ident, $($args:expr),+) => {
        match $self {
            Self::Single(f) => f.$method($($args),+),
            Self::Range(f) => f.$method($($args),+),
            Self::HashRange(f) => f.$method($($args),+),
        }
    };
}

impl Field for FieldVariant {
    fn common(&self) -> &FieldCommon {
        delegate_field_method!(self, common)
    }
    
    fn common_mut(&mut self) -> &mut FieldCommon {
        delegate_field_method!(self, common_mut)
    }

    fn refresh_from_db(&mut self, db_ops: &DbOperations) {
        delegate_field_method!(self, refresh_from_db, db_ops)
    }

    fn write_mutation(&mut self, key_value: &crate::schema::types::key_value::KeyValue, atom: crate::atom::Atom, pub_key: String) {
        delegate_field_method!(self, write_mutation, key_value, atom, pub_key)
    }

    fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        filter: Option<HashRangeFilter>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError> {
        // Refresh field data from database first
        self.refresh_from_db(db_ops);

        // Fetch actual atom content from database using shared helper
        let results = match self {
            FieldVariant::Single(f) => f.apply_filter(filter),
            FieldVariant::Range(f) => f.apply_filter(filter),
            FieldVariant::HashRange(f) => f.apply_filter(filter),
        };
        fetch_atoms_for_matches(db_ops, results.matches)
    }
}
