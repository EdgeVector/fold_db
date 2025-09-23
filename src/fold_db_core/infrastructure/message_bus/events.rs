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

/// Trait for types that can be used as events in the message bus
pub trait EventType: Clone + Send + 'static {
    /// Get the unique type identifier for this event type
    fn type_id() -> &'static str;
}

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
    DataPersisted(DataPersisted),
    // Query/mutation events
    QueryExecuted(QueryExecuted),
    MutationExecuted(MutationExecuted),
    // Request/Response events
    AtomCreateRequest(AtomCreateRequest),
    AtomCreateResponse(AtomCreateResponse),
    AtomUpdateRequest(AtomUpdateRequest),
    AtomUpdateResponse(AtomUpdateResponse),
    // Molecule requests/responses
    MoleculeCreateRequest(MoleculeCreateRequest),
    MoleculeCreateResponse(MoleculeCreateResponse),
    MoleculeUpdateRequest(MoleculeUpdateRequest),
    MoleculeUpdateResponse(MoleculeUpdateResponse),
    MoleculeQueryRequest(MoleculeQueryRequest),
    MoleculeQueryResponse(MoleculeQueryResponse),
    MoleculeGetRequest(MoleculeGetRequest),
    MoleculeGetResponse(MoleculeGetResponse),
    // Other requests/responses
    FieldValueSetRequest(FieldValueSetRequest),
    FieldValueSetResponse(FieldValueSetResponse),
    FieldUpdateRequest(FieldUpdateRequest),
    FieldUpdateResponse(FieldUpdateResponse),
    SchemaLoadRequest(SchemaLoadRequest),
    SchemaLoadResponse(SchemaLoadResponse),
    SchemaApprovalRequest(SchemaApprovalRequest),
    SchemaApprovalResponse(SchemaApprovalResponse),
    AtomHistoryRequest(AtomHistoryRequest),
    AtomHistoryResponse(AtomHistoryResponse),
    AtomGetRequest(AtomGetRequest),
    AtomGetResponse(AtomGetResponse),
    FieldValueQueryRequest(FieldValueQueryRequest),
    FieldValueQueryResponse(FieldValueQueryResponse),
    SchemaStatusRequest(SchemaStatusRequest),
    SchemaStatusResponse(SchemaStatusResponse),
    SchemaDiscoveryRequest(SchemaDiscoveryRequest),
    SchemaDiscoveryResponse(SchemaDiscoveryResponse),
    SystemInitializationRequest(SystemInitializationRequest),
    SystemInitializationResponse(SystemInitializationResponse),
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
            Event::DataPersisted(_) => "DataPersisted",
            Event::QueryExecuted(_) => "QueryExecuted",
            Event::MutationExecuted(_) => "MutationExecuted",
            Event::AtomCreateRequest(_) => "AtomCreateRequest",
            Event::AtomCreateResponse(_) => "AtomCreateResponse",
            Event::AtomUpdateRequest(_) => "AtomUpdateRequest",
            Event::AtomUpdateResponse(_) => "AtomUpdateResponse",
            // Molecule requests/responses
            Event::MoleculeCreateRequest(_) => "MoleculeCreateRequest",
            Event::MoleculeCreateResponse(_) => "MoleculeCreateResponse",
            Event::MoleculeUpdateRequest(_) => "MoleculeUpdateRequest",
            Event::MoleculeUpdateResponse(_) => "MoleculeUpdateResponse",
            Event::MoleculeQueryRequest(_) => "MoleculeQueryRequest",
            Event::MoleculeQueryResponse(_) => "MoleculeQueryResponse",
            Event::MoleculeGetRequest(_) => "MoleculeGetRequest",
            Event::MoleculeGetResponse(_) => "MoleculeGetResponse",
            Event::FieldValueSetRequest(_) => "FieldValueSetRequest",
            Event::FieldValueSetResponse(_) => "FieldValueSetResponse",
            Event::FieldUpdateRequest(_) => "FieldUpdateRequest",
            Event::FieldUpdateResponse(_) => "FieldUpdateResponse",
            Event::SchemaLoadRequest(_) => "SchemaLoadRequest",
            Event::SchemaLoadResponse(_) => "SchemaLoadResponse",
            Event::SchemaApprovalRequest(_) => "SchemaApprovalRequest",
            Event::SchemaApprovalResponse(_) => "SchemaApprovalResponse",
            Event::AtomHistoryRequest(_) => "AtomHistoryRequest",
            Event::AtomHistoryResponse(_) => "AtomHistoryResponse",
            Event::AtomGetRequest(_) => "AtomGetRequest",
            Event::AtomGetResponse(_) => "AtomGetResponse",
            Event::FieldValueQueryRequest(_) => "FieldValueQueryRequest",
            Event::FieldValueQueryResponse(_) => "FieldValueQueryResponse",
            Event::SchemaStatusRequest(_) => "SchemaStatusRequest",
            Event::SchemaStatusResponse(_) => "SchemaStatusResponse",
            Event::SchemaDiscoveryRequest(_) => "SchemaDiscoveryRequest",
            Event::SchemaDiscoveryResponse(_) => "SchemaDiscoveryResponse",
            Event::SystemInitializationRequest(_) => "SystemInitializationRequest",
            Event::SystemInitializationResponse(_) => "SystemInitializationResponse",
        }
    }
}

impl EventType for Event {
    fn type_id() -> &'static str {
        "Event"
    }
}
