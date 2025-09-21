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
    pub fields_affected: usize,
}

impl EventType for MutationExecuted {
    fn type_id() -> &'static str {
        "MutationExecuted"
    }
}
