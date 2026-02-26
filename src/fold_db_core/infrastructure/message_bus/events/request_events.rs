use crate::schema::types::key_value::KeyValue;
use crate::schema::types::Mutation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::EventType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldValueSetRequest {
    pub correlation_id: String,
    pub schema_name: String,
    pub field_name: String,
    pub value: Value,
    pub source_pub_key: String,
    /// Context information about the mutation that triggered this request
    pub mutation_context:
        Option<crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext>,
}

impl EventType for FieldValueSetRequest {
    fn type_id() -> &'static str {
        "FieldValueSetRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldValueSetResponse {
    pub correlation_id: String,
    pub success: bool,
    pub molecule_uuid: Option<String>,
    pub error: Option<String>,
    /// Normalized key snapshot with hash, range, and fields data
    pub key_snapshot: Option<KeySnapshot>,
}

/// Normalized key snapshot for field processing responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeySnapshot {
    pub hash: Option<String>,
    pub range: Option<String>,
    pub fields: serde_json::Map<String, serde_json::Value>,
}

impl EventType for FieldValueSetResponse {
    fn type_id() -> &'static str {
        "FieldValueSetResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldValueQueryRequest {
    pub correlation_id: String,
    pub schema_name: String,
    pub field_name: String,
    pub filter: Option<Value>,
}

impl EventType for FieldValueQueryRequest {
    fn type_id() -> &'static str {
        "FieldValueQueryRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationRequest {
    pub correlation_id: String,
    pub mutation: Mutation,
}

impl EventType for MutationRequest {
    fn type_id() -> &'static str {
        "MutationRequest"
    }
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

impl EventType for BackfillExpectedMutations {
    fn type_id() -> &'static str {
        "BackfillExpectedMutations"
    }
}

/// Request to index a field value (for background/async indexing)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexRequest {
    pub schema_name: String,
    pub field_name: String,
    pub key_value: KeyValue,
    pub value: Value,
}

impl EventType for IndexRequest {
    fn type_id() -> &'static str {
        "IndexRequest"
    }
}

/// Batch request to index multiple field values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchIndexRequest {
    pub operations: Vec<IndexRequest>,
}

impl EventType for BatchIndexRequest {
    fn type_id() -> &'static str {
        "BatchIndexRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackfillMutationFailed {
    /// Unique backfill hash for this run
    pub backfill_hash: String,
    /// Error message
    pub error: String,
}

impl EventType for BackfillMutationFailed {
    fn type_id() -> &'static str {
        "BackfillMutationFailed"
    }
}
