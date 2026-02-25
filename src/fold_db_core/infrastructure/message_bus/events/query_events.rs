use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::EventType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryExecuted {
    pub query_type: String,
    pub schema: String,
    pub execution_time_ms: u64,
    pub result_count: usize,
}

impl EventType for QueryExecuted {
    fn type_id() -> &'static str {
        "QueryExecuted"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationExecuted {
    pub operation: String,
    pub schema: String,
    pub execution_time_ms: u64,
    pub fields_affected: Vec<String>,
    /// Context information about the mutation for transform execution
    pub mutation_context:
        Option<crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext>,
    /// Actual data payload for indexing (list of rows, each row is a map of field->value)
    pub data: Option<Vec<std::collections::HashMap<String, serde_json::Value>>>,
    /// User ID for multi-tenant isolation
    pub user_id: Option<String>,
    /// Molecule version numbers at time of mutation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub molecule_versions: Option<std::collections::HashSet<u64>>,
    /// General-purpose metadata carried from the originating Mutation (e.g. progress_id)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl EventType for MutationExecuted {
    fn type_id() -> &'static str {
        "MutationExecuted"
    }
}

