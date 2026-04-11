use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json;

use crate::fold_db_core::infrastructure::message_bus::AsyncMessageBus;
use crate::schema::types::field::Field;
use crate::schema::types::{DeclarativeSchemaDefinition, Schema, SchemaError};
use crate::schema::{SchemaState, SchemaWithState};
use crate::view::registry::{ViewRegistry, ViewState};
use crate::view::types::TransformView;
use crate::view::wasm_engine::WasmTransformEngine;

/// Core schema management system that combines schema interpretation, validation, and management.
///
/// SchemaCore is responsible for:
/// - Loading and validating schemas from JSON
/// - Managing schema storage and persistence
/// - Handling schema field mappings
/// - Providing schema access and validation services
///
/// This unified component simplifies the schema system by combining the functionality
/// previously split across SchemaManager and SchemaInterpreter.
pub struct SchemaCore {
    /// Storage for loaded schemas
    schemas: Arc<Mutex<HashMap<String, Schema>>>,
    /// Storage for all schemas known to the system and their load state
    schema_states: Arc<Mutex<HashMap<String, SchemaState>>>,
    /// Maps blocked/superseded schema names to their replacement schema names
    superseded_by: Arc<Mutex<HashMap<String, String>>>,
    /// Unified database operations with storage abstraction
    db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
    /// Message bus for event-driven communication
    message_bus: Arc<AsyncMessageBus>,
    /// Registry for transform views
    view_registry: Mutex<ViewRegistry>,
}

/// Acquire a `Mutex<HashMap<String, T>>` lock, mapping poison errors to `SchemaError`.
fn lock_map<'a, T>(
    map: &'a Mutex<HashMap<String, T>>,
    name: &str,
) -> Result<std::sync::MutexGuard<'a, HashMap<String, T>>, SchemaError> {
    map.lock()
        .map_err(|_| SchemaError::InvalidData(format!("Failed to acquire {} lock", name)))
}

impl SchemaCore {
    /// Creates a new SchemaCore with DbOperations (storage abstraction)
    pub async fn new(
        db_ops: std::sync::Arc<crate::db_operations::DbOperations>,
        message_bus: Arc<AsyncMessageBus>,
    ) -> Result<Self, SchemaError> {
        // load schemas from db (async)
        let schemas = db_ops.get_all_schemas().await?;

        let schema_states = db_ops.get_all_schema_states().await?;
        let superseded_by = db_ops.get_all_superseded_by().await?;

        // Load transform views from storage
        let views = db_ops.get_all_views().await?;
        let view_states = db_ops.get_all_view_states().await?;
        let wasm_engine = Arc::new(WasmTransformEngine::new()?);
        let view_registry = ViewRegistry::load(views, view_states, wasm_engine);

        let schema_core = Self {
            schemas: Arc::new(Mutex::new(schemas)),
            schema_states: Arc::new(Mutex::new(schema_states)),
            superseded_by: Arc::new(Mutex::new(superseded_by)),
            db_ops,
            message_bus,
            view_registry: Mutex::new(view_registry),
        };

        Ok(schema_core)
    }

    /// Reload schemas from the persistent store, merging any newly-discovered
    /// schemas into the in-memory cache. Existing entries are NOT overwritten
    /// (additive merge only). Returns the count of newly added schemas.
    pub async fn reload_from_store(&self) -> Result<usize, SchemaError> {
        let stored_schemas = self.db_ops.get_all_schemas().await?;
        let stored_states = self.db_ops.get_all_schema_states().await?;

        let mut schemas = lock_map(&self.schemas, "schemas")?;
        let mut states = lock_map(&self.schema_states, "schema_states")?;

        let mut added = 0usize;
        for (name, mut schema) in stored_schemas {
            if !schemas.contains_key(&name) {
                // Ensure runtime_fields are populated — schemas coming from
                // sync replay won't have them (runtime_fields is #[serde(skip)]).
                if schema.runtime_fields.is_empty() {
                    schema.populate_runtime_fields()?;
                }
                schemas.insert(name.clone(), schema);
                let state = stored_states.get(&name).copied().unwrap_or_default();
                states.insert(name, state);
                added += 1;
            }
        }

        if added > 0 {
            log::info!("reload_from_store: added {} new schema(s) to cache", added);
        }

        Ok(added)
    }

