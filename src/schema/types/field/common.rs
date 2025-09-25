use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::fees::types::config::FieldPaymentConfig;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::Transform;
use crate::db_operations::DbOperations;
use crate::schema::types::key_config::KeyConfig;
use crate::atom::Molecule;
use serde_json::Value;
/// Common interface for all schema fields.
///
/// The `Field` trait exposes accessors for properties shared by all field
/// implementations. These mirror the methods that previously existed on
/// `SchemaField`.
pub trait Field {
    /// Gets the common field data
    fn common(&self) -> &FieldCommon;
    
    /// Gets the common field data mutably
    fn common_mut(&mut self) -> &mut FieldCommon;

    /// Refreshes the field's data from the database using the provided key configuration.
    fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations);

    /// Writes a mutation to the field
    fn write_mutation(&mut self, key_config: &KeyConfig, atom: crate::atom::Atom, pub_key: String);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FieldType {
    Single,
    Range,
    HashRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCommon {
    pub molecule_uuid: Option<String>,
    pub field_mappers: HashMap<String, String>,
    pub transform: Option<Transform>,
    #[serde(default = "default_writable")]
    pub writable: bool,
}

fn default_writable() -> bool {
    true
}

impl FieldCommon {
    pub fn new(
        field_mappers: HashMap<String, String>,
    ) -> Self {
        Self {
            molecule_uuid: None,
            field_mappers,
            transform: None,
            writable: true,
        }
    }

    // Convenience methods to avoid repetition
    pub fn molecule_uuid(&self) -> Option<&String> {
        self.molecule_uuid.as_ref()
    }

    pub fn set_molecule_uuid(&mut self, uuid: String) {
        self.molecule_uuid = Some(uuid);
    }

    pub fn field_mappers(&self) -> &HashMap<String, String> {
        &self.field_mappers
    }

    pub fn set_field_mappers(&mut self, mappers: HashMap<String, String>) {
        self.field_mappers = mappers;
    }

    pub fn transform(&self) -> Option<&Transform> {
        self.transform.as_ref()
    }

    pub fn set_transform(&mut self, transform: Transform) {
        self.transform = Some(transform);
    }

    pub fn writable(&self) -> bool {
        self.writable
    }

    pub fn set_writable(&mut self, writable: bool) {
        self.writable = writable;
    }
}

#[macro_export]
macro_rules! impl_field {
    ($t:ty) => {
        impl $crate::schema::types::field::Field for $t {
            fn common(&self) -> &$crate::schema::types::field::FieldCommon {
                &self.inner
            }
            
            fn common_mut(&mut self) -> &mut $crate::schema::types::field::FieldCommon {
                &mut self.inner
            }

            fn refresh_from_db(&mut self, db_ops: &$crate::db_operations::DbOperations) {
                log::error!("refresh_from_db not implemented for {}", stringify!($t));
            }

            fn write_mutation(&mut self, key_config: &$crate::schema::types::key_config::KeyConfig, atom: $crate::atom::Atom, pub_key: String) {
                log::error!("write_mutation not implemented for {}", stringify!($t));
            }
        }
    };
}

// Re-export the macro for use in this module
pub use impl_field;
