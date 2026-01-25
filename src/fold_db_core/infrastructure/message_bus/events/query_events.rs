use serde::{Deserialize, Serialize};

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
    /// User ID for multi-tenant isolation (used by IndexOrchestrator to set user context)
    pub user_id: Option<String>,
}

impl EventType for MutationExecuted {
    fn type_id() -> &'static str {
        "MutationExecuted"
    }
}