    pub fn get_schemas(&self) -> Result<HashMap<String, Schema>, SchemaError> {
        Ok(lock_map(&self.schemas, "schemas")?.clone())
    }

    pub fn get_schema_states(&self) -> Result<HashMap<String, SchemaState>, SchemaError> {
        Ok(lock_map(&self.schema_states, "schema_states")?.clone())
    }

    pub fn get_schemas_with_states(&self) -> Result<Vec<SchemaWithState>, SchemaError> {
        let schemas = self.get_schemas()?;
        let schema_states = self.get_schema_states()?;

        let mut with_states = Vec::with_capacity(schemas.len());
        for (name, schema) in schemas {
            let state = schema_states.get(&name).copied().unwrap_or_default();
            with_states.push(SchemaWithState::new(schema, state));
        }

        Ok(with_states)
    }

    /// Returns only active (non-Blocked) schemas for UI listings.
    /// Blocked schemas have been superseded and should not appear in the Data Browser.
    pub fn get_active_schemas_with_states(&self) -> Result<Vec<SchemaWithState>, SchemaError> {
        let all = self.get_schemas_with_states()?;
        Ok(all
            .into_iter()
            .filter(|s| s.state != SchemaState::Blocked)
            .collect())
    }

    pub async fn set_schema_state(
        &self,
        schema_name: &str,
        schema_state: SchemaState,
    ) -> Result<(), SchemaError> {
        if schema_state == SchemaState::Approved {
            self.apply_field_mappers(schema_name).await?;
        }

        // Persist to database first - this is the source of truth
        self.db_ops
            .store_schema_state(schema_name, &schema_state)
            .await?;

        // Update in-memory cache only after successful persistence
        lock_map(&self.schema_states, "schema_states")?
            .insert(schema_name.to_string(), schema_state);

        Ok(())
    }

    /// Approve a schema if it's not already approved (idempotent operation).
    /// Does NOT override Blocked state — blocked schemas have been superseded
    /// and must not be re-approved.
    pub async fn approve(&self, schema_name: &str) -> Result<(), SchemaError> {
        let current_state = self
            .get_schema_states()?
            .get(schema_name)
            .copied()
            .unwrap_or_default();

        if current_state == SchemaState::Blocked {
            log::debug!(
                "Skipping approve for blocked schema '{}' — it has been superseded",
                schema_name
            );
            return Ok(());
        }

        if current_state != SchemaState::Approved {
            self.set_schema_state(schema_name, SchemaState::Approved)
                .await?;
        }

        Ok(())
    }

