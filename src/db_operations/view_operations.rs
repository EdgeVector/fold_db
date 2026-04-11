use super::core::DbOperations;
use crate::schema::SchemaError;
use crate::storage::traits::TypedStore;
use crate::view::registry::ViewState;
use crate::view::types::{TransformView, ViewCacheState};
use std::collections::HashMap;

impl DbOperations {
    /// Store a transform view definition.
    pub async fn store_view(
        &self,
        view_name: &str,
        view: &TransformView,
    ) -> Result<(), SchemaError> {
        self.views_store().put_item(view_name, view).await?;
        self.views_store().inner().flush().await?;
        Ok(())
    }

    /// Get a transform view by name.
    pub async fn get_view(&self, view_name: &str) -> Result<Option<TransformView>, SchemaError> {
        Ok(self.views_store().get_item(view_name).await?)
    }

    /// Get all transform views.
    pub async fn get_all_views(&self) -> Result<Vec<TransformView>, SchemaError> {
        let items: Vec<(String, TransformView)> =
            self.views_store().scan_items_with_prefix("").await?;
        Ok(items.into_iter().map(|(_, v)| v).collect())
    }

    /// Delete a transform view.
    pub async fn delete_view(&self, view_name: &str) -> Result<(), SchemaError> {
        self.views_store().delete_item(view_name).await?;
        self.views_store().inner().flush().await?;
        Ok(())
    }

    /// Store a view state.
    pub async fn store_view_state(
        &self,
        view_name: &str,
        state: &ViewState,
    ) -> Result<(), SchemaError> {
        self.view_states_store().put_item(view_name, state).await?;
        self.view_states_store().inner().flush().await?;
        Ok(())
    }

    /// Get all view states.
    pub async fn get_all_view_states(&self) -> Result<HashMap<String, ViewState>, SchemaError> {
        let items: Vec<(String, ViewState)> =
            self.view_states_store().scan_items_with_prefix("").await?;
        Ok(items.into_iter().collect())
    }

    /// Delete a view state.
    pub async fn delete_view_state(&self, view_name: &str) -> Result<(), SchemaError> {
        self.view_states_store().delete_item(view_name).await?;
        self.view_states_store().inner().flush().await?;
        Ok(())
    }

    /// Get the cache state for an entire view.
    pub async fn get_view_cache_state(
        &self,
        view_name: &str,
    ) -> Result<ViewCacheState, SchemaError> {
        Ok(self
            .transform_field_states_store()
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
        self.transform_field_states_store()
            .put_item(view_name, state)
            .await?;
        self.transform_field_states_store().inner().flush().await?;
        Ok(())
    }

    /// Clear cache state for a view (used when removing a view).
    pub async fn clear_view_cache_state(&self, view_name: &str) -> Result<(), SchemaError> {
        self.transform_field_states_store()
            .delete_item(view_name)
            .await?;
        self.transform_field_states_store().inner().flush().await?;
        Ok(())
    }
}
