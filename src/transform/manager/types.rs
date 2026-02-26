use crate::fold_db_core::query::formatter::Record;
use crate::schema::SchemaError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;

/// Result of transform execution containing structured records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    /// The records produced by the transform
    pub records: Vec<Record>,
    /// Metadata about the execution
    pub metadata: HashMap<String, Value>,
}

impl TransformResult {
    /// Create a new TransformResult with the given records
    pub fn new(records: Vec<Record>) -> Self {
        Self {
            records,
            metadata: HashMap::new(),
        }
    }

}

use async_trait::async_trait;

/// Trait abstraction over transform execution for easier testing.
/// All execution is now event-driven through the message bus.
#[async_trait]
pub trait TransformRunner: Send + Sync {
    async fn execute_transform_with_context(
        &self,
        transform_id: &str,
        mutation_context: &Option<
            crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext,
        >,
    ) -> Result<TransformResult, SchemaError>;
    fn transform_exists(&self, transform_id: &str) -> Result<bool, SchemaError>;
    fn get_transforms_for_field(
        &self,
        schema_name: &str,
        field_name: &str,
    ) -> Result<HashSet<String>, SchemaError>;
}
