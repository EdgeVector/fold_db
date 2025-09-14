use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::EventType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldValueSet {
    pub field: String,
    pub value: Value,
    pub source: String,
    /// Context information about the mutation that triggered this event
    pub mutation_context: Option<MutationContext>,
}

/// Context information about a mutation for smarter transform execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationContext {
    /// The range key value that was mutated (for range and hashrange schemas)
    pub range_key: Option<String>,
    /// The hash key value that was mutated (for hashrange schemas)
    pub hash_key: Option<String>,
    /// The mutation hash for tracking
    pub mutation_hash: Option<String>,
    /// Whether this mutation should trigger incremental processing
    pub incremental: bool,
}


impl EventType for FieldValueSet {
    fn type_id() -> &'static str {
        "FieldValueSet"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomCreated {
    pub atom_id: String,
    pub data: Value,
}

impl EventType for AtomCreated {
    fn type_id() -> &'static str {
        "AtomCreated"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomUpdated {
    pub atom_id: String,
    pub data: Value,
}

impl EventType for AtomUpdated {
    fn type_id() -> &'static str {
        "AtomUpdated"
    }
}

// Molecule events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeCreated {
    pub molecule_uuid: String,
    pub molecule_type: String,
    pub field_path: String,
}

impl EventType for MoleculeCreated {
    fn type_id() -> &'static str {
        "MoleculeCreated"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeUpdated {
    pub molecule_uuid: String,
    pub field_path: String,
    pub operation: String,
}

impl EventType for MoleculeUpdated {
    fn type_id() -> &'static str {
        "MoleculeUpdated"
    }
}

