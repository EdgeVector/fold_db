use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::access::types::FieldAccessPolicy;
use crate::db_operations::DbOperations;
use crate::schema::types::declarative_schemas::FieldMapper;
use crate::schema::types::field::FieldValue;
use crate::schema::types::field::HashRangeFilter;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::SchemaError;
use crate::schema::types::Transform;

/// Bundles all write-time provenance through the Field trait.
/// Contains the atom plus optional metadata that should be stored
/// per-key on the molecule (surviving atom dedup).
pub struct WriteContext {
    pub atom: crate::atom::Atom,
    pub pub_key: String,
    pub source_file_name: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub schema_name: String,
    pub field_name: String,
    /// The signing keypair for molecule signatures.
    pub signer: std::sync::Arc<crate::security::Ed25519KeyPair>,
}

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
    fn write_mutation(&mut self, key_value: &KeyValue, ctx: WriteContext);

    /// Resolves field values by refreshing the field, applying filters, and fetching atom content.
    /// If `as_of` is provided, rewinds the molecule to that point in time before resolving.
    async fn resolve_value(
        &mut self,
        db_ops: &Arc<DbOperations>,
        filter: Option<HashRangeFilter>,
        as_of: Option<DateTime<Utc>>,
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
    /// Per-field access control policy. None = legacy behavior (no access checks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_policy: Option<FieldAccessPolicy>,
    /// Org hash inherited from the parent schema.
    /// When set, all Sled keys for this field's data are prefixed with `{org_hash}:`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_hash: Option<String>,
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
            access_policy: None,
            org_hash: None,
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

    pub fn writable(&self) -> bool {
        self.writable
    }

    /// Build a storage key, prepending the org_hash prefix when present.
    ///
    /// - Personal: `base_key` (e.g. `atom:{uuid}`, `ref:{uuid}`)
    /// - Org: `{org_hash}:{base_key}` (e.g. `{org_hash}:atom:{uuid}`)
    pub fn storage_key(&self, base_key: &str) -> String {
        build_storage_key(self.org_hash.as_deref(), base_key)
    }

    pub fn org_hash(&self) -> Option<&str> {
        self.org_hash.as_deref()
    }

    pub fn set_org_hash(&mut self, org_hash: Option<String>) {
        self.org_hash = org_hash;
    }
}

/// Build a storage key, prepending the org_hash prefix when present.
///
/// - Personal: `base_key` (e.g. `atom:{uuid}`, `ref:{uuid}`)
/// - Org: `{org_hash}:{base_key}` (e.g. `{org_hash}:atom:{uuid}`)
pub fn build_storage_key(org_hash: Option<&str>, base_key: &str) -> String {
    match org_hash {
        Some(hash) => format!("{hash}:{base_key}"),
        None => base_key.to_string(),
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
                ctx: $crate::schema::types::field::WriteContext,
            ) {
                let _ = (key_value, ctx);
                log::error!("write_mutation not implemented for {}", stringify!($t));
            }

            async fn resolve_value(
                &mut self,
                db_ops: &std::sync::Arc<$crate::db_operations::DbOperations>,
                filter: Option<$crate::schema::types::field::HashRangeFilter>,
                _as_of: Option<chrono::DateTime<chrono::Utc>>,
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
