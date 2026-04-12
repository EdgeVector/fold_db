use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::request_events::KeySnapshot;
use crate::schema::types::key_value::KeyValue;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldValueSet {
    pub field: String,
    pub value: Value,
    pub source: String,
    /// Context information about the mutation that triggered this event
    pub mutation_context: Option<MutationContext>,
    /// Normalized key snapshot emitted with the event
    pub key_snapshot: Option<KeySnapshot>,
}

/// Context information about a mutation for smarter transform execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationContext {
    /// The key configuration containing hash and range field values
    pub key_value: Option<KeyValue>,
    /// The mutation hash for tracking
    pub mutation_hash: Option<String>,
    /// Whether this mutation should trigger incremental processing
    pub incremental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomCreated {
    pub atom_id: String,
    pub data: Value,
}

// Molecule events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeCreated {
    pub molecule_uuid: String,
    pub molecule_type: String,
    pub field_path: String,
}
