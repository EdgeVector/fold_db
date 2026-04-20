//! Schema domain store.
//!
//! Owns the storage namespaces for schemas, schema states, and
//! schema supersede-by mappings. External callers access schema
//! operations through this type via `DbOperations::schemas()`.

use crate::schema::{Schema, SchemaError, SchemaState};
use crate::storage::traits::{KvStore, TypedStore};
use crate::storage::TypedKvStore;
use crate::sync::org_sync::strip_org_prefix;
use std::collections::HashMap;
use std::sync::Arc;

/// Domain store for schema-related persistence.
#[derive(Clone)]
pub struct SchemaStore {
    schemas_store: Arc<TypedKvStore<dyn KvStore>>,
    schema_states_store: Arc<TypedKvStore<dyn KvStore>>,
    superseded_by_store: Arc<TypedKvStore<dyn KvStore>>,
}

impl SchemaStore {
    pub(crate) fn new(
        schemas_store: Arc<TypedKvStore<dyn KvStore>>,
        schema_states_store: Arc<TypedKvStore<dyn KvStore>>,
        superseded_by_store: Arc<TypedKvStore<dyn KvStore>>,
    ) -> Self {
        Self {
            schemas_store,
            schema_states_store,
            superseded_by_store,
        }
    }

    /// Access the raw schemas namespace. Restricted to other modules inside
    /// `fold_db` that need generic typed access (e.g. org purge).
    pub(crate) fn raw_schemas(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.schemas_store
    }

    /// Access the raw schema-states namespace (crate-internal).
    pub(crate) fn raw_schema_states(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.schema_states_store
    }

    /// Access the raw superseded-by namespace (crate-internal).
    pub(crate) fn raw_superseded_by(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.superseded_by_store
    }

    /// Get a specific schema by name
    pub async fn get_schema(&self, schema_name: &str) -> Result<Option<Schema>, SchemaError> {
        let mut schema_opt: Option<Schema> = self.schemas_store.get_item(schema_name).await?;

        // Populate runtime_fields if schema exists
        if let Some(schema) = &mut schema_opt {
            schema.populate_runtime_fields()?;
        }

        Ok(schema_opt)
    }

    /// Get the state of a specific schema
    pub async fn get_schema_state(
        &self,
        schema_name: &str,
    ) -> Result<Option<SchemaState>, SchemaError> {
        Ok(self.schema_states_store.get_item(schema_name).await?)
    }

    /// Store a schema.
    ///
    /// When the schema carries an `org_hash`, the entry is ALSO written under
    /// `{org_hash}:{schema_name}` so `SyncPartitioner` routes that copy to the
    /// org sync log. Without the dual-write, the partitioner only ever sees
    /// the bare key and the schema never reaches org peers, leaving them with
    /// orphaned molecules (alpha BLOCKER af4ba).
    pub async fn store_schema(
        &self,
        schema_name: &str,
        schema: &Schema,
    ) -> Result<(), SchemaError> {
        self.schemas_store.put_item(schema_name, schema).await?;
        if let Some(org_hash) = schema.org_hash.as_deref() {
            let org_key = format!("{org_hash}:{schema_name}");
            self.schemas_store.put_item(&org_key, schema).await?;
        }
        self.schemas_store.inner().flush().await?;
        Ok(())
    }

    /// Store schema state.
    ///
    /// Mirrors `store_schema`'s dual-write when the schema carries an
    /// `org_hash` so peers receive the approval state alongside the schema
    /// body. The `org_hash` is resolved from the stored schema.
    pub async fn store_schema_state(
        &self,
        schema_name: &str,
        state: &SchemaState,
    ) -> Result<(), SchemaError> {
        self.schema_states_store
            .put_item(schema_name, state)
            .await?;
        if let Some(schema) = self.schemas_store.get_item::<Schema>(schema_name).await? {
            if let Some(org_hash) = schema.org_hash.as_deref() {
                let org_key = format!("{org_hash}:{schema_name}");
                self.schema_states_store.put_item(&org_key, state).await?;
            }
        }
        self.schema_states_store.inner().flush().await?;
        Ok(())
    }

