use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::EventType;
use crate::field_value::{FieldType as ValueFieldType, TypedFieldValue};
use crate::fold_db_core::infrastructure::message_bus::atom_events::MutationContext;
use crate::schema::types::SchemaError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomCreateRequest {
    pub correlation_id: String,
    pub schema_name: String,
    pub source_pub_key: String,
    pub prev_atom_uuid: Option<String>,
    pub content: Value,
    pub status: Option<String>,
}

impl EventType for AtomCreateRequest {
    fn type_id() -> &'static str {
        "AtomCreateRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomCreateResponse {
    pub correlation_id: String,
    pub success: bool,
    pub atom_uuid: Option<String>,
    pub error: Option<String>,
    pub atom_data: Option<Value>,
}

impl EventType for AtomCreateResponse {
    fn type_id() -> &'static str {
        "AtomCreateResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomUpdateRequest {
    pub correlation_id: String,
    pub atom_uuid: String,
    pub content: Value,
    pub source_pub_key: String,
}

impl EventType for AtomUpdateRequest {
    fn type_id() -> &'static str {
        "AtomUpdateRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomUpdateResponse {
    pub correlation_id: String,
    pub success: bool,
    pub error: Option<String>,
}

impl EventType for AtomUpdateResponse {
    fn type_id() -> &'static str {
        "AtomUpdateResponse"
    }
}

// Molecule request/response types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeCreateRequest {
    pub correlation_id: String,
    pub molecule_uuid: String,
    pub atom_uuid: String,
    pub source_pub_key: String,
    pub molecule_type: String,
}

impl EventType for MoleculeCreateRequest {
    fn type_id() -> &'static str {
        "MoleculeCreateRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeCreateResponse {
    pub correlation_id: String,
    pub success: bool,
    pub error: Option<String>,
}

impl EventType for MoleculeCreateResponse {
    fn type_id() -> &'static str {
        "MoleculeCreateResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeUpdateRequest {
    pub correlation_id: String,
    pub molecule_uuid: String,
    pub atom_uuid: String,
    pub source_pub_key: String,
    pub molecule_type: String,
    pub additional_data: Option<Value>,
}

impl EventType for MoleculeUpdateRequest {
    fn type_id() -> &'static str {
        "MoleculeUpdateRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeUpdateResponse {
    pub correlation_id: String,
    pub success: bool,
    pub error: Option<String>,
}

impl EventType for MoleculeUpdateResponse {
    fn type_id() -> &'static str {
        "MoleculeUpdateResponse"
    }
}

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

impl FieldValueSetRequest {
    /// Returns the field value wrapped in a [`TypedFieldValue`] without validation.
    pub fn typed_value(&self) -> TypedFieldValue {
        TypedFieldValue::from(&self.value)
    }

    /// Returns the typed value after validating it against the expected [`FieldType`].
    pub fn typed_value_with_validation(
        &self,
        expected: ValueFieldType,
    ) -> Result<TypedFieldValue, SchemaError> {
        let typed = self.typed_value();
        typed.ensure_type_with_context(expected, Some(self.field_name.as_str()))?;
        Ok(typed)
    }

    /// Constructs a [`FieldValueSetRequest`] directly from a [`TypedFieldValue`].
    pub fn from_typed_value(
        correlation_id: String,
        schema_name: String,
        field_name: String,
        typed_value: TypedFieldValue,
        source_pub_key: String,
        mutation_context: Option<MutationContext>,
    ) -> Self {
        Self {
            correlation_id,
            schema_name,
            field_name,
            value: typed_value.into_inner(),
            source_pub_key,
            mutation_context,
        }
    }
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
pub struct FieldUpdateRequest {
    pub correlation_id: String,
    pub schema_name: String,
    pub field_name: String,
    pub value: Value,
    pub source_pub_key: String,
}

impl FieldUpdateRequest {
    /// Returns the field value wrapped in a [`TypedFieldValue`] without validation.
    pub fn typed_value(&self) -> TypedFieldValue {
        TypedFieldValue::from(&self.value)
    }

