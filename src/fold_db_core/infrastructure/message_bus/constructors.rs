//! Convenience constructors for event types
//!
//! This module provides convenient constructor methods for all event types
//! to make event creation more ergonomic.

use super::events::*;
use super::request_events::KeySnapshot;
use serde_json::Value;

// ========== Core Event Constructors ==========

impl FieldValueSet {
    /// Create a new FieldValueSet event
    pub fn new(field: impl Into<String>, value: Value, source: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            value,
            source: source.into(),
            mutation_context: None,
            key_snapshot: None,
        }
    }

    /// Create a new FieldValueSet event with mutation context
    pub fn with_context(
        field: impl Into<String>,
        value: Value,
        source: impl Into<String>,
        mutation_context: atom_events::MutationContext,
    ) -> Self {
        Self {
            field: field.into(),
            value,
            source: source.into(),
            mutation_context: Some(mutation_context),
            key_snapshot: None,
        }
    }

    /// Create a new FieldValueSet event with a key snapshot
    pub fn with_keys(
        field: impl Into<String>,
        value: Value,
        source: impl Into<String>,
        key_snapshot: KeySnapshot,
    ) -> Self {
        Self {
            field: field.into(),
            value,
            source: source.into(),
            mutation_context: None,
            key_snapshot: Some(key_snapshot),
        }
    }

    /// Create a new FieldValueSet event with mutation context and key snapshot
    pub fn with_context_and_keys(
        field: impl Into<String>,
        value: Value,
        source: impl Into<String>,
        mutation_context: atom_events::MutationContext,
        key_snapshot: KeySnapshot,
    ) -> Self {
        Self {
            field: field.into(),
            value,
            source: source.into(),
            mutation_context: Some(mutation_context),
            key_snapshot: Some(key_snapshot),
        }
    }
}
impl AtomCreated {
    /// Create a new AtomCreated event
    pub fn new(atom_id: impl Into<String>, data: Value) -> Self {
        Self {
            atom_id: atom_id.into(),
            data,
        }
    }
}

impl AtomUpdated {
    /// Create a new AtomUpdated event
    pub fn new(atom_id: impl Into<String>, data: Value) -> Self {
        Self {
            atom_id: atom_id.into(),
            data,
        }
    }
}

impl MoleculeCreated {
    /// Create a new MoleculeCreated event
    pub fn new(
        molecule_uuid: impl Into<String>,
        molecule_type: impl Into<String>,
        field_path: impl Into<String>,
    ) -> Self {
        Self {
            molecule_uuid: molecule_uuid.into(),
            molecule_type: molecule_type.into(),
            field_path: field_path.into(),
        }
    }
}

impl MoleculeUpdated {
    /// Create a new MoleculeUpdated event
    pub fn new(
        molecule_uuid: impl Into<String>,
        field_path: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self {
            molecule_uuid: molecule_uuid.into(),
            field_path: field_path.into(),
            operation: operation.into(),
        }
    }
}

impl SchemaLoaded {
    /// Create a new SchemaLoaded event
    pub fn new(schema_name: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            schema_name: schema_name.into(),
            status: status.into(),
        }
    }
}

impl TransformExecuted {
    /// Create a new TransformExecuted event
    pub fn new(transform_id: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            transform_id: transform_id.into(),
            result: result.into(),
        }
    }
}

impl SchemaChanged {
    /// Create a new SchemaChanged event
    pub fn new(schema: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
        }
    }
}

impl TransformTriggered {
    /// Create a new TransformTriggered event
    pub fn new(transform_id: impl Into<String>) -> Self {
        Self {
            transform_id: transform_id.into(),
            mutation_context: None,
        }
    }

    /// Create a new TransformTriggered event with mutation context
    pub fn with_context(
        transform_id: impl Into<String>,
        mutation_context: atom_events::MutationContext,
    ) -> Self {
        Self {
            transform_id: transform_id.into(),
            mutation_context: Some(mutation_context),
        }
    }
}

impl QueryExecuted {
    /// Create a new QueryExecuted event
    pub fn new(
        query_type: impl Into<String>,
        schema: impl Into<String>,
        execution_time_ms: u64,
        result_count: usize,
    ) -> Self {
        Self {
            query_type: query_type.into(),
            schema: schema.into(),
            execution_time_ms,
            result_count,
        }
    }
}

