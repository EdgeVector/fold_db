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
    pub mutation_context: Option<crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext>,
}

impl EventType for MutationExecuted {
    fn type_id() -> &'static str {
        "MutationExecuted"
    }
}
