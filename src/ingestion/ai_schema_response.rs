use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parsed AI response for schema analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct AISchemaResponse {
    /// List of existing schema names that match the input data
    pub existing_schemas: Vec<String>,
    /// New schema definition if no existing schemas match
    pub new_schemas: Option<Value>,
    /// Mapping from JSON field paths to schema field paths
    pub mutation_mappers: std::collections::HashMap<String, String>,
}