    /// Returns the typed value after validating it against the expected [`FieldType`].
    pub fn typed_value_with_validation(
        &self,
        expected: ValueFieldType,
    ) -> Result<TypedFieldValue, SchemaError> {
        let typed = self.typed_value();
        typed.ensure_type_with_context(expected, Some(self.field_name.as_str()))?;
        Ok(typed)
    }

    /// Constructs a [`FieldUpdateRequest`] directly from a [`TypedFieldValue`].
    pub fn from_typed_value(
        correlation_id: String,
        schema_name: String,
        field_name: String,
        typed_value: TypedFieldValue,
        source_pub_key: String,
    ) -> Self {
        Self {
            correlation_id,
            schema_name,
            field_name,
            value: typed_value.into_inner(),
            source_pub_key,
        }
    }
}

impl EventType for FieldUpdateRequest {
    fn type_id() -> &'static str {
        "FieldUpdateRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldUpdateResponse {
    pub correlation_id: String,
    pub success: bool,
    pub molecule_uuid: Option<String>,
    pub error: Option<String>,
}

impl EventType for FieldUpdateResponse {
    fn type_id() -> &'static str {
        "FieldUpdateResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaLoadRequest {
    pub correlation_id: String,
    pub schema_name: String,
}

impl EventType for SchemaLoadRequest {
    fn type_id() -> &'static str {
        "SchemaLoadRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaLoadResponse {
    pub correlation_id: String,
    pub success: bool,
    pub schema_data: Option<Value>,
    pub error: Option<String>,
}

impl EventType for SchemaLoadResponse {
    fn type_id() -> &'static str {
        "SchemaLoadResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaApprovalRequest {
    pub correlation_id: String,
    pub schema_name: String,
}

impl EventType for SchemaApprovalRequest {
    fn type_id() -> &'static str {
        "SchemaApprovalRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaApprovalResponse {
    pub correlation_id: String,
    pub success: bool,
    pub error: Option<String>,
}

impl EventType for SchemaApprovalResponse {
    fn type_id() -> &'static str {
        "SchemaApprovalResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomHistoryRequest {
    pub correlation_id: String,
    pub molecule_uuid: String,
}

impl EventType for AtomHistoryRequest {
    fn type_id() -> &'static str {
        "AtomHistoryRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomHistoryResponse {
    pub correlation_id: String,
    pub success: bool,
    pub history: Option<Vec<Value>>,
    pub error: Option<String>,
}

impl EventType for AtomHistoryResponse {
    fn type_id() -> &'static str {
        "AtomHistoryResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomGetRequest {
    pub correlation_id: String,
    pub molecule_uuid: String,
}

impl EventType for AtomGetRequest {
    fn type_id() -> &'static str {
        "AtomGetRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtomGetResponse {
    pub correlation_id: String,
    pub success: bool,
    pub atom_data: Option<Value>,
    pub error: Option<String>,
}

impl EventType for AtomGetResponse {
    fn type_id() -> &'static str {
        "AtomGetResponse"
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
pub struct FieldValueQueryResponse {
    pub correlation_id: String,
    pub success: bool,
    pub field_value: Option<Value>,
    pub error: Option<String>,
}

impl EventType for FieldValueQueryResponse {
    fn type_id() -> &'static str {
        "FieldValueQueryResponse"
    }
}

// Molecule query types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeQueryRequest {
    pub correlation_id: String,
    pub molecule_uuid: String,
}

impl EventType for MoleculeQueryRequest {
    fn type_id() -> &'static str {
        "MoleculeQueryRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeQueryResponse {
    pub correlation_id: String,
    pub success: bool,
    pub exists: bool,
    pub error: Option<String>,
}

impl EventType for MoleculeQueryResponse {
    fn type_id() -> &'static str {
        "MoleculeQueryResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaStatusRequest {
    pub correlation_id: String,
}

