//! Event type definitions and unified Event enum
use serde::{Deserialize, Serialize};

pub mod atom_events;
pub mod query_events;
pub mod request_events;
pub mod schema_events;

pub use atom_events::*;
pub use query_events::*;
pub use request_events::*;
pub use schema_events::*;

/// Unified event enumeration that encompasses all event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Event {
    // Core atom events
    FieldValueSet(FieldValueSet),
    AtomCreated(AtomCreated),
    AtomUpdated(AtomUpdated),
    // Molecule events
    MoleculeCreated(MoleculeCreated),
    MoleculeUpdated(MoleculeUpdated),
    // Schema-related events
    SchemaLoaded(SchemaLoaded),
    TransformExecuted(TransformExecuted),
    SchemaChanged(SchemaChanged),
    TransformTriggered(TransformTriggered),
    TransformRegistered(TransformRegistered),
    // Query/mutation events
    QueryExecuted(QueryExecuted),
    MutationExecuted(MutationExecuted),
    MutationRequest(MutationRequest),
    BackfillExpectedMutations(BackfillExpectedMutations),
    BackfillMutationFailed(BackfillMutationFailed),
    TransformRegistrationRequest(TransformRegistrationRequest),
    TransformRegistrationResponse(TransformRegistrationResponse),
    // SystemInitializationRequest removed
    // Index events
    IndexRequest(IndexRequest),
    BatchIndexRequest(BatchIndexRequest),
    SchemaApproved(SchemaApproved),
}

impl Event {
    /// Get the event type as a string identifier
    pub fn event_type(&self) -> &'static str {
        match self {
            Event::FieldValueSet(_) => "FieldValueSet",
            Event::AtomCreated(_) => "AtomCreated",
            Event::AtomUpdated(_) => "AtomUpdated",
            // Molecule events
            Event::MoleculeCreated(_) => "MoleculeCreated",
            Event::MoleculeUpdated(_) => "MoleculeUpdated",
            Event::SchemaLoaded(_) => "SchemaLoaded",
            Event::TransformExecuted(_) => "TransformExecuted",
            Event::SchemaChanged(_) => "SchemaChanged",
            Event::TransformTriggered(_) => "TransformTriggered",
            Event::TransformRegistered(_) => "TransformRegistered",
            Event::QueryExecuted(_) => "QueryExecuted",
            Event::MutationExecuted(_) => "MutationExecuted",
            Event::MutationRequest(_) => "MutationRequest",
            Event::BackfillExpectedMutations(_) => "BackfillExpectedMutations",
            Event::BackfillMutationFailed(_) => "BackfillMutationFailed",
            Event::TransformRegistrationRequest(_) => "TransformRegistrationRequest",
            Event::TransformRegistrationResponse(_) => "TransformRegistrationResponse",
            Event::SchemaApproved(_) => "SchemaApproved",
            // SystemInitializationRequest variant removed
            // Index events
            Event::IndexRequest(_) => "IndexRequest",
            Event::BatchIndexRequest(_) => "BatchIndexRequest",
        }
    }

    /// Get a list of all possible event types
    pub fn all_event_types() -> Vec<&'static str> {
        vec![
            "FieldValueSet",
            "AtomCreated",
            "AtomUpdated",
            "MoleculeCreated",
            "MoleculeUpdated",
            "SchemaLoaded",
            "TransformExecuted",
            "SchemaChanged",
            "TransformTriggered",
            "TransformRegistered",
            "QueryExecuted",
            "MutationExecuted",
            "MutationRequest",
            "BackfillExpectedMutations",
            "BackfillMutationFailed",
            "TransformRegistrationRequest",
            "TransformRegistrationResponse",
            "SchemaApproved",
            "IndexRequest",
            "BatchIndexRequest",
        ]
    }
}

