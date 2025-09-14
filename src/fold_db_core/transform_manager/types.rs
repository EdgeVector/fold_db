use crate::schema::SchemaError;
use serde_json::Value as JsonValue;
use std::collections::HashSet;

/// Trait abstraction over transform execution for easier testing.
/// All execution is now event-driven through the message bus.
pub trait TransformRunner: Send + Sync {
    fn execute_transform_now(&self, transform_id: &str) -> Result<JsonValue, SchemaError>;
    fn execute_transform_with_context(
        &self, 
        transform_id: &str, 
        mutation_context: &Option<crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext>
    ) -> Result<JsonValue, SchemaError>;
    fn transform_exists(&self, transform_id: &str) -> Result<bool, SchemaError>;
    fn get_transforms_for_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<HashSet<String>, SchemaError>;
}
