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

impl TransformExecuted {
    /// Create a new TransformExecuted event
    pub fn new(transform_id: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            transform_id: transform_id.into(),
            result: result.into(),
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

