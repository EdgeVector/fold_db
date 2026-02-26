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

/// Envelope that wraps an event with user context for multi-tenant processing
///
/// This envelope ensures that when events are published to message buses (SNS/SQS),
/// the user_id context is preserved so consumers can process events in the correct
/// user's context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventEnvelope {
    /// The wrapped event
    pub event: Event,
    /// User ID for multi-tenant isolation
    pub user_id: Option<String>,
    /// Optional correlation ID for tracing
    pub correlation_id: Option<String>,
    /// Timestamp when envelope was created
    pub timestamp_ms: u64,
}

impl EventEnvelope {
    /// Create a new envelope with current user context
    pub fn new(event: Event) -> Self {
        let user_id = crate::logging::core::get_current_user_id();
        Self {
            event,
            user_id,
            correlation_id: None,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Create envelope with explicit user_id
    pub fn with_user(event: Event, user_id: String) -> Self {
        Self {
            event,
            user_id: Some(user_id),
            correlation_id: None,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Add a correlation ID for request tracing
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Get the event type
    pub fn event_type(&self) -> &'static str {
        self.event.event_type()
    }

    /// Serialize envelope to JSON bytes for transport
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize envelope from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Process the event with user context restored
    ///
    /// Use this when receiving events from external sources (SQS, SNS, HTTP)
    /// to ensure storage operations use the correct user_id.
    ///
    /// # Example
    /// ```ignore
    /// let envelope = EventEnvelope::from_bytes(&message_bytes)?;
    /// envelope.process_with_context(|event| async move {
    ///     // Storage operations here will use the envelope's user_id
    ///     handle_event(event).await
    /// }).await;
    /// ```
    pub async fn process_with_context<F, Fut, T>(self, f: F) -> T
    where
        F: FnOnce(Event) -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        if let Some(user_id) = self.user_id {
            crate::logging::core::run_with_user(&user_id, f(self.event)).await
        } else {
            f(self.event).await
        }
    }
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
    TransformRegistered(TransformRegistered),
    DataPersisted(DataPersisted),
    // Query/mutation events
    QueryExecuted(QueryExecuted),
    MutationExecuted(MutationExecuted),
    MutationRequest(MutationRequest),
    // Request/Response events
    FieldValueSetRequest(FieldValueSetRequest),
    FieldValueSetResponse(FieldValueSetResponse),
    FieldValueQueryRequest(FieldValueQueryRequest),
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
            Event::DataPersisted(_) => "DataPersisted",
            Event::QueryExecuted(_) => "QueryExecuted",
            Event::MutationExecuted(_) => "MutationExecuted",
            Event::MutationRequest(_) => "MutationRequest",
            // Request/Response events
            Event::FieldValueSetRequest(_) => "FieldValueSetRequest",
            Event::FieldValueSetResponse(_) => "FieldValueSetResponse",
            Event::FieldValueQueryRequest(_) => "FieldValueQueryRequest",
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
            "DataPersisted",
            "QueryExecuted",
            "MutationExecuted",
            "MutationRequest",
            "FieldValueSetRequest",
            "FieldValueSetResponse",
            "FieldValueQueryRequest",
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

