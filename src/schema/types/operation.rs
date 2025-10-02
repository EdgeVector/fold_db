use crate::schema::types::operations::MutationType;
use crate::schema::types::key_value::KeyValue;
use crate::schema::types::field::HashRangeFilter;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Represents an operation that can be performed on the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Operation {
    #[serde(rename = "query")]
    Query {
        schema: String,
        fields: Vec<String>,
        filter: Option<HashRangeFilter>,
    },
    #[serde(rename = "mutation")]
    Mutation {
        schema: String,
        fields_and_values: HashMap<String, Value>,
        key_value: KeyValue,
        mutation_type: MutationType,
    },
}
