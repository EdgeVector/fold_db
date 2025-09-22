use super::{FieldDefinition, FieldValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Convenience alias for a native record flowing through transforms.
pub type NativeRecord = HashMap<String, FieldValue>;

/// High-level description of a native transform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeTransformSpec {
    /// Human-friendly transform identifier.
    pub name: String,
    /// Concrete transform behavior.
    #[serde(rename = "type")]
    pub transform_type: NativeTransformType,
}

/// Currently supported native transform kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeTransformType {
    /// Produce a new record by mapping inputs to outputs.
    Map(NativeMapTransform),
}

/// Definition for a map transform where each output field references a computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeMapTransform {
    /// Output fields keyed by field name.
    pub fields: HashMap<String, NativeMapField>,
}

impl NativeMapTransform {
    /// Create an empty map transform.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a field definition and computation into the transform.
    pub fn insert_field(&mut self, name: impl Into<String>, field: NativeMapField) {
        self.fields.insert(name.into(), field);
    }
}

/// Metadata and computation strategy for a single output field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeMapField {
    /// Declared output field definition.
    pub definition: FieldDefinition,
    /// How to compute the field value.
    pub computation: NativeFieldComputation,
}

impl NativeMapField {
    /// Convenience constructor for wiring field metadata and computation together.
    #[must_use]
    pub fn new(definition: FieldDefinition, computation: NativeFieldComputation) -> Self {
        Self {
            definition,
            computation,
        }
    }
}

/// Supported computation strategies for producing an output field value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NativeFieldComputation {
    /// Directly copy a value from the input payload.
    InputField { field: String },
    /// Emit a pre-defined constant value.
    Constant { value: FieldValue },
    /// Placeholder for expression-based mappings (implemented in later tasks).
    Expression { expression: String },
    /// Placeholder for function invocation (implemented in later tasks).
    Function {
        name: String,
        #[serde(default)]
        arguments: Vec<NativeFieldComputation>,
    },
}
