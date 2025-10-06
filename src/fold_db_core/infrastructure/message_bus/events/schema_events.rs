use serde::{Deserialize, Serialize};

use super::EventType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaLoaded {
    pub schema_name: String,
    pub status: String,
}

impl EventType for SchemaLoaded {
    fn type_id() -> &'static str {
        "SchemaLoaded"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformExecuted {
    pub transform_id: String,
    pub result: String,
}

impl EventType for TransformExecuted {
    fn type_id() -> &'static str {
        "TransformExecuted"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaChanged {
    pub schema: String,
}

impl EventType for SchemaChanged {
    fn type_id() -> &'static str {
        "SchemaChanged"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformTriggered {
    pub transform_id: String,
    /// Context information about the mutation that triggered this transform
    pub mutation_context:
        Option<crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext>,
}

impl EventType for TransformTriggered {
    fn type_id() -> &'static str {
        "TransformTriggered"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformRegistrationRequest {
    pub registration: crate::schema::types::TransformRegistration,
    pub correlation_id: String,
}

impl EventType for TransformRegistrationRequest {
    fn type_id() -> &'static str {
        "TransformRegistrationRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformRegistrationResponse {
    pub correlation_id: String,
    pub success: bool,
    pub error: Option<String>,
}

impl EventType for TransformRegistrationResponse {
    fn type_id() -> &'static str {
        "TransformRegistrationResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataPersisted {
    /// The schema name where data was persisted
    pub schema_name: String,
    /// The transform ID that generated the data (if applicable)
    pub transform_id: Option<String>,
    /// The correlation ID of the mutation that persisted the data
    pub correlation_id: String,
    /// Additional context about what was persisted
    pub context: Option<String>,
}

impl EventType for DataPersisted {
    fn type_id() -> &'static str {
        "DataPersisted"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformRegistered {
    /// The transform ID that was registered
    pub transform_id: String,
    /// The source schema name for backfill
    pub source_schema_name: String,
    /// The correlation ID from the registration request
    pub correlation_id: String,
}

impl EventType for TransformRegistered {
    fn type_id() -> &'static str {
        "TransformRegistered"
    }
}
