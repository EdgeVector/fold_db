use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parsed AI response for schema analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct AISchemaResponse {
    /// New schema definition created from the data structure
    pub new_schemas: Option<Value>,
    /// Mapping from JSON field paths to schema field paths
    pub mutation_mappers: std::collections::HashMap<String, String>,
}
