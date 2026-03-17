use super::core::DbOperations;
use crate::schema::SchemaError;
use crate::view::registry::ViewState;
use crate::view::types::{TransformFieldState, TransformView};
use std::collections::HashMap;

impl DbOperations {
    /// Store a transform view definition.
    pub async fn store_view(
        &self,
        view_name: &str,
        view: &TransformView,
    ) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;

        self.views_store().put_item(view_name, view).await?;
        self.views_store().inner().flush().await?;
        Ok(())
    }

    /// Get a transform view by name.
    pub async fn get_view(
        &self,
        view_name: &str,
    ) -> Result<Option<TransformView>, SchemaError> {
        use crate::storage::traits::TypedStore;

        Ok(self.views_store().get_item(view_name).await?)
    }

    /// Get all transform views.
    pub async fn get_all_views(&self) -> Result<Vec<TransformView>, SchemaError> {
        use crate::storage::traits::TypedStore;

        let keys = self.views_store().list_keys_with_prefix("").await?;
        let mut views = Vec::new();
        for key in keys {
            if let Some(view) = self.views_store().get_item::<TransformView>(&key).await? {
                views.push(view);
            }
        }
        Ok(views)
    }

    /// Delete a transform view.
    pub async fn delete_view(&self, view_name: &str) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;

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
        use crate::storage::traits::TypedStore;

        self.view_states_store().put_item(view_name, state).await?;
        self.view_states_store().inner().flush().await?;
        Ok(())
    }

    /// Get all view states.
    pub async fn get_all_view_states(&self) -> Result<HashMap<String, ViewState>, SchemaError> {
        use crate::storage::traits::TypedStore;

        let keys = self.view_states_store().list_keys_with_prefix("").await?;
        let mut states = HashMap::new();
        for key in keys {
            if let Some(state) = self
                .view_states_store()
                .get_item::<ViewState>(&key)
                .await?
            {
                states.insert(key, state);
            }
        }
        Ok(states)
    }

    /// Delete a view state.
    pub async fn delete_view_state(&self, view_name: &str) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;

        self.view_states_store().delete_item(view_name).await?;
        self.view_states_store().inner().flush().await?;
        Ok(())
    }

    /// Get a transform field state for a specific view field.
    /// Key format: "{view_name}:{field_name}"
    pub async fn get_transform_field_state(
        &self,
        view_name: &str,
        field_name: &str,
    ) -> Result<TransformFieldState, SchemaError> {
        use crate::storage::traits::TypedStore;

        let key = format!("{}:{}", view_name, field_name);
        Ok(self
            .transform_field_states_store()
            .get_item::<TransformFieldState>(&key)
            .await?
            .unwrap_or(TransformFieldState::Empty))
    }

    /// Set a transform field state for a specific view field.
    pub async fn set_transform_field_state(
        &self,
        view_name: &str,
        field_name: &str,
        state: &TransformFieldState,
    ) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;

        let key = format!("{}:{}", view_name, field_name);
        self.transform_field_states_store()
            .put_item(&key, state)
            .await?;
        self.transform_field_states_store().inner().flush().await?;
        Ok(())
    }

    /// Clear all field states for a view (used when removing a view).
    pub async fn clear_view_field_states(&self, view_name: &str) -> Result<(), SchemaError> {
        use crate::storage::traits::TypedStore;

        let prefix = format!("{}:", view_name);
        let keys = self
            .transform_field_states_store()
            .list_keys_with_prefix(&prefix)
            .await?;
        for key in keys {
            self.transform_field_states_store()
                .delete_item(&key)
                .await?;
        }
        self.transform_field_states_store().inner().flush().await?;
        Ok(())
    }
}
