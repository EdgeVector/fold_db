//! Thin delegators forwarding transform-view methods on `DbOperations`
//! to the underlying `ViewStore`. New code should prefer
//! `db_ops.views()` directly.

use super::core::DbOperations;
use crate::schema::SchemaError;
use crate::view::registry::ViewState;
use crate::view::types::{TransformView, ViewCacheState};
use std::collections::HashMap;

impl DbOperations {
    pub async fn store_view(
        &self,
        view_name: &str,
        view: &TransformView,
    ) -> Result<(), SchemaError> {
        self.views().store_view(view_name, view).await
    }

    pub async fn get_view(&self, view_name: &str) -> Result<Option<TransformView>, SchemaError> {
        self.views().get_view(view_name).await
    }

    pub async fn get_all_views(&self) -> Result<Vec<TransformView>, SchemaError> {
        self.views().get_all_views().await
    }

    pub async fn delete_view(&self, view_name: &str) -> Result<(), SchemaError> {
        self.views().delete_view(view_name).await
    }

    pub async fn store_view_state(
        &self,
        view_name: &str,
        state: &ViewState,
    ) -> Result<(), SchemaError> {
        self.views().store_view_state(view_name, state).await
    }

    pub async fn get_all_view_states(&self) -> Result<HashMap<String, ViewState>, SchemaError> {
        self.views().get_all_view_states().await
    }

    pub async fn delete_view_state(&self, view_name: &str) -> Result<(), SchemaError> {
        self.views().delete_view_state(view_name).await
    }

    pub async fn get_view_cache_state(
        &self,
        view_name: &str,
    ) -> Result<ViewCacheState, SchemaError> {
        self.views().get_view_cache_state(view_name).await
    }

    pub async fn set_view_cache_state(
        &self,
        view_name: &str,
        state: &ViewCacheState,
    ) -> Result<(), SchemaError> {
        self.views().set_view_cache_state(view_name, state).await
    }

    pub async fn clear_view_cache_state(&self, view_name: &str) -> Result<(), SchemaError> {
        self.views().clear_view_cache_state(view_name).await
    }
}
