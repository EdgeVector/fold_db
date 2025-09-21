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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformRegistrationRequest {
    pub registration: crate::schema::types::TransformRegistration,
    pub correlation_id: String,
}

impl EventType for TransformRegistrationRequest {
    fn type_id() -> &'static str {
        "TransformRegistrationRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