impl EventType for SchemaStatusRequest {
    fn type_id() -> &'static str {
        "SchemaStatusRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaStatusResponse {
    pub correlation_id: String,
    pub success: bool,
    pub status_data: Option<Value>,
    pub error: Option<String>,
}

impl EventType for SchemaStatusResponse {
    fn type_id() -> &'static str {
        "SchemaStatusResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaDiscoveryRequest {
    pub correlation_id: String,
}

impl EventType for SchemaDiscoveryRequest {
    fn type_id() -> &'static str {
        "SchemaDiscoveryRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaDiscoveryResponse {
    pub correlation_id: String,
    pub success: bool,
    pub report_data: Option<Value>,
    pub error: Option<String>,
}

impl EventType for SchemaDiscoveryResponse {
    fn type_id() -> &'static str {
        "SchemaDiscoveryResponse"
    }
}

// Molecule get types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeGetRequest {
    pub correlation_id: String,
    pub molecule_uuid: String,
}

impl EventType for MoleculeGetRequest {
    fn type_id() -> &'static str {
        "MoleculeGetRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoleculeGetResponse {
    pub correlation_id: String,
    pub success: bool,
    pub molecule_data: Option<Value>,
    pub error: Option<String>,
}

impl EventType for MoleculeGetResponse {
    fn type_id() -> &'static str {
        "MoleculeGetResponse"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemInitializationRequest {
    pub correlation_id: String,
    pub db_path: String,
    pub orchestrator_config: Option<Value>,
}

impl EventType for SystemInitializationRequest {
    fn type_id() -> &'static str {
        "SystemInitializationRequest"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemInitializationResponse {
    pub correlation_id: String,
    pub success: bool,
    pub error: Option<String>,
}

impl EventType for SystemInitializationResponse {
    fn type_id() -> &'static str {
        "SystemInitializationResponse"
    }
}

#[cfg(test)]
mod typed_value_request_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn field_value_set_request_enforces_expected_type() {
        let request = FieldValueSetRequest {
            correlation_id: "corr".into(),
            schema_name: "schema".into(),
            field_name: "schema.field".into(),
            value: json!("value"),
            source_pub_key: "pubkey".into(),
            mutation_context: None,
        };

        let typed = request
            .typed_value_with_validation(ValueFieldType::String)
            .expect("string value should be accepted");
        assert_eq!(typed.as_string().unwrap(), "value");

        let error = request
            .typed_value_with_validation(ValueFieldType::Integer)
            .expect_err("integer validation should fail");
        match error {
            SchemaError::InvalidData(message) => {
                assert!(message.contains("schema.field"));
                assert!(message.contains("integer"));
            }
            other => panic!("unexpected error variant: {:?}", other),
        }
    }

    #[test]
    fn field_value_set_request_from_typed_value_preserves_payload() {
        let typed = TypedFieldValue::from("payload");
        let request = FieldValueSetRequest::from_typed_value(
            "corr".into(),
            "schema".into(),
            "schema.field".into(),
            typed,
            "pub".into(),
            None,
        );
        assert_eq!(request.value, json!("payload"));
    }

    #[test]
    fn field_update_request_enforces_expected_type() {
        let request = FieldUpdateRequest {
            correlation_id: "corr".into(),
            schema_name: "schema".into(),
            field_name: "schema.field".into(),
            value: json!(42),
            source_pub_key: "pubkey".into(),
        };

        request
            .typed_value_with_validation(ValueFieldType::Integer)
            .expect("integer value should be accepted");

        let error = request
            .typed_value_with_validation(ValueFieldType::String)
            .expect_err("string validation should fail");
        match error {
            SchemaError::InvalidData(message) => assert!(message.contains("string")),
            other => panic!("unexpected error variant: {:?}", other),
        }
    }

    #[test]
    fn field_update_request_from_typed_value_preserves_payload() {
        let typed = TypedFieldValue::from(99_i64);
        let request = FieldUpdateRequest::from_typed_value(
            "corr".into(),
            "schema".into(),
            "schema.field".into(),
            typed,
            "pub".into(),
        );
        assert_eq!(request.value, json!(99));
    }
}
