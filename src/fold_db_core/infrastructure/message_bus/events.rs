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
    SchemaChanged(SchemaChanged),
    // Query/mutation events
    QueryExecuted(QueryExecuted),
    MutationExecuted(MutationExecuted),
    MutationRequest(MutationRequest),
    // Index events
    IndexRequest(IndexRequest),
    BatchIndexRequest(BatchIndexRequest),
}

impl Event {
    /// Get the event type as a string identifier
    pub fn event_type(&self) -> &'static str {
        match self {
            Event::FieldValueSet(_) => "FieldValueSet",
            Event::AtomCreated(_) => "AtomCreated",
            Event::AtomUpdated(_) => "AtomUpdated",
            Event::MoleculeCreated(_) => "MoleculeCreated",
            Event::MoleculeUpdated(_) => "MoleculeUpdated",
            Event::SchemaLoaded(_) => "SchemaLoaded",
            Event::SchemaChanged(_) => "SchemaChanged",
            Event::QueryExecuted(_) => "QueryExecuted",
            Event::MutationExecuted(_) => "MutationExecuted",
            Event::MutationRequest(_) => "MutationRequest",
            Event::IndexRequest(_) => "IndexRequest",
            Event::BatchIndexRequest(_) => "BatchIndexRequest",
        }
    }
}