    async fn apply_field_mappers(&self, schema_name: &str) -> Result<(), SchemaError> {
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
            let source_schema = if let Some(schema) = source_cache.get(&source_schema_name) {
                schema
            } else {
                // Use db_ops.get_schema directly (not self.get_schema) to bypass
                // the superseded_by redirect chain. The source schema may already
                // be blocked and superseded to point back to this schema, which
                // would cause a circular redirect. We need the raw source schema
                // with its molecule UUIDs.
                let fetched = match self.db_ops.get_schema(&source_schema_name).await {
                    Ok(Some(s)) => s,
                    Ok(None) => {
                        log::warn!(
                            "apply_field_mappers: source schema '{}' not found, skipping its mappers",
                            source_schema_name
                        );
                        continue;
                    }
                    Err(e) => {
                        log::warn!(
                            "apply_field_mappers: error loading source schema '{}': {}, skipping",
                            source_schema_name,
                            e
                        );
                        continue;
                    }
                };
                source_cache.insert(source_schema_name.clone(), fetched);
                source_cache
                    .get(&source_schema_name)
                    .expect("source schema inserted")
            };

            let Some(source_field) = source_schema.runtime_fields.get(mapper.source_field()) else {
                log::warn!(
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
                log::warn!(
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
            lock_map(&self.schemas, "schemas")?.insert(schema_name.to_string(), schema);
        }

        Ok(())
    }

    pub async fn block_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        self.set_schema_state(schema_name, SchemaState::Blocked)
            .await
    }

    /// Block a schema and record its successor for query redirection.
    /// Used during schema expansion: the old schema is blocked locally,
    /// and queries against it transparently redirect to the new schema.
    pub async fn block_and_supersede(
        &self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), SchemaError> {
        self.set_schema_state(old_name, SchemaState::Blocked)
            .await?;

        self.db_ops.store_superseded_by(old_name, new_name).await?;

        lock_map(&self.superseded_by, "superseded_by")?
            .insert(old_name.to_string(), new_name.to_string());

        Ok(())
    }

    pub fn get_message_bus(&self) -> Arc<AsyncMessageBus> {
        Arc::clone(&self.message_bus)
    }

    /// Remove all schemas belonging to the given org from both in-memory cache
    /// and persistent storage. Returns the names of removed schemas.
    ///
    /// This should be called alongside `DbOperations::purge_org_data` when
    /// purging an organization — `purge_org_data` only removes org-prefixed
    /// atom/molecule/history keys, but schemas are stored by name (not prefixed).
    pub async fn purge_org_schemas(&self, org_hash: &str) -> Result<Vec<String>, SchemaError> {
        use crate::storage::traits::TypedStore;

        // Find schemas with matching org_hash in the in-memory cache
        let names_to_remove: Vec<String> = {
            let schemas = lock_map(&self.schemas, "schemas")?;
            schemas
                .iter()
                .filter(|(_, schema)| schema.org_hash.as_deref() == Some(org_hash))
                .map(|(name, _)| name.clone())
                .collect()
        };

        // Remove from in-memory caches and persistent stores
        for name in &names_to_remove {
            lock_map(&self.schemas, "schemas")?.remove(name);
            lock_map(&self.schema_states, "schema_states")?.remove(name);

            // Delete from persistent stores (ignore errors on missing keys)
            let _ = self.db_ops.schemas_store().delete_item(name).await;
            let _ = self.db_ops.schema_states_store().delete_item(name).await;
        }

        if !names_to_remove.is_empty() {
            log::info!(
                "purged {} org schemas for org {}: {:?}",
                names_to_remove.len(),
                org_hash,
                names_to_remove
            );
        }

        Ok(names_to_remove)
    }

    /// Update an existing schema in both the database and in-memory cache.
    /// Used by ingestion to add Reference topologies after child schemas are resolved.
    pub async fn update_schema(&self, schema: &Schema) -> Result<(), SchemaError> {
        let name = &schema.name;
        self.db_ops.store_schema(name, schema).await?;
        lock_map(&self.schemas, "schemas")?.insert(name.clone(), schema.clone());
        Ok(())
    }

    pub fn get_schema_metadata(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        Ok(lock_map(&self.schemas, "schemas")?
            .get(schema_name)
            .cloned())
    }

    /// Fetches a schema by name, checking both in-memory cache and database.
    /// If the schema is `Blocked` and has a superseded-by entry, follows the chain (max 5 hops).
    /// Note: This is STRICTLY case-sensitive.
    pub async fn get_schema(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        self.get_schema_resolved(schema_name, 0).await
    }

    /// Internal helper that follows superseded-by chains with a hop counter.
    fn get_schema_resolved(
        &self,
        schema_name: &str,
        depth: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<Schema>, SchemaError>> + Send + '_>,
    > {
        let schema_name = schema_name.to_string();
        Box::pin(async move {
            let schema_name = schema_name.as_str();
            const MAX_REDIRECT_HOPS: usize = 5;

            if depth > MAX_REDIRECT_HOPS {
                return Err(SchemaError::InvalidData(format!(
                    "Superseded-by chain for schema '{}' exceeds maximum depth of {}",
                    schema_name, MAX_REDIRECT_HOPS
                )));
            }

            // Check if this schema is blocked and has a successor
            let successor = {
                let state = lock_map(&self.schema_states, "schema_states")?
                    .get(schema_name)
                    .copied();
                if state == Some(SchemaState::Blocked) {
                    lock_map(&self.superseded_by, "superseded_by")?
                        .get(schema_name)
                        .cloned()
                } else {
                    None
                }
            };

            if let Some(new_name) = successor {
                log::info!(
                    "Schema '{}' is blocked with successor, redirecting to '{}'",
                    schema_name,
                    new_name
                );
                return self.get_schema_resolved(&new_name, depth + 1).await;
            }

            // 1. Try exact match in memory
            if let Some(schema) = self.get_schema_metadata(schema_name)? {
                return Ok(Some(schema));
            }

            // 2. Try exact match in database (refresh cache if found)
            if let Some(schema) = self
                .db_ops
                .get_schema(schema_name)
                .await
                .map_err(|e| SchemaError::InvalidData(e.to_string()))?
            {
                // Update memory
                self.load_schema_internal(schema.clone()).await?;
                log::info!(
                    "Refreshed schema '{}' from database (stale cache)",
                    schema.name
                );
                return Ok(Some(schema));
            }

            Ok(None)
        }) // end Box::pin
    }

    pub async fn load_schema_internal(&self, schema: Schema) -> Result<(), SchemaError> {
        // Ensure runtime_fields are populated. Schemas arriving from the schema
        // service have runtime_fields empty (it's #[serde(skip)]). Without this
        // call, mutations fail with "Field not found in runtime_fields".
        // Only populate if empty — callers like interpret_declarative_schema may
        // have already populated and set additional state (molecule UUIDs, policies).
        let mut schema = schema;
        if schema.runtime_fields.is_empty() {
            schema.populate_runtime_fields()?;
        }

        let name = schema.name.clone();

        // Check if schema exists in database
        let existing_schema = self.db_ops.get_schema(&name).await?;

        if existing_schema.is_some() {
            // Existing schema — update the in-memory cache, but protect molecule
            // state. When the schema service returns a schema definition during
            // ingestion, it doesn't carry field_molecule_uuids. If we replaced the
            // cached schema unconditionally, molecule state would be lost and
            // subsequent mutations would create new molecules instead of appending.
            //
            // When the mutation_manager calls this after writing, the incoming
            // schema DOES have field_molecule_uuids (from sync_molecule_uuids),
            // so we allow the replacement.
            {
                let mut schemas = lock_map(&self.schemas, "schemas")?;

                let incoming_has_molecules = schema
                    .field_molecule_uuids
                    .as_ref()
                    .is_some_and(|m| !m.is_empty());

                if incoming_has_molecules {
                    // Incoming schema carries molecule state (from mutation_manager) — use it
                    schemas.insert(name.clone(), schema);
                } else if let Some(cached) = schemas.get(&name) {
                    let cached_has_molecules = cached
                        .field_molecule_uuids
                        .as_ref()
                        .is_some_and(|m| !m.is_empty());
                    if cached_has_molecules {
                        // Cached schema has molecule state, incoming doesn't — preserve cache
                        log::debug!(
                            "load_schema_internal: preserving cached molecule state for '{}'",
                            name
                        );
                    } else {
                        schemas.insert(name.clone(), schema);
                    }
                } else {
                    schemas.insert(name.clone(), schema);
                }
            } // Drop the lock before await

            // Preserve existing state from database
            let existing_state = self.db_ops.get_schema_state(&name).await?;
            let state = existing_state.unwrap_or(SchemaState::Available);
            lock_map(&self.schema_states, "schema_states")?.insert(name.clone(), state);
        } else {
            // New schema - persist to database and update in-memory caches
            self.db_ops.store_schema(&name, &schema).await?;
            self.db_ops
                .store_schema_state(&name, &SchemaState::Available)
                .await?;

            lock_map(&self.schemas, "schemas")?.insert(name.clone(), schema);
            lock_map(&self.schema_states, "schema_states")?
                .insert(name.clone(), SchemaState::Available);
        }

        Ok(())
    }

    /// Load schema from JSON string (creates Available schema)
    /// Only supports declarative schema format
    pub async fn load_schema_from_json(&self, json_str: &str) -> Result<(), SchemaError> {
        // Parse JSON string to DeclarativeSchemaDefinition
        let declarative_schema: DeclarativeSchemaDefinition = serde_json::from_str(json_str)
            .map_err(|e| {
                SchemaError::InvalidData(format!("Failed to parse declarative schema: {}", e))
            })?;

        // Validate all fields have data classifications
        if let Some(ref fields) = declarative_schema.fields {
            let unclassified: Vec<&str> = fields
                .iter()
                .filter(|f| {
                    !declarative_schema
                        .field_data_classifications
                        .contains_key(*f)
                })
                .map(|f| f.as_str())
                .collect();
            if !unclassified.is_empty() {
                return Err(SchemaError::InvalidData(format!(
                    "Schema '{}' has unclassified fields: {}. All fields must have a DataClassification.",
                    declarative_schema.name,
                    unclassified.join(", ")
                )));
            }
        }

        // Convert declarative schema to Schema
        let schema = self
            .interpret_declarative_schema(declarative_schema)
            .await?;

        // Load the schema using the existing method
        self.load_schema_internal(schema).await
    }

    /// Load schema from file (creates Available schema)
    /// Only supports declarative schema format
    pub async fn load_schema_from_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), SchemaError> {
        // Use the existing parse_schema_file method which handles declarative schemas
        if let Some(schema) = self.parse_schema_file(path.as_ref()).await? {
            self.load_schema_internal(schema).await
        } else {
            Err(SchemaError::InvalidData(
                "No schema found in file".to_string(),
            ))
        }
    }
    // ========== TRANSFORM VIEW API ==========

    /// Register a new transform view. Validates source references,
    /// checks for cycles, and persists to storage.
    pub async fn register_view(&self, view: TransformView) -> Result<(), SchemaError> {
        let view_name = view.name.clone();
        {
            let schemas = lock_map(&self.schemas, "schemas")?;
            let mut registry = self.view_registry.lock().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
            })?;

            // Check cross-registry name uniqueness: view name must not collide with a schema
            if schemas.contains_key(&view.name) {
                return Err(SchemaError::InvalidData(format!(
                    "Name '{}' is already used by a schema",
                    view.name
                )));
            }

            registry.register_view(view, |name| schemas.contains_key(name))?;
        };

