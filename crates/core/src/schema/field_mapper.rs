//! Field mapper service for schema expansion.
//!
//! When a schema is expanded (superseded by a new schema that adds fields), the new
//! target schema's shared fields carry `FieldMapper` entries pointing back at the old
//! source schema's fields. On approval, this service copies the source molecule UUIDs
//! onto the target runtime fields so that reads/writes against the new schema land on
//! the same molecules — no data migration required.
//!
//! ## When `apply_field_mappers` is called
//!
//! It is invoked from `SchemaCore::set_schema_state` whenever a schema transitions to
//! `SchemaState::Approved`. It is a no-op for schemas without `field_mappers`.
//!
//! ## Circular redirect handling
//!
//! During schema expansion the old source schema is typically already `Blocked` and
//! has been recorded in the `superseded_by` map as pointing at the new target schema.
//! If we resolved the source through the normal `SchemaCore::get_schema` path we would
//! follow the redirect and end up looking at the target (the schema currently being
//! approved) — a circular lookup. To avoid this, the service reads the source schema
//! directly from `DbOperations::get_schema`, which bypasses the redirect map and
//! returns the raw stored schema (with its original molecule UUIDs intact).
//!
//! ## Schema expansion workflow
//!
//! 1. A new superset schema is created whose shared fields reference the old schema
//!    via `FieldMapper { source_schema, source_field }`.
//! 2. The old schema is blocked and a `superseded_by` entry is recorded.
//! 3. The new schema is approved, which calls `apply_field_mappers`.
//! 4. For each `(target_field, mapper)` entry, the service copies
//!    `molecule_uuid` from the source runtime field onto the target runtime field.
//! 5. The mutated schema is re-synced (`sync_molecule_uuids`) and persisted.
//!
//! New fields (those without a mapper) are left untouched — they receive a fresh
//! molecule UUID on first mutation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::db_operations::DbOperations;
use crate::schema::types::field::Field;
use crate::schema::types::{Schema, SchemaError};

/// Extracts and applies `FieldMapper` entries during schema approval.
///
/// Holds a reference to `DbOperations` (for direct, redirect-bypassing schema reads
/// and for persisting the updated target schema) and a handle to the in-memory
/// schema cache owned by `SchemaCore` (so updates are visible immediately without
/// requiring a reload).
pub struct FieldMapperService {
    db_ops: Arc<DbOperations>,
    schemas_cache: Arc<Mutex<HashMap<String, Schema>>>,
}

impl FieldMapperService {
    pub fn new(
        db_ops: Arc<DbOperations>,
        schemas_cache: Arc<Mutex<HashMap<String, Schema>>>,
    ) -> Self {
        Self {
            db_ops,
            schemas_cache,
        }
    }

    /// Apply every `FieldMapper` on `schema_name`, copying molecule UUIDs from the
    /// referenced source fields onto the corresponding target runtime fields.
    ///
    /// This is a no-op when the schema has no field mappers. See the module docs for
    /// the full workflow and circular-redirect rationale.
    pub async fn apply_field_mappers(&self, schema_name: &str) -> Result<(), SchemaError> {
        let mut schema = self.db_ops.get_schema(schema_name).await?.ok_or_else(|| {
            SchemaError::InvalidData(format!("Schema '{}' not found in database", schema_name))
        })?;

        let Some(field_mappers) = schema.field_mappers().cloned() else {
            return Ok(());
        };

        if field_mappers.is_empty() {
            return Ok(());
        }

        let mut source_cache: HashMap<String, Schema> = HashMap::new();
        let mut updated = false;

        for (target_field, mapper) in field_mappers {
            let source_schema_name = mapper.source_schema().to_string();

            let source_schema = match self
                .resolve_source_schema(&source_schema_name, &mut source_cache)
                .await
            {
                Some(s) => s,
                None => continue,
            };

            let Some(source_field) = source_schema.runtime_fields.get(mapper.source_field()) else {
                tracing::warn!(
                    "apply_field_mappers: source field '{}.{}' not in runtime_fields, skipping",
                    source_schema_name,
                    mapper.source_field()
                );
                continue;
            };

            // If the source field doesn't have a molecule UUID yet (no data written),
            // skip it — the target field will get a fresh molecule on first mutation.
            let Some(molecule_uuid) = source_field.common().molecule_uuid().cloned() else {
                continue;
            };

            let Some(target_runtime_field) = schema.runtime_fields.get_mut(&target_field) else {
                tracing::warn!(
                    "apply_field_mappers: target field '{}' not in runtime_fields, skipping",
                    target_field
                );
                continue;
            };

            target_runtime_field
                .common_mut()
                .set_molecule_uuid(molecule_uuid.clone());
            target_runtime_field
                .common_mut()
                .set_field_mappers(HashMap::from([(target_field.clone(), mapper.clone())]));

            updated = true;
        }

        if updated {
            schema.sync_molecule_uuids();
            self.db_ops.store_schema(schema_name, &schema).await?;
            self.schemas_cache
                .lock()
                .map_err(|_| SchemaError::InvalidData("Failed to acquire schemas lock".into()))?
                .insert(schema_name.to_string(), schema);
        }

        Ok(())
    }

    /// Load a source schema for field mapping, caching the result for the duration
    /// of a single `apply_field_mappers` call.
    ///
    /// Uses `DbOperations::get_schema` directly (bypassing any `superseded_by`
    /// redirect) because during schema expansion the source schema is typically
    /// already blocked and redirected to the target currently being approved.
    /// Following that redirect would produce a circular lookup. We need the raw
    /// source schema with its original molecule UUIDs.
    ///
    /// Returns `None` (with a warning log) if the source schema can't be loaded;
    /// the caller should skip that mapper entry.
    async fn resolve_source_schema<'a>(
        &self,
        source_schema_name: &str,
        source_cache: &'a mut HashMap<String, Schema>,
    ) -> Option<&'a Schema> {
        if !source_cache.contains_key(source_schema_name) {
            let fetched = match self.db_ops.get_schema(source_schema_name).await {
                Ok(Some(s)) => s,
                Ok(None) => {
                    tracing::warn!(
                        "apply_field_mappers: source schema '{}' not found, skipping its mappers",
                        source_schema_name
                    );
                    return None;
                }
                Err(e) => {
                    tracing::warn!(
                        "apply_field_mappers: error loading source schema '{}': {}, skipping",
                        source_schema_name,
                        e
                    );
                    return None;
                }
            };
            source_cache.insert(source_schema_name.to_string(), fetched);
        }
        source_cache.get(source_schema_name)
    }
}
