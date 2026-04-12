//! Event type definitions and unified Event enum
use serde::{Deserialize, Serialize};

pub mod atom_events;
pub mod query_events;
pub mod request_events;

pub use atom_events::*;
pub use query_events::*;
pub use request_events::*;

/// Unified event enumeration that encompasses all event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Event {
    // Core atom events
    FieldValueSet(FieldValueSet),
    AtomCreated(AtomCreated),
    // Molecule events
    MoleculeCreated(MoleculeCreated),
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
            Event::MoleculeCreated(_) => "MoleculeCreated",
            Event::QueryExecuted(_) => "QueryExecuted",
            Event::MutationExecuted(_) => "MutationExecuted",
            Event::MutationRequest(_) => "MutationRequest",
            Event::IndexRequest(_) => "IndexRequest",
            Event::BatchIndexRequest(_) => "BatchIndexRequest",
        }
    }
}
