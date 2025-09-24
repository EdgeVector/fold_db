use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use crate::fees::types::config::FieldPaymentConfig;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::field::{
    Field, FieldCommon, FieldType, HashRangeField, RangeField, SingleField,
};
use crate::schema::molecule_variants::MoleculeVariant;
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

impl Field for FieldVariant {
    fn permission_policy(&self) -> &PermissionsPolicy {
        match self {
            Self::Single(f) => f.permission_policy(),
            Self::Range(f) => f.permission_policy(),
            Self::HashRange(f) => f.permission_policy(),
        }
    }

    fn payment_config(&self) -> &FieldPaymentConfig {
        match self {
            Self::Single(f) => f.payment_config(),
            Self::Range(f) => f.payment_config(),
            Self::HashRange(f) => f.payment_config(),
        }
    }

    fn molecule_uuid(&self) -> Option<&String> {
        match self {
            Self::Single(f) => f.molecule_uuid(),
            Self::Range(f) => f.molecule_uuid(),
            Self::HashRange(f) => f.molecule_uuid(),
        }
    }

    fn set_molecule_uuid(&mut self, uuid: String) {
        match self {
            Self::Single(f) => f.set_molecule_uuid(uuid),
            Self::Range(f) => f.set_molecule_uuid(uuid),
            Self::HashRange(f) => f.set_molecule_uuid(uuid),
        }
    }

    fn field_mappers(&self) -> &HashMap<String, String> {
        match self {
            Self::Single(f) => f.field_mappers(),
            Self::Range(f) => f.field_mappers(),
            Self::HashRange(f) => f.field_mappers(),
        }
    }

    fn set_field_mappers(&mut self, mappers: HashMap<String, String>) {
        match self {
            Self::Single(f) => f.set_field_mappers(mappers),
            Self::Range(f) => f.set_field_mappers(mappers),
            Self::HashRange(f) => f.set_field_mappers(mappers),
        }
    }

    fn transform(&self) -> Option<&Transform> {
        match self {
            Self::Single(f) => f.transform(),
            Self::Range(f) => f.transform(),
            Self::HashRange(f) => f.transform(),
        }
    }

    fn set_transform(&mut self, transform: Transform) {
        match self {
            Self::Single(f) => f.set_transform(transform),
            Self::Range(f) => f.set_transform(transform),
            Self::HashRange(f) => f.set_transform(transform),
        }
    }

    fn writable(&self) -> bool {
        match self {
            Self::Single(f) => f.writable(),
            Self::Range(f) => f.writable(),
            Self::HashRange(f) => f.writable(),
        }
    }

    fn set_writable(&mut self, writable: bool) {
        match self {
            Self::Single(f) => f.set_writable(writable),
            Self::Range(f) => f.set_writable(writable),
            Self::HashRange(f) => f.set_writable(writable),
        }
    }
}

