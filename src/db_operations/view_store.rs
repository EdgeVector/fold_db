//! Transform-view domain store.
//!
//! Owns the namespaces for transform view definitions, view states,
//! and the local-only transform cache. External callers reach these via
//! `DbOperations::views()`.
//!
//! The `transform_cache` namespace is declared local-only in
//! `SyncingNamespacedStore::LOCAL_ONLY_NAMESPACES` — writes to it never
//! append to the sync log. Cached transform output is derived per-device
//! and must not cross the wire (see `docs/design/multi_device_transforms.md`,
//! "What Syncs vs. What Doesn't").

use crate::schema::SchemaError;
use crate::storage::traits::{KvStore, TypedStore};
use crate::storage::TypedKvStore;
use crate::view::registry::ViewState;
use crate::view::transform_field_override::TransformFieldOverride;
use crate::view::types::{TransformView, ViewCacheState};
use std::collections::HashMap;
use std::sync::Arc;

/// Domain store for transform view persistence.
#[derive(Clone)]
pub struct ViewStore {
    views_store: Arc<TypedKvStore<dyn KvStore>>,
    view_states_store: Arc<TypedKvStore<dyn KvStore>>,
    /// Local-only cache of computed `ViewCacheState` per view. Routed
    /// through the `transform_cache` namespace, which `SyncingNamespacedStore`
    /// excludes from the sync log.
    transform_cache_store: Arc<TypedKvStore<dyn KvStore>>,
    /// Per-(view, field, key) override molecules. Synced like any other
    /// molecule write — converges across replicas via LWW on `written_at`.
    transform_field_overrides_store: Arc<TypedKvStore<dyn KvStore>>,
}

impl ViewStore {
    pub(crate) fn new(
        views_store: Arc<TypedKvStore<dyn KvStore>>,
        view_states_store: Arc<TypedKvStore<dyn KvStore>>,
        transform_cache_store: Arc<TypedKvStore<dyn KvStore>>,
        transform_field_overrides_store: Arc<TypedKvStore<dyn KvStore>>,
    ) -> Self {
        Self {
            views_store,
            view_states_store,
            transform_cache_store,
            transform_field_overrides_store,
        }
    }

    /// Crate-internal raw handles (for org purge).
    pub(crate) fn raw_views(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.views_store
    }

    pub(crate) fn raw_view_states(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.view_states_store
    }

    pub(crate) fn raw_transform_cache(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.transform_cache_store
    }

    /// Store a transform view definition.
    pub async fn store_view(
        &self,
        view_name: &str,
        view: &TransformView,
    ) -> Result<(), SchemaError> {
        self.views_store.put_item(view_name, view).await?;
        self.views_store.inner().flush().await?;
        Ok(())
    }

    /// Get a transform view by name.
    pub async fn get_view(&self, view_name: &str) -> Result<Option<TransformView>, SchemaError> {
        Ok(self.views_store.get_item(view_name).await?)
    }

    /// Get all transform views.
    pub async fn get_all_views(&self) -> Result<Vec<TransformView>, SchemaError> {
        let items: Vec<(String, TransformView)> =
            self.views_store.scan_items_with_prefix("").await?;
        Ok(items.into_iter().map(|(_, v)| v).collect())
    }

    /// Delete a transform view.
    pub async fn delete_view(&self, view_name: &str) -> Result<(), SchemaError> {
        self.views_store.delete_item(view_name).await?;
        self.views_store.inner().flush().await?;
        Ok(())
    }

    /// Store a view state.
    pub async fn store_view_state(
        &self,
        view_name: &str,
        state: &ViewState,
    ) -> Result<(), SchemaError> {
        self.view_states_store.put_item(view_name, state).await?;
        self.view_states_store.inner().flush().await?;
        Ok(())
    }

    /// Get all view states.
    pub async fn get_all_view_states(&self) -> Result<HashMap<String, ViewState>, SchemaError> {
        let items: Vec<(String, ViewState)> =
            self.view_states_store.scan_items_with_prefix("").await?;
        Ok(items.into_iter().collect())
    }

    /// Delete a view state.
    pub async fn delete_view_state(&self, view_name: &str) -> Result<(), SchemaError> {
        self.view_states_store.delete_item(view_name).await?;
        self.view_states_store.inner().flush().await?;
        Ok(())
    }

    /// Get the cache state for an entire view.
    pub async fn get_view_cache_state(
        &self,
        view_name: &str,
    ) -> Result<ViewCacheState, SchemaError> {
        Ok(self
            .transform_cache_store
            .get_item::<ViewCacheState>(view_name)
            .await?
            .unwrap_or(ViewCacheState::Empty))
    }

