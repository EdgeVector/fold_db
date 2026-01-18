use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::db_operations::DbOperations;
use crate::schema::types::declarative_schemas::FieldMapper;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::HashRangeFilter;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;
use crate::schema::types::Transform;
/// Common interface for all schema fields.
///
/// The `Field` trait exposes accessors for properties shared by all field
/// implementations. These mirror the methods that previously existed on
/// `SchemaField`.
#[async_trait]
pub trait Field: Send + Sync {
    /// Gets the common field data
    fn common(&self) -> &FieldCommon;

    /// Gets the common field data mutably
    fn common_mut(&mut self) -> &mut FieldCommon;

    /// Refreshes the field's data from the database using the provided key configuration.
    async fn refresh_from_db(&mut self, db_ops: &crate::db_operations::DbOperations);

    /// Writes a mutation to the field
    fn write_mutation(&mut self, key_value: &KeyValue, atom: crate::atom::Atom, pub_key: String);

    /// Resolves field values by refreshing the field, applying filters, and fetching atom content
    async fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        filter: Option<HashRangeFilter>,
    ) -> Result<HashMap<KeyValue, FieldValue>, SchemaError>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FieldType {
    Single,
    Range,
    HashRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FieldCommon {
    pub molecule_uuid: Option<String>,
    pub field_mappers: HashMap<String, FieldMapper>,
    pub transform: Option<Transform>,
    #[serde(default = "default_writable")]
    pub writable: bool,
}

fn default_writable() -> bool {
    true
}

impl FieldCommon {
    pub fn new(field_mappers: HashMap<String, FieldMapper>) -> Self {
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

    pub fn field_mappers(&self) -> &HashMap<String, FieldMapper> {
        &self.field_mappers
    }

    pub fn set_field_mappers(&mut self, mappers: HashMap<String, FieldMapper>) {
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
        #[async_trait::async_trait]
        impl $crate::schema::types::field::Field for $t {
            fn common(&self) -> &$crate::schema::types::field::FieldCommon {
                &self.inner
            }

            fn common_mut(&mut self) -> &mut $crate::schema::types::field::FieldCommon {
                &mut self.inner
            }

            async fn refresh_from_db(&mut self, db_ops: &$crate::db_operations::DbOperations) {
                log::error!("refresh_from_db not implemented for {}", stringify!($t));
            }

            fn write_mutation(
                &mut self,
                key_value: &$crate::schema::types::key_value::KeyValue,
                atom: $crate::atom::Atom,
                pub_key: String,
            ) {
                log::error!("write_mutation not implemented for {}", stringify!($t));
            }

            async fn resolve_value(
                &mut self,
                db_ops: &std::sync::Arc<$crate::db_operations::DbOperations>,
                filter: Option<$crate::schema::types::field::HashRangeFilter>,
            ) -> Result<
                std::collections::HashMap<
                    $crate::schema::types::key_value::KeyValue,
                    $crate::schema::types::field::FieldValue,
                >,
                $crate::schema::types::SchemaError,
            > {
                log::error!("resolve_value not implemented for {}", stringify!($t));
                Err($crate::schema::types::SchemaError::InvalidField(format!(
                    "resolve_value not implemented for {}",
                    stringify!($t)
                )))
            }
        }
    };
}

// Re-export the macro for use in this module
pub use impl_field;
