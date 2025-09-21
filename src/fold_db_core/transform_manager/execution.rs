use super::manager::TransformManager;
use crate::schema::types::SchemaError;
use serde_json::Value as JsonValue;
use std::sync::Arc;

impl TransformManager {
    /// Execute a single transform with input fetching and computation
    pub fn execute_single_transform(
        _transform_id: &str,
        transform: &crate::schema::types::Transform,
        db_ops: &Arc<crate::db_operations::DbOperations>,
        _fold_db: Option<&mut crate::fold_db_core::FoldDB>,
    ) -> Result<JsonValue, SchemaError> {
        crate::fold_db_core::transform_manager::input_fetcher::InputFetcher::execute_single_transform(_transform_id, transform, db_ops, _fold_db)
    }

    /// Execute a single transform with mutation context for incremental processing
    pub fn execute_single_transform_with_context(
        _transform_id: &str,
        transform: &crate::schema::types::Transform,
        db_ops: &Arc<crate::db_operations::DbOperations>,
        mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
        _fold_db: Option<&mut crate::fold_db_core::FoldDB>,
    ) -> Result<JsonValue, SchemaError> {
        crate::fold_db_core::transform_manager::input_fetcher::InputFetcher::execute_single_transform_with_context(_transform_id, transform, db_ops, mutation_context, _fold_db)
    }
}
