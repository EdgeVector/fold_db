use serde::{Deserialize, Serialize};

use crate::schema::types::schema::Schema;

/// State of a schema within the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SchemaState {
    /// Schema discovered from files but not yet approved by user
    #[default]
    Available,
    /// Schema approved by user, can be queried, mutated, field-mapped and transforms run
    Approved,
    /// Schema blocked by user, cannot be queried or mutated but field-mapping and transforms still run
    Blocked,
}

/// Schema definition bundled with its current state for UI/API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaWithState {
    /// All schema fields serialized at the top level
    #[serde(flatten)]
    pub schema: Schema,
    /// Current state of the schema
    pub state: SchemaState,
}

impl SchemaWithState {
    /// Create a new [`SchemaWithState`] from components
    pub fn new(schema: Schema, state: SchemaState) -> Self {
        Self { schema, state }
    }

    /// Access the schema name (helper to avoid cloning when only the name is needed)
    pub fn name(&self) -> &str {
        &self.schema.name
    }
}
