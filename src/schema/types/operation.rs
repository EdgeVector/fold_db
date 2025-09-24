use crate::schema::types::MutationType;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Represents an operation that can be performed on the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Operation {
    #[serde(rename = "query")]
    Query {
        schema: String,
        fields: Vec<String>,
        filter: Option<Value>,
    },
    #[serde(rename = "mutation")]
    Mutation {
        schema: String,
        fields_and_values: HashMap<String, Value>,
        keys_and_values: HashMap<String, String>,
        mutation_type: MutationType,
    },
}
