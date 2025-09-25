use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use crate::fees::types::config::FieldPaymentConfig;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::{
    Field, FieldCommon, FieldType, HashRangeField, RangeField, SingleField,
};
use crate::db_operations::DbOperations;
use crate::schema::types::Transform;

/// Enumeration over all field variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldVariant {
    /// Single value field
    Single(SingleField),
    /// Range of values
    Range(RangeField),
    /// Hash-range field for complex indexing
    HashRange(HashRangeField),
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

    fn write_mutation(&mut self, key_config: &crate::schema::types::key_config::KeyConfig, atom: crate::atom::Atom, pub_key: String) {
        delegate_field_method!(self, write_mutation, key_config, atom, pub_key)
    }
}