    /// Get all schemas.
    ///
    /// Org-prefixed keys (`{org_hash}:{name}`) are sync routing duplicates of
    /// the bare-key entry and are filtered out so callers see each schema
    /// exactly once, under its real name.
    pub async fn get_all_schemas(&self) -> Result<HashMap<String, Schema>, SchemaError> {
        let items: Vec<(String, Schema)> = self.schemas_store.scan_items_with_prefix("").await?;

        let mut schemas = HashMap::with_capacity(items.len());
        for (key, mut schema) in items {
            if strip_org_prefix(&key).is_some() {
                continue;
            }
            schema.populate_runtime_fields()?;
            schemas.insert(key, schema);
        }

        Ok(schemas)
    }

    /// Store a schema superseded-by mapping (old_name → new_name)
    pub async fn store_superseded_by(
        &self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), SchemaError> {
        self.superseded_by_store
            .put_item(old_name, &new_name.to_string())
            .await?;
        self.superseded_by_store.inner().flush().await?;
        Ok(())
    }

    /// Get all superseded-by mappings
    pub async fn get_all_superseded_by(&self) -> Result<HashMap<String, String>, SchemaError> {
        let items: Vec<(String, String)> =
            self.superseded_by_store.scan_items_with_prefix("").await?;
        Ok(items.into_iter().collect())
    }

    /// Get all schema states.
    ///
    /// Org-prefixed keys are filtered out for the same reason as
    /// `get_all_schemas`.
    pub async fn get_all_schema_states(&self) -> Result<HashMap<String, SchemaState>, SchemaError> {
        let items: Vec<(String, SchemaState)> =
            self.schema_states_store.scan_items_with_prefix("").await?;
        Ok(items
            .into_iter()
            .filter(|(k, _)| strip_org_prefix(k).is_none())
            .collect())
    }

    /// Delete a schema entry (name only) from the schemas namespace.
    ///
    /// Resolves `org_hash` from the stored schema and also deletes the
    /// org-prefixed companion entry so the sync routing copy does not linger.
    pub async fn delete_schema(&self, schema_name: &str) -> Result<(), SchemaError> {
        let org_hash = self
            .schemas_store
            .get_item::<Schema>(schema_name)
            .await?
            .and_then(|s| s.org_hash);
        self.schemas_store.delete_item(schema_name).await?;
        if let Some(org_hash) = org_hash {
            let org_key = format!("{org_hash}:{schema_name}");
            self.schemas_store.delete_item(&org_key).await?;
        }
        Ok(())
    }