    /// Set the cache state for an entire view.
    pub async fn set_view_cache_state(
        &self,
        view_name: &str,
        state: &ViewCacheState,
    ) -> Result<(), SchemaError> {
        self.transform_cache_store
            .put_item(view_name, state)
            .await?;
        self.transform_cache_store.inner().flush().await?;
        Ok(())
    }

    /// Clear cache state for a view (used when removing a view).
    pub async fn clear_view_cache_state(&self, view_name: &str) -> Result<(), SchemaError> {
        self.transform_cache_store.delete_item(view_name).await?;
        self.transform_cache_store.inner().flush().await?;
        Ok(())
    }

    // ===== Transform field overrides =====
    //
    // Per-(view, field, key_value) override molecules. Stored in their own
    // namespace so they participate in the unified sync log like any other
    // molecule write — converging across replicas via LWW on `written_at`.

    /// Build the storage key for an override entry. The view name and field
    /// name come from registered schemas (already constrained to identifier
    /// characters), so plain `|` separation is safe; key_str is opaque user
    /// input but lives at the tail and never participates in prefix scans
    /// past the field segment.
    fn override_key(view_name: &str, field_name: &str, key_str: &str) -> String {
        format!("{}|{}|{}", view_name, field_name, key_str)
    }

    /// Prefix that scans all overrides for a view.
    fn override_view_prefix(view_name: &str) -> String {
        format!("{}|", view_name)
    }

    /// Read an override for a single (view, field, key) tuple, if any.
    pub async fn get_transform_field_override(
        &self,
        view_name: &str,
        field_name: &str,
        key_str: &str,
    ) -> Result<Option<TransformFieldOverride>, SchemaError> {
        let key = Self::override_key(view_name, field_name, key_str);
        Ok(self
            .transform_field_overrides_store
            .get_item::<TransformFieldOverride>(&key)
            .await?)
    }

    /// LWW-write an override. Returns whether the on-disk state changed.
    /// If an existing entry is at least as new as `incoming`, the write is
    /// dropped — this is what makes replay of older log entries idempotent.
    pub async fn put_transform_field_override(
        &self,
        view_name: &str,
        field_name: &str,
        key_str: &str,
        incoming: &TransformFieldOverride,
    ) -> Result<bool, SchemaError> {
        let key = Self::override_key(view_name, field_name, key_str);

        if let Some(existing) = self
            .transform_field_overrides_store
            .get_item::<TransformFieldOverride>(&key)
            .await?
        {
            if !TransformFieldOverride::should_replace(&existing, incoming) {
                return Ok(false);
            }
        }

        self.transform_field_overrides_store
            .put_item(&key, incoming)
            .await?;
        self.transform_field_overrides_store.inner().flush().await?;
        Ok(true)
    }

    /// Scan all overrides for a view. Returns (field_name, key_str, override)
    /// tuples — caller groups by field as needed.
    pub async fn scan_transform_field_overrides(
        &self,
        view_name: &str,
    ) -> Result<Vec<(String, String, TransformFieldOverride)>, SchemaError> {
        let prefix = Self::override_view_prefix(view_name);
        let items: Vec<(String, TransformFieldOverride)> = self
            .transform_field_overrides_store
            .scan_items_with_prefix(&prefix)
            .await?;

        let mut out = Vec::with_capacity(items.len());
        for (raw_key, value) in items {
            // raw_key = "{view}|{field}|{key_str}". Splitting on the first
            // two `|` separators gives back the components; the trailing
            // `key_str` is preserved verbatim even if it contains `|`.
            let after_view = raw_key.strip_prefix(&prefix).ok_or_else(|| {
                SchemaError::InvalidData(format!(
                    "override key '{}' missing prefix '{}'",
                    raw_key, prefix
                ))
            })?;
            let mut parts = after_view.splitn(2, '|');
            let field = parts
                .next()
                .ok_or_else(|| {
                    SchemaError::InvalidData(format!("override key '{}' missing field", raw_key))
                })?
                .to_string();
            let key_str = parts.next().unwrap_or("").to_string();
            out.push((field, key_str, value));
        }
        Ok(out)
    }

    /// Delete every override for a view (used when the view is removed).
    pub async fn clear_transform_field_overrides(
        &self,
        view_name: &str,
    ) -> Result<(), SchemaError> {
        let prefix = Self::override_view_prefix(view_name);
        let keys = self
            .transform_field_overrides_store
            .list_keys_with_prefix(&prefix)
            .await?;
        if keys.is_empty() {
            return Ok(());
        }
        self.transform_field_overrides_store
            .batch_delete_keys(keys)
            .await?;
        self.transform_field_overrides_store.inner().flush().await?;
        Ok(())
    }