impl MutationExecuted {
    /// Create a new MutationExecuted event
    pub fn new(
        operation: impl Into<String>,
        schema: impl Into<String>,
        execution_time_ms: u64,
        fields_affected: Vec<String>,
    ) -> Self {
        Self {
            operation: operation.into(),
            schema: schema.into(),
            execution_time_ms,
            fields_affected,
            mutation_context: None,
            data: None,
            user_id: crate::logging::core::get_current_user_id(),
            molecule_versions: None,
            metadata: None,
        }
    }

    /// Create a new MutationExecuted event with mutation context
    pub fn with_context(
        operation: impl Into<String>,
        schema: impl Into<String>,
        execution_time_ms: u64,
        fields_affected: Vec<String>,
        mutation_context: Option<atom_events::MutationContext>,
    ) -> Self {
        Self {
            operation: operation.into(),
            schema: schema.into(),
            execution_time_ms,
            fields_affected,
            mutation_context,
            data: None,
            user_id: crate::logging::core::get_current_user_id(),
            molecule_versions: None,
            metadata: None,
        }
    }
}

impl DataPersisted {
    /// Create a new DataPersisted event
    pub fn new(schema_name: impl Into<String>, correlation_id: impl Into<String>) -> Self {
        Self {
            schema_name: schema_name.into(),
            correlation_id: correlation_id.into(),
            transform_id: None,
            context: None,
        }
    }

    /// Create a new DataPersisted event with transform context
    pub fn with_transform(
        schema_name: impl Into<String>,
        correlation_id: impl Into<String>,
        transform_id: impl Into<String>,
    ) -> Self {
        Self {
            schema_name: schema_name.into(),
            correlation_id: correlation_id.into(),
            transform_id: Some(transform_id.into()),
            context: None,
        }
    }

    /// Create a new DataPersisted event with additional context
    pub fn with_context(
        schema_name: impl Into<String>,
        correlation_id: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self {
            schema_name: schema_name.into(),
            correlation_id: correlation_id.into(),
            transform_id: None,
            context: Some(context.into()),
        }
    }
}

// ========== Request/Response Event Constructors ==========

impl FieldValueSetRequest {
    /// Create a new FieldValueSetRequest
    pub fn new(
        correlation_id: String,
        schema_name: String,
        field_name: String,
        value: Value,
        source_pub_key: String,
    ) -> Self {
        Self {
            correlation_id,
            schema_name,
            field_name,
            value,
            source_pub_key,
            mutation_context: None,
        }
    }

    /// Create a new FieldValueSetRequest with mutation context
    pub fn with_context(
        correlation_id: String,
        schema_name: String,
        field_name: String,
        value: Value,
        source_pub_key: String,
        mutation_context: atom_events::MutationContext,
    ) -> Self {
        Self {
            correlation_id,
            schema_name,
            field_name,
            value,
            source_pub_key,
            mutation_context: Some(mutation_context),
        }
    }
}

impl FieldValueSetResponse {
    /// Create a new FieldValueSetResponse
    pub fn new(
        correlation_id: String,
        success: bool,
        molecule_uuid: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            molecule_uuid,
            error,
            key_snapshot: None,
        }
    }

    /// Create a new FieldValueSetResponse with key snapshot
    pub fn with_key_snapshot(
        correlation_id: String,
        success: bool,
        molecule_uuid: Option<String>,
        error: Option<String>,
        key_snapshot: Option<KeySnapshot>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            molecule_uuid,
            error,
            key_snapshot,
        }
    }
}

impl FieldUpdateRequest {
    /// Create a new FieldUpdateRequest
    pub fn new(
        correlation_id: String,
        schema_name: String,
        field_name: String,
        value: Value,
        source_pub_key: String,
    ) -> Self {
        Self {
            correlation_id,
            schema_name,
            field_name,
            value,
            source_pub_key,
        }
    }
}

impl FieldUpdateResponse {
    /// Create a new FieldUpdateResponse
    pub fn new(
        correlation_id: String,
        success: bool,
        molecule_uuid: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            molecule_uuid,
            error,
        }
    }
}

impl SchemaLoadRequest {
    /// Create a new SchemaLoadRequest
    pub fn new(correlation_id: String, schema_name: String) -> Self {
        Self {
            correlation_id,
            schema_name,
        }
    }
}