    /// Delete a schema state entry.
    ///
    /// Resolves `org_hash` from the stored schema when it still exists, so
    /// the companion org-prefixed state entry is also removed. Callers that
    /// have already deleted the schema first must invoke `delete_schema`
    /// (which handles both namespaces transitively) instead.
    pub async fn delete_schema_state(&self, schema_name: &str) -> Result<(), SchemaError> {
        let org_hash = self
            .schemas_store
            .get_item::<Schema>(schema_name)
            .await?
            .and_then(|s| s.org_hash);
        self.schema_states_store.delete_item(schema_name).await?;
        if let Some(org_hash) = org_hash {
            let org_key = format!("{org_hash}:{schema_name}");
            self.schema_states_store.delete_item(&org_key).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Schema;
    use crate::schema::SchemaState;
    use crate::storage::inmemory_backend::InMemoryNamespacedStore;
    use crate::storage::traits::NamespacedStore;

    async fn build_store() -> SchemaStore {
        let backend = Arc::new(InMemoryNamespacedStore::new()) as Arc<dyn NamespacedStore>;
        let schemas = Arc::new(TypedKvStore::new(
            backend.open_namespace("schemas").await.unwrap(),
        ));
        let states = Arc::new(TypedKvStore::new(
            backend.open_namespace("schema_states").await.unwrap(),
        ));
        let superseded = Arc::new(TypedKvStore::new(
            backend.open_namespace("superseded_by").await.unwrap(),
        ));
        SchemaStore::new(schemas, states, superseded)
    }

    fn build_schema(name: &str, org_hash: Option<&str>) -> Schema {
        let mut v = serde_json::json!({
            "name": name,
            "schema_type": "Single",
            "fields": ["pk"],
            "field_data_classifications": {
                "pk": { "sensitivity_level": 0, "data_domain": "general" }
            }
        });
        if let Some(h) = org_hash {
            v["org_hash"] = serde_json::json!(h);
            v["trust_domain"] = serde_json::json!(format!("org:{h}"));
        }
        serde_json::from_value(v).expect("test schema must deserialize")
    }

    #[tokio::test]
    async fn store_schema_dual_writes_when_org_hash_set() {
        let store = build_store().await;
        let org_hash = "a".repeat(64);
        let schema = build_schema("org_notes", Some(&org_hash));
        store.store_schema("org_notes", &schema).await.unwrap();

        let bare: Option<Schema> = store.schemas_store.get_item("org_notes").await.unwrap();
        assert!(bare.is_some(), "bare key must be written");

        let org_key = format!("{}:org_notes", org_hash);
        let prefixed: Option<Schema> = store.schemas_store.get_item(&org_key).await.unwrap();
        assert!(
            prefixed.is_some(),
            "org-prefixed key must be written when org_hash is set so SyncPartitioner routes the copy to the org log"
        );
    }

    #[tokio::test]
    async fn store_schema_personal_writes_only_bare_key() {
        let store = build_store().await;
        let schema = build_schema("personal_notes", None);
        store.store_schema("personal_notes", &schema).await.unwrap();

        let items: Vec<(String, Schema)> = store
            .schemas_store
            .scan_items_with_prefix("")
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "personal_notes");
    }

    #[tokio::test]
    async fn get_all_schemas_filters_out_org_prefixed_duplicates() {
        let store = build_store().await;
        let org_hash = "b".repeat(64);
        let schema = build_schema("shared_notes", Some(&org_hash));
        store.store_schema("shared_notes", &schema).await.unwrap();

        let all = store.get_all_schemas().await.unwrap();
        assert_eq!(
            all.len(),
            1,
            "org-prefixed companion must not surface as a second schema"
        );
        assert!(
            all.contains_key("shared_notes"),
            "bare name must be the only visible key"
        );
    }

    #[tokio::test]
    async fn store_schema_state_dual_writes_for_org_tagged_schema() {
        let store = build_store().await;
        let org_hash = "c".repeat(64);
        let schema = build_schema("org_notes", Some(&org_hash));
        store.store_schema("org_notes", &schema).await.unwrap();

        store
            .store_schema_state("org_notes", &SchemaState::Approved)
            .await
            .unwrap();

        let org_key = format!("{}:org_notes", org_hash);
        let bare: Option<SchemaState> = store
            .schema_states_store
            .get_item("org_notes")
            .await
            .unwrap();
        let prefixed: Option<SchemaState> =
            store.schema_states_store.get_item(&org_key).await.unwrap();
        assert_eq!(bare, Some(SchemaState::Approved));
        assert_eq!(
            prefixed,
            Some(SchemaState::Approved),
            "org-prefixed state companion must be written so peers receive approval"
        );
    }

    #[tokio::test]
    async fn get_all_schema_states_filters_out_org_prefixed_duplicates() {
        let store = build_store().await;
        let org_hash = "d".repeat(64);
        let schema = build_schema("org_notes", Some(&org_hash));
        store.store_schema("org_notes", &schema).await.unwrap();
        store
            .store_schema_state("org_notes", &SchemaState::Approved)
            .await
            .unwrap();

        let states = store.get_all_schema_states().await.unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states.get("org_notes"), Some(&SchemaState::Approved));
    }

    #[tokio::test]
    async fn delete_schema_removes_both_bare_and_org_prefixed_entries() {
        let store = build_store().await;
        let org_hash = "e".repeat(64);
        let schema = build_schema("org_notes", Some(&org_hash));
        store.store_schema("org_notes", &schema).await.unwrap();

        store.delete_schema("org_notes").await.unwrap();

        let items: Vec<(String, Schema)> = store
            .schemas_store
            .scan_items_with_prefix("")
            .await
            .unwrap();
        assert!(
            items.is_empty(),
            "both bare and org-prefixed keys must be removed, got {:?}",
            items
        );
    }
}