    pub(crate) fn raw_transform_field_overrides(&self) -> &Arc<TypedKvStore<dyn KvStore>> {
        &self.transform_field_overrides_store
    }
}

#[cfg(test)]
mod override_tests {
    use super::*;
    use crate::db_operations::DbOperations;
    use crate::storage::SledPool;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn fresh_store() -> (TempDir, ViewStore) {
        let tmp = TempDir::new().unwrap();
        let pool = Arc::new(SledPool::new(tmp.path().to_path_buf()));
        let ops = DbOperations::from_sled(pool).await.unwrap();
        (tmp, ops.views().clone())
    }

    #[tokio::test]
    async fn put_get_round_trip() {
        let (_tmp, store) = fresh_store().await;
        let o = TransformFieldOverride::with_timestamp(json!("hello"), "pkA", 100);

        let wrote = store
            .put_transform_field_override("V", "f", "k1", &o)
            .await
            .unwrap();
        assert!(wrote);

        let got = store
            .get_transform_field_override("V", "f", "k1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got, o);
    }

    #[tokio::test]
    async fn lww_newer_wins() {
        let (_tmp, store) = fresh_store().await;
        let older = TransformFieldOverride::with_timestamp(json!("a"), "pkA", 100);
        let newer = TransformFieldOverride::with_timestamp(json!("b"), "pkB", 101);

        // Order doesn't matter — older then newer:
        assert!(store
            .put_transform_field_override("V", "f", "k", &older)
            .await
            .unwrap());
        assert!(store
            .put_transform_field_override("V", "f", "k", &newer)
            .await
            .unwrap());
        let got = store
            .get_transform_field_override("V", "f", "k")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.value, json!("b"));
        assert_eq!(got.written_at, 101);
    }

    #[tokio::test]
    async fn lww_older_dropped() {
        let (_tmp, store) = fresh_store().await;
        let older = TransformFieldOverride::with_timestamp(json!("a"), "pkA", 100);
        let newer = TransformFieldOverride::with_timestamp(json!("b"), "pkB", 101);

        assert!(store
            .put_transform_field_override("V", "f", "k", &newer)
            .await
            .unwrap());
        // Replaying the older entry must not regress the value.
        let wrote_old = store
            .put_transform_field_override("V", "f", "k", &older)
            .await
            .unwrap();
        assert!(!wrote_old, "older write should be no-op");
        let got = store
            .get_transform_field_override("V", "f", "k")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.value, json!("b"));
    }

    #[tokio::test]
    async fn scan_returns_all_overrides_for_view() {
        let (_tmp, store) = fresh_store().await;
        let o1 = TransformFieldOverride::with_timestamp(json!(1), "pk", 1);
        let o2 = TransformFieldOverride::with_timestamp(json!(2), "pk", 2);
        let other = TransformFieldOverride::with_timestamp(json!(99), "pk", 1);
        store
            .put_transform_field_override("V", "f1", "k1", &o1)
            .await
            .unwrap();
        store
            .put_transform_field_override("V", "f2", "k2", &o2)
            .await
            .unwrap();
        store
            .put_transform_field_override("OtherView", "f1", "k1", &other)
            .await
            .unwrap();

        let mut entries = store.scan_transform_field_overrides("V").await.unwrap();
        entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "f1");
        assert_eq!(entries[0].1, "k1");
        assert_eq!(entries[0].2.value, json!(1));
        assert_eq!(entries[1].0, "f2");
        assert_eq!(entries[1].1, "k2");
    }

    #[tokio::test]
    async fn clear_removes_only_target_view() {
        let (_tmp, store) = fresh_store().await;
        let o = TransformFieldOverride::with_timestamp(json!(1), "pk", 1);
        store
            .put_transform_field_override("V", "f", "k", &o)
            .await
            .unwrap();
        store
            .put_transform_field_override("OtherView", "f", "k", &o)
            .await
            .unwrap();

        store.clear_transform_field_overrides("V").await.unwrap();

        assert!(store
            .get_transform_field_override("V", "f", "k")
            .await
            .unwrap()
            .is_none());
        assert!(store
            .get_transform_field_override("OtherView", "f", "k")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn key_with_pipe_round_trips() {
        // Real range keys can include `|` (e.g. composite IDs). The split
        // on `|` for scan must preserve the trailing key verbatim.
        let (_tmp, store) = fresh_store().await;
        let o = TransformFieldOverride::with_timestamp(json!("v"), "pk", 1);
        store
            .put_transform_field_override("V", "f", "a|b|c", &o)
            .await
            .unwrap();

        let got = store
            .get_transform_field_override("V", "f", "a|b|c")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got, o);

        let scanned = store.scan_transform_field_overrides("V").await.unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].0, "f");
        assert_eq!(scanned[0].1, "a|b|c");
    }
}
