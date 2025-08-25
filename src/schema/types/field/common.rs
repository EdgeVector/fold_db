use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::fees::types::config::FieldPaymentConfig;
use crate::permissions::types::policy::PermissionsPolicy;
use crate::schema::types::Transform;

/// Common interface for all schema fields.
///
/// The `Field` trait exposes accessors for properties shared by all field
/// implementations. These mirror the methods that previously existed on
/// `SchemaField`.
pub trait Field {
    /// Returns the permission policy associated with this field.
    fn permission_policy(&self) -> &PermissionsPolicy;

    /// Returns the payment configuration for this field.
    fn payment_config(&self) -> &FieldPaymentConfig;

    /// Gets the atom reference uuid for this field, if one exists.
    fn molecule_uuid(&self) -> Option<&String>;

    /// Sets the atom reference uuid for this field.
    fn set_molecule_uuid(&mut self, uuid: String);

    /// Returns any field mappers configured for this field.
    fn field_mappers(&self) -> &HashMap<String, String>;

    /// Sets the field mappers for this field.
    fn set_field_mappers(&mut self, mappers: HashMap<String, String>);

    /// Returns the transform associated with this field, if any.
    fn transform(&self) -> Option<&Transform>;

    /// Sets the transform for this field.
    fn set_transform(&mut self, transform: Transform);

    /// Indicates whether this field is writable.
    fn writable(&self) -> bool;

    /// Sets whether this field can be written to.
    fn set_writable(&mut self, writable: bool);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FieldType {
    Single,
    Range,
    HashRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCommon {
    pub permission_policy: PermissionsPolicy,
    pub payment_config: FieldPaymentConfig,
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
        permission_policy: PermissionsPolicy,
        payment_config: FieldPaymentConfig,
        field_mappers: HashMap<String, String>,
    ) -> Self {
        Self {
            permission_policy,
            payment_config,
            molecule_uuid: None,
            field_mappers,
            transform: None,
            writable: true,
        }
    }
}

#[macro_export]
macro_rules! impl_field {
    ($t:ty) => {
        impl $crate::schema::types::field::Field for $t {
            fn permission_policy(&self) -> &$crate::permissions::types::policy::PermissionsPolicy {
                &self.inner.permission_policy
            }

            fn payment_config(&self) -> &$crate::fees::types::config::FieldPaymentConfig {
                &self.inner.payment_config
            }

            fn molecule_uuid(&self) -> Option<&String> {
                self.inner.molecule_uuid.as_ref()
            }

            fn set_molecule_uuid(&mut self, uuid: String) {
                self.inner.molecule_uuid = Some(uuid);
            }

            fn field_mappers(&self) -> &std::collections::HashMap<String, String> {
                &self.inner.field_mappers
            }

            fn set_field_mappers(&mut self, mappers: std::collections::HashMap<String, String>) {
                self.inner.field_mappers = mappers;
            }

            fn transform(&self) -> Option<&$crate::schema::types::Transform> {
                self.inner.transform.as_ref()
            }

            fn set_transform(&mut self, transform: $crate::schema::types::Transform) {
                self.inner.transform = Some(transform);
            }

            fn writable(&self) -> bool {
                self.inner.writable
            }

            fn set_writable(&mut self, writable: bool) {
                self.inner.writable = writable;
            }
        }
    };
}

// Re-export the macro for use in this module
pub use impl_field;
