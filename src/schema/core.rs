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
        Ok(all.into_iter().filter(|s| s.state != SchemaState::Blocked).collect())
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
                            source_schema_name, e
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
            lock_map(&self.schemas, "schemas")?
                .insert(schema_name.to_string(), schema);
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

        self.db_ops
            .store_superseded_by(old_name, new_name)
            .await?;

        lock_map(&self.superseded_by, "superseded_by")?
            .insert(old_name.to_string(), new_name.to_string());

        Ok(())
    }

    pub fn get_message_bus(&self) -> Arc<AsyncMessageBus> {
        Arc::clone(&self.message_bus)
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
        Ok(lock_map(&self.schemas, "schemas")?.get(schema_name).cloned())
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<Schema>, SchemaError>> + Send + '_>> {
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

                let incoming_has_molecules = schema.field_molecule_uuids
                    .as_ref()
                    .is_some_and(|m| !m.is_empty());

                if incoming_has_molecules {
                    // Incoming schema carries molecule state (from mutation_manager) — use it
                    schemas.insert(name.clone(), schema);
                } else if let Some(cached) = schemas.get(&name) {
                    let cached_has_molecules = cached.field_molecule_uuids
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
            lock_map(&self.schema_states, "schema_states")?
                .insert(name.clone(), state);
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
            let mut registry = self
                .view_registry
                .lock()
                .map_err(|_| SchemaError::InvalidData("Failed to acquire view_registry lock".to_string()))?;

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
            let registry = self
                .view_registry
                .lock()
                .map_err(|_| SchemaError::InvalidData("Failed to acquire view_registry lock".to_string()))?;
            registry.get_view(&view_name).unwrap().clone()
        };

        // Persist view and state to storage
        self.db_ops.store_view(&view_clone.name, &view_clone).await?;
        self.db_ops
            .store_view_state(&view_clone.name, &ViewState::Available)
            .await?;

        Ok(())
    }

    /// Get a view by name.
    pub fn get_view(&self, name: &str) -> Result<Option<TransformView>, SchemaError> {
        let registry = self
            .view_registry
            .lock()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire view_registry lock".to_string()))?;
        Ok(registry.get_view(name).cloned())
    }

    /// List all views with their states.
    pub fn get_views_with_states(&self) -> Result<Vec<(TransformView, ViewState)>, SchemaError> {
        let registry = self
            .view_registry
            .lock()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire view_registry lock".to_string()))?;
        Ok(registry
            .get_views_with_states()
            .into_iter()
            .map(|(v, s)| (v.clone(), s))
            .collect())
    }

    /// Approve a view.
    pub async fn approve_view(&self, name: &str) -> Result<(), SchemaError> {
        {
            let mut registry = self
                .view_registry
                .lock()
                .map_err(|_| SchemaError::InvalidData("Failed to acquire view_registry lock".to_string()))?;
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
            let mut registry = self
                .view_registry
                .lock()
                .map_err(|_| SchemaError::InvalidData("Failed to acquire view_registry lock".to_string()))?;
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
            let mut registry = self
                .view_registry
                .lock()
                .map_err(|_| SchemaError::InvalidData("Failed to acquire view_registry lock".to_string()))?;
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
        let registry = self
            .view_registry
            .lock()
            .map_err(|_| SchemaError::InvalidData("Failed to acquire view_registry lock".to_string()))?;
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
    pub async fn new_for_testing() -> Result<Self, SchemaError> {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .map_err(|e| SchemaError::InvalidData(e.to_string()))?;
        let db_ops = std::sync::Arc::new(
            crate::db_operations::DbOperations::from_sled(
                db,
                Arc::new(crate::db_operations::native_index::FastEmbedModel::new()),
            )
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

    fn blogpost_schema_json() -> String {
        // Declarative schema format used in available_schemas
        // Minimal fields map is acceptable per current parser
        r#"{
            "name": "BlogPost",
            "key": { "range_field": "publish_date" },
            "fields": {
                "title": {},
                "content": {},
                "author": {},
                "publish_date": {}
            }
        }"#
        .to_string()
    }

    fn wordindex_schema_json() -> String {
        r#"{
            "name": "BlogPostWordIndex",
            "key": { "hash_field": "word", "range_field": "publish_date" },
            "fields": {
                "word": {},
                "publish_date": {}
            }
        }"#
        .to_string()
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
    async fn reload_schema_from_json_preserves_molecule_uuids() {
        // Simulate the bug: load a schema, set molecule UUIDs (as mutation_manager does),
        // then reload from JSON (as ingestion does for each file). The molecule UUIDs
        // on the cached schema should survive the reload.
        let core = SchemaCore::new_for_testing().await.expect("init core");

        // Load schema for the first time
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("load blogpost");

        // Simulate what mutation_manager does: set molecule_uuid on a runtime field
        // and sync to field_molecule_uuids
        {
            let mut schemas = core.schemas.lock().unwrap();
            let schema = schemas.get_mut("BlogPost").expect("schema exists");
            let field = schema
                .runtime_fields
                .get_mut("title")
                .expect("title field");
            field.common_mut().set_molecule_uuid("mol-uuid-title".to_string());
            schema.sync_molecule_uuids();

            // Persist to DB so load_schema_internal sees it exists
        }
        // Also store to DB
        let schema = {
            let schemas = core.schemas.lock().unwrap();
            schemas.get("BlogPost").unwrap().clone()
        };
        core.db_ops
            .store_schema("BlogPost", &schema)
            .await
            .expect("store schema");

        // Now reload from JSON (simulates what ingestion does for each file)
        // The JSON from the schema service does NOT have field_molecule_uuids
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .expect("reload blogpost");

        // Verify the molecule UUID survived the reload
        let schemas = core.get_schemas().expect("get_schemas");
        let schema = schemas.get("BlogPost").expect("BlogPost exists");

        // The persisted field_molecule_uuids should still have our molecule UUID
        // because load_schema_internal should preserve existing state
        let mol_uuids = schema
            .field_molecule_uuids
            .as_ref()
            .expect("field_molecule_uuids should exist");
        assert_eq!(
            mol_uuids.get("title"),
            Some(&"mol-uuid-title".to_string()),
            "molecule UUID for title should survive schema reload"
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
        let active = core.get_active_schemas_with_states().expect("active schemas");
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
        let schema = core.get_schema("BlogPost").await.expect("get").expect("some");
        assert_eq!(schema.name, "BlogPostWordIndex");
    }
}