        // Re-acquire to get the stored view
        let view_clone = {
            let registry = self.view_registry.lock().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
            })?;
            registry.get_view(&view_name).unwrap().clone()
        };

        // Persist view and state to storage
        self.db_ops
            .store_view(&view_clone.name, &view_clone)
            .await?;
        self.db_ops
            .store_view_state(&view_clone.name, &ViewState::Available)
            .await?;

        Ok(())
    }

    /// Get a view by name.
    pub fn get_view(&self, name: &str) -> Result<Option<TransformView>, SchemaError> {
        let registry = self.view_registry.lock().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
        })?;
        Ok(registry.get_view(name).cloned())
    }

    /// List all views with their states.
    pub fn get_views_with_states(&self) -> Result<Vec<(TransformView, ViewState)>, SchemaError> {
        let registry = self.view_registry.lock().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
        })?;
        Ok(registry
            .get_views_with_states()
            .into_iter()
            .map(|(v, s)| (v.clone(), s))
            .collect())
    }

    /// Approve a view.
    pub async fn approve_view(&self, name: &str) -> Result<(), SchemaError> {
        {
            let mut registry = self.view_registry.lock().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
            })?;
            registry.approve_view(name)?;
        }
        self.db_ops
            .store_view_state(name, &ViewState::Approved)
            .await?;
        Ok(())
    }

    /// Block a view.
    pub async fn block_view(&self, name: &str) -> Result<(), SchemaError> {
        {
            let mut registry = self.view_registry.lock().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
            })?;
            registry.block_view(name)?;
        }
        self.db_ops
            .store_view_state(name, &ViewState::Blocked)
            .await?;
        Ok(())
    }

    /// Remove a view and clean up storage.
    pub async fn remove_view(&self, name: &str) -> Result<(), SchemaError> {
        {
            let mut registry = self.view_registry.lock().map_err(|_| {
                SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
            })?;
            registry.remove_view(name)?;
        }
        self.db_ops.delete_view(name).await?;
        self.db_ops.delete_view_state(name).await?;
        self.db_ops.clear_view_cache_state(name).await?;
        Ok(())
    }

    /// Check if a name is used by either a schema or a view.
    pub fn name_exists(&self, name: &str) -> Result<bool, SchemaError> {
        let schemas = lock_map(&self.schemas, "schemas")?;
        if schemas.contains_key(name) {
            return Ok(true);
        }
        let registry = self.view_registry.lock().map_err(|_| {
            SchemaError::InvalidData("Failed to acquire view_registry lock".to_string())
        })?;
        Ok(registry.name_exists(name))
    }

    /// Get the view registry (for query/mutation integration).
    pub fn view_registry(&self) -> &Mutex<ViewRegistry> {
        &self.view_registry
    }

    /// Get the database operations (for view field state access).
    pub fn db_ops(&self) -> &Arc<crate::db_operations::DbOperations> {
        &self.db_ops
    }

    /// Creates a new SchemaCore for testing purposes with a temporary database
    #[allow(deprecated)]
    pub async fn new_for_testing() -> Result<Self, SchemaError> {
        let tmp = tempfile::TempDir::new().map_err(|e| SchemaError::InvalidData(e.to_string()))?;
        let pool = std::sync::Arc::new(crate::storage::SledPool::new(tmp.into_path()));
        let db_ops = std::sync::Arc::new(
            crate::db_operations::DbOperations::from_sled(pool)
                .await
                .map_err(|e| SchemaError::InvalidData(e.to_string()))?,
        );
        let message_bus = Arc::new(AsyncMessageBus::new());
        Self::new(db_ops, message_bus).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestSchemaBuilder;

    fn blogpost_schema_json() -> String {
        TestSchemaBuilder::new("BlogPost")
            .fields(&["title", "content", "author"])
            .range_key("publish_date")
            .build_json()
    }

    fn wordindex_schema_json() -> String {
        TestSchemaBuilder::new("BlogPostWordIndex")
            .hash_key("word")
            .range_key("publish_date")
            .build_json()
    }

    #[tokio::test]
    async fn new_for_testing_starts_with_empty_schemas() {
        let core = SchemaCore::new_for_testing().await.expect("init core");
        let schemas = core.get_schemas().expect("get_schemas");
        assert!(schemas.is_empty(), "expected no schemas at start");
    }

    #[tokio::test]
    async fn load_schema_from_json_adds_available_schema() {
        let core = SchemaCore::new_for_testing().await.expect("init core");
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("load blogpost");

        let schemas = core.get_schemas().expect("get_schemas");
        assert!(
            schemas.contains_key("BlogPost"),
            "BlogPost should be loaded"
        );

        let states = core.get_schema_states().expect("get states");
        assert_eq!(states.get("BlogPost"), Some(&SchemaState::Available));
    }

    #[tokio::test]
    async fn get_schemas_with_states_returns_default_available() {
        let core = SchemaCore::new_for_testing().await.expect("init core");
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("load blogpost");

        let schemas_with_states = core.get_schemas_with_states().expect("get with states");
        assert_eq!(schemas_with_states.len(), 1);
        let schema_entry = schemas_with_states
            .iter()
            .find(|entry| entry.name() == "BlogPost")
            .expect("BlogPost entry");
        assert_eq!(schema_entry.state, SchemaState::Available);
    }

    #[tokio::test]
    async fn load_multiple_schemas_from_json() {
        let core = SchemaCore::new_for_testing().await.expect("init core");
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("load blogpost");
        core.load_schema_from_json(&wordindex_schema_json())
            .await
            .expect("load wordindex");

        let schemas = core.get_schemas().expect("get_schemas");
        assert!(schemas.contains_key("BlogPost"));
        assert!(schemas.contains_key("BlogPostWordIndex"));

        let states = core.get_schema_states().expect("get states");
        assert_eq!(states.get("BlogPost"), Some(&SchemaState::Available));
        assert_eq!(
            states.get("BlogPostWordIndex"),
            Some(&SchemaState::Available)
        );
    }

    #[tokio::test]
    async fn load_schema_from_file_works_with_declarative_format() {
        let core = SchemaCore::new_for_testing().await.expect("init core");
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BlogPost.json");
        std::fs::write(&path, blogpost_schema_json()).expect("write schema json");

        core.load_schema_from_file(&path)
            .await
            .expect("load from file");
        let schemas = core.get_schemas().expect("get_schemas");
        assert!(schemas.contains_key("BlogPost"));
    }

    #[tokio::test]
    async fn blogpost_wordindex_sets_hashrange_keyconfig() {
        use crate::schema::types::SchemaType;

        let core = SchemaCore::new_for_testing().await.expect("init core");
        core.load_schema_from_json(&wordindex_schema_json())
            .await
            .expect("load wordindex");

        let schemas = core.get_schemas().expect("get_schemas");
        let s = schemas.get("BlogPostWordIndex").expect("schema exists");

        // Verify schema_type is HashRange
        assert_eq!(s.schema_type, SchemaType::HashRange);

        // Verify key configuration
        let key = s.key.as_ref().expect("key should be present");
        assert_eq!(key.hash_field.as_deref(), Some("word"));
        assert_eq!(key.range_field.as_deref(), Some("publish_date"));
    }

    #[tokio::test]
    async fn load_wordindex_schema_from_file() {
        let core = SchemaCore::new_for_testing().await.expect("init core");
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BlogPostWordIndex.json");
        std::fs::write(&path, wordindex_schema_json()).expect("write schema json");

        core.load_schema_from_file(&path)
            .await
            .expect("load from file");
        let schemas = core.get_schemas().expect("get_schemas");
        assert!(schemas.contains_key("BlogPostWordIndex"));
    }

    #[tokio::test]
    async fn reload_schema_derives_deterministic_molecule_uuids() {
        // Molecule UUIDs are now derived deterministically from schema_name + field_name.
        // Verify that after load, reload, the UUIDs are always the expected deterministic value.
        let core = SchemaCore::new_for_testing().await.expect("init core");

        // Load schema for the first time
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("load blogpost");

        let expected_uuid = crate::atom::deterministic_molecule_uuid("BlogPost", "title");

        // Check molecule UUID after first load
        {
            let schemas = core.schemas.lock().unwrap();
            let schema = schemas.get("BlogPost").expect("schema exists");
            let field = schema.runtime_fields.get("title").expect("title field");
            assert_eq!(
                field.common().molecule_uuid(),
                Some(&expected_uuid),
                "molecule UUID should be deterministic after first load"
            );
        }

        // Store to DB so load_schema_internal sees it exists
        let schema = {
            let schemas = core.schemas.lock().unwrap();
            schemas.get("BlogPost").unwrap().clone()
        };
        core.db_ops
            .store_schema("BlogPost", &schema)
            .await
            .expect("store schema");

        // Reload from JSON (simulates what ingestion does for each file)
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("reload blogpost");

        // Verify the deterministic molecule UUID is still correct after reload
        let schemas = core.get_schemas().expect("get_schemas");
        let schema = schemas.get("BlogPost").expect("BlogPost exists");
        let field = schema.runtime_fields.get("title").expect("title field");
        assert_eq!(
            field.common().molecule_uuid(),
            Some(&expected_uuid),
            "molecule UUID should remain deterministic after schema reload"
        );
    }

    #[tokio::test]
    async fn get_active_schemas_excludes_blocked() {
        let core = SchemaCore::new_for_testing().await.expect("init core");
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("load blogpost");
        core.load_schema_from_json(&wordindex_schema_json())
            .await
            .expect("load wordindex");

        // Approve both, then block one
        core.set_schema_state("BlogPost", SchemaState::Approved)
            .await
            .expect("approve blogpost");
        core.set_schema_state("BlogPostWordIndex", SchemaState::Approved)
            .await
            .expect("approve wordindex");
        core.block_schema("BlogPost").await.expect("block blogpost");

        // get_schemas_with_states returns all (including blocked)
        let all = core.get_schemas_with_states().expect("all schemas");
        assert_eq!(all.len(), 2);

        // get_active_schemas_with_states excludes blocked
        let active = core
            .get_active_schemas_with_states()
            .expect("active schemas");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name(), "BlogPostWordIndex");
    }

    #[tokio::test]
    async fn block_and_supersede_redirects_get_schema() {
        let core = SchemaCore::new_for_testing().await.expect("init core");
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("load blogpost");
        core.load_schema_from_json(&wordindex_schema_json())
            .await
            .expect("load wordindex");

        core.set_schema_state("BlogPost", SchemaState::Approved)
            .await
            .expect("approve");
        core.set_schema_state("BlogPostWordIndex", SchemaState::Approved)
            .await
            .expect("approve");

        // Supersede BlogPost → BlogPostWordIndex
        core.block_and_supersede("BlogPost", "BlogPostWordIndex")
            .await
            .expect("supersede");

        // get_schema("BlogPost") should redirect to BlogPostWordIndex
        let schema = core
            .get_schema("BlogPost")
            .await
            .expect("get")
            .expect("some");
        assert_eq!(schema.name, "BlogPostWordIndex");
    }

    #[tokio::test]
    async fn reload_from_store_adds_new_schemas() {
        // Create a SchemaCore, store a schema directly to Sled (bypassing
        // the in-memory cache), then verify reload_from_store picks it up.
        let tmp = tempfile::TempDir::new().expect("tmpdir");
        let pool = std::sync::Arc::new(crate::storage::SledPool::new(tmp.path().to_path_buf()));
        let db_ops = std::sync::Arc::new(
            crate::db_operations::DbOperations::from_sled(pool)
                .await
                .expect("db_ops"),
        );
        let message_bus = Arc::new(AsyncMessageBus::new());
        let core = SchemaCore::new(Arc::clone(&db_ops), Arc::clone(&message_bus))
            .await
            .expect("init core");

        // Confirm cache is empty
        assert!(core.get_schemas().unwrap().is_empty());

        // Write a schema directly to Sled (simulating what sync replay does)
        let json = r#"{
            "name": "SyncedSchema",
            "key": { "range_field": "created_at" },
            "fields": { "title": {}, "created_at": {} }
        }"#;
        let declarative: crate::schema::types::DeclarativeSchemaDefinition =
            serde_json::from_str(json).expect("parse");
        let schema = core
            .interpret_declarative_schema(declarative)
            .await
            .expect("interpret");
        db_ops
            .store_schema("SyncedSchema", &schema)
            .await
            .expect("store");
        db_ops
            .store_schema_state("SyncedSchema", &SchemaState::Approved)
            .await
            .expect("store state");

        // Cache should still be empty (we bypassed it)
        assert!(!core.get_schemas().unwrap().contains_key("SyncedSchema"));

        // Reload from store
        let added = core.reload_from_store().await.expect("reload");
        assert_eq!(added, 1);

        // Now cache should have it
        assert!(core.get_schemas().unwrap().contains_key("SyncedSchema"));
        assert_eq!(
            core.get_schema_states()
                .unwrap()
                .get("SyncedSchema")
                .copied(),
            Some(SchemaState::Approved)
        );
    }

    #[tokio::test]
    async fn reload_from_store_preserves_existing() {
        // Verify that reload_from_store does NOT overwrite schemas already in cache.
        let tmp = tempfile::TempDir::new().expect("tmpdir");
        let pool = std::sync::Arc::new(crate::storage::SledPool::new(tmp.path().to_path_buf()));
        let db_ops = std::sync::Arc::new(
            crate::db_operations::DbOperations::from_sled(pool)
                .await
                .expect("db_ops"),
        );
        let message_bus = Arc::new(AsyncMessageBus::new());
        let core = SchemaCore::new(Arc::clone(&db_ops), Arc::clone(&message_bus))
            .await
            .expect("init core");

        // Load a schema normally (into both cache and Sled)
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("load");

        // Reload should add 0 new schemas (BlogPost is already in cache)
        let added = core.reload_from_store().await.expect("reload");
        assert_eq!(added, 0);

        // Still exactly one schema
        assert_eq!(core.get_schemas().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn reload_from_store_populates_runtime_fields() {
        // Schemas written to Sled by sync replay have runtime_fields = {} (#[serde(skip)]).
        // Verify that reload_from_store populates runtime_fields on load.
        let tmp = tempfile::TempDir::new().expect("tmpdir");
        let pool = std::sync::Arc::new(crate::storage::SledPool::new(tmp.path().to_path_buf()));
        let db_ops = std::sync::Arc::new(
            crate::db_operations::DbOperations::from_sled(pool)
                .await
                .expect("db_ops"),
        );
        let message_bus = Arc::new(AsyncMessageBus::new());
        let core = SchemaCore::new(Arc::clone(&db_ops), Arc::clone(&message_bus))
            .await
            .expect("init core");

        // Build a schema, then store it directly to Sled
        let json = r#"{
            "name": "RuntimeTest",
            "key": { "range_field": "ts" },
            "fields": { "content": {}, "ts": {} }
        }"#;
        let declarative: crate::schema::types::DeclarativeSchemaDefinition =
            serde_json::from_str(json).expect("parse");
        let schema = core
            .interpret_declarative_schema(declarative)
            .await
            .expect("interpret");
        db_ops
            .store_schema("RuntimeTest", &schema)
            .await
            .expect("store");

        // Reload
        let added = core.reload_from_store().await.expect("reload");
        assert_eq!(added, 1);

        // Verify runtime_fields are populated
        let schemas = core.get_schemas().unwrap();
        let s = schemas.get("RuntimeTest").expect("exists");
        assert!(
            !s.runtime_fields.is_empty(),
            "runtime_fields should be populated after reload"
        );
        assert!(
            s.runtime_fields.contains_key("content"),
            "runtime_fields should contain 'content'"
        );
    }
}
