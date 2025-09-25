use super::manager::TransformManager;
use crate::schema::types::SchemaError;
use crate::fold_db_core::transform_manager::input_fetcher::InputFetcher;
use crate::transform::executor::TransformExecutor;
use serde_json::Value as JsonValue;
use std::sync::Arc;

impl TransformManager {
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
        // Fetch input values with context
        let input_values = InputFetcher::fetch_input_values_with_context(
            transform, 
            db_ops, 
            mutation_context,
            _fold_db
        )?;
        
        // Execute the transform
        TransformExecutor::execute_transform(transform, input_values)
    }
}