impl SchemaLoadResponse {
    /// Create a new SchemaLoadResponse
    pub fn new(
        correlation_id: String,
        success: bool,
        schema_data: Option<Value>,
        error: Option<String>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            schema_data,
            error,
        }
    }
}

impl SchemaApprovalRequest {
    /// Create a new SchemaApprovalRequest
    pub fn new(correlation_id: String, schema_name: String) -> Self {
        Self {
            correlation_id,
            schema_name,
        }
    }
}

impl SchemaApprovalResponse {
    /// Create a new SchemaApprovalResponse
    pub fn new(correlation_id: String, success: bool, error: Option<String>) -> Self {
        Self {
            correlation_id,
            success,
            error,
        }
    }
}

impl AtomHistoryRequest {
    /// Create a new AtomHistoryRequest
    pub fn new(correlation_id: String, molecule_uuid: String) -> Self {
        Self {
            correlation_id,
            molecule_uuid,
        }
    }
}

impl AtomHistoryResponse {
    /// Create a new AtomHistoryResponse
    pub fn new(
        correlation_id: String,
        success: bool,
        history: Option<Vec<Value>>,
        error: Option<String>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            history,
            error,
        }
    }
}

impl AtomGetRequest {
    /// Create a new AtomGetRequest
    pub fn new(correlation_id: String, molecule_uuid: String) -> Self {
        Self {
            correlation_id,
            molecule_uuid,
        }
    }
}

impl AtomGetResponse {
    /// Create a new AtomGetResponse
    pub fn new(
        correlation_id: String,
        success: bool,
        atom_data: Option<Value>,
        error: Option<String>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            atom_data,
            error,
        }
    }
}

impl FieldValueQueryRequest {
    /// Create a new FieldValueQueryRequest
    pub fn new(
        correlation_id: String,
        schema_name: String,
        field_name: String,
        filter: Option<Value>,
    ) -> Self {
        Self {
            correlation_id,
            schema_name,
            field_name,
            filter,
        }
    }
}

impl FieldValueQueryResponse {
    /// Create a new FieldValueQueryResponse
    pub fn new(
        correlation_id: String,
        success: bool,
        field_value: Option<Value>,
        error: Option<String>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            field_value,
            error,
        }
    }
}

impl MoleculeQueryRequest {
    /// Create a new MoleculeQueryRequest
    pub fn new(correlation_id: String, molecule_uuid: String) -> Self {
        Self {
            correlation_id,
            molecule_uuid,
        }
    }
}

impl MoleculeQueryResponse {
    /// Create a new MoleculeQueryResponse
    pub fn new(correlation_id: String, success: bool, exists: bool, error: Option<String>) -> Self {
        Self {
            correlation_id,
            success,
            exists,
            error,
        }
    }
}

impl SchemaStatusRequest {
    /// Create a new SchemaStatusRequest
    pub fn new(correlation_id: String) -> Self {
        Self { correlation_id }
    }
}

impl SchemaStatusResponse {
    /// Create a new SchemaStatusResponse
    pub fn new(
        correlation_id: String,
        success: bool,
        status_data: Option<Value>,
        error: Option<String>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            status_data,
            error,
        }
    }
}

impl SchemaDiscoveryRequest {
    /// Create a new SchemaDiscoveryRequest
    pub fn new(correlation_id: String) -> Self {
        Self { correlation_id }
    }
}

impl SchemaDiscoveryResponse {
    /// Create a new SchemaDiscoveryResponse
    pub fn new(
        correlation_id: String,
        success: bool,
        report_data: Option<Value>,
        error: Option<String>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            report_data,
            error,
        }
    }
}

impl MoleculeGetRequest {
    /// Create a new MoleculeGetRequest
    pub fn new(correlation_id: String, molecule_uuid: String) -> Self {
        Self {
            correlation_id,
            molecule_uuid,
        }
    }
}

impl MoleculeGetResponse {
    /// Create a new MoleculeGetResponse
    pub fn new(
        correlation_id: String,
        success: bool,
        molecule_data: Option<Value>,
        error: Option<String>,
    ) -> Self {
        Self {
            correlation_id,
            success,
            molecule_data,
            error,
        }
    }
}

// SystemInitializationResponse constructor removed
