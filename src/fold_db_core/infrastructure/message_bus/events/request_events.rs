use crate::schema::types::key_value::KeyValue;
use crate::schema::types::Mutation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Normalized key snapshot for field processing responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeySnapshot {
    pub hash: Option<String>,
    pub range: Option<String>,
    pub fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationRequest {
    pub correlation_id: String,
    pub mutation: Mutation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackfillExpectedMutations {
    /// The transform/schema id producing mutations (e.g., BlogPostWordIndex)
    pub transform_id: String,
    /// Unique backfill hash for this run
    pub backfill_hash: String,
    /// Total number of mutations expected to be emitted for this backfill
    pub count: u64,
}

/// Request to index a field value (for background/async indexing)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexRequest {
    pub schema_name: String,
    pub field_name: String,
    pub key_value: KeyValue,
    pub value: Value,
}

/// Batch request to index multiple field values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchIndexRequest {
    pub operations: Vec<IndexRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackfillMutationFailed {
    /// Unique backfill hash for this run
    pub backfill_hash: String,
    /// Error message
    pub error: String,
}
