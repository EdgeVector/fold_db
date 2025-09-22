use crate::transform::native::{
    FieldDefinition, FieldDefinitionError, FieldType, FieldValue, NativeFieldComputation,
    NativeMapField, NativeMapTransform, NativeRecord, NativeTransformSpec, NativeTransformType,
};
use std::collections::HashMap;
use thiserror::Error;

/// Native execution errors emitted by [`NativeTransformExecutor`].
#[derive(Debug, Error, PartialEq)]
pub enum NativeTransformError {
    /// Underlying field definition does not satisfy validation rules.
    #[error("field definition for '{field}' is invalid: {source}")]
    InvalidFieldDefinition {
        field: String,
        #[source]
        source: FieldDefinitionError,
    },
    /// Required input field is missing from the provided record.
    #[error("required input field '{input_field}' missing while computing '{output_field}'")]
    MissingInput {
        input_field: String,
        output_field: String,
    },
    /// Produced value does not match the declared [`FieldType`].
    #[error(
        "output field '{field}' produced value with mismatched type (expected {expected:?}, got {actual:?})"
    )]
    TypeMismatch {
        field: String,
        expected: Box<FieldType>,
        actual: Box<FieldType>,
    },
    /// Encountered a computation variant that is not yet supported.
    #[error("computation for field '{field}' is not supported: {reason}")]
    UnsupportedComputation { field: String, reason: String },
}

/// Executes native transform specifications against [`FieldValue`] inputs.
#[derive(Debug, Default)]
pub struct NativeTransformExecutor;

impl NativeTransformExecutor {
    /// Construct a new executor instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Execute the provided transform specification against the supplied native record.
    pub fn execute(
        &self,
        transform: &NativeTransformSpec,
        input: &NativeRecord,
    ) -> Result<NativeRecord, NativeTransformError> {
        match &transform.transform_type {
            NativeTransformType::Map(map_transform) => {
                self.execute_map_transform(map_transform, input)
            }
        }
    }

    fn execute_map_transform(
        &self,
        transform: &NativeMapTransform,
        input: &NativeRecord,
    ) -> Result<NativeRecord, NativeTransformError> {
        let mut output: HashMap<String, FieldValue> =
            HashMap::with_capacity(transform.fields.len());

        for (field_name, map_field) in &transform.fields {
            let value = self.compute_map_field(field_name, map_field, input)?;
            output.insert(field_name.clone(), value);
        }

        Ok(output)
    }

    fn compute_map_field(
        &self,
        field_name: &str,
        map_field: &NativeMapField,
        input: &NativeRecord,
    ) -> Result<FieldValue, NativeTransformError> {
        map_field.definition.validate().map_err(|source| {
            NativeTransformError::InvalidFieldDefinition {
                field: field_name.to_string(),
                source,
            }
        })?;

        let value = match &map_field.computation {
            NativeFieldComputation::InputField { field } => {
                self.read_input(field_name, field, &map_field.definition, input)?
            }
            NativeFieldComputation::Constant { value } => value.clone(),
            NativeFieldComputation::Expression { expression } => {
                return Err(NativeTransformError::UnsupportedComputation {
                    field: field_name.to_string(),
                    reason: format!("expression '{expression}' evaluation is not implemented"),
                })
            }
            NativeFieldComputation::Function { name, .. } => {
                return Err(NativeTransformError::UnsupportedComputation {
                    field: field_name.to_string(),
                    reason: format!("function '{name}' execution requires function registry"),
                })
            }
        };

        self.ensure_type_matches(field_name, &map_field.definition, &value)?;

        Ok(value)
    }

    fn read_input(
        &self,
        output_field: &str,
        input_field: &str,
        definition: &FieldDefinition,
        input: &NativeRecord,
    ) -> Result<FieldValue, NativeTransformError> {
        if let Some(value) = input.get(input_field) {
            return Ok(value.clone());
        }

        if let Some(default_value) = definition.effective_default() {
            return Ok(default_value);
        }

        Err(NativeTransformError::MissingInput {
            input_field: input_field.to_string(),
            output_field: output_field.to_string(),
        })
    }

    fn ensure_type_matches(
        &self,
        field_name: &str,
        definition: &FieldDefinition,
        value: &FieldValue,
    ) -> Result<(), NativeTransformError> {
        if definition.field_type.matches(value) {
            return Ok(());
        }

        Err(NativeTransformError::TypeMismatch {
            field: field_name.to_string(),
            expected: Box::new(definition.field_type.clone()),
            actual: Box::new(value.field_type()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::native::{FieldType, FieldValue};

    fn string_field(name: &str, required: bool) -> FieldDefinition {
        FieldDefinition::new(name.to_string(), FieldType::String).with_required(required)
    }

    #[test]
    fn map_transform_copies_input_fields() {
        let mut map = NativeMapTransform::new();
        map.insert_field(
            "full_name",
            NativeMapField::new(
                string_field("full_name", true),
                NativeFieldComputation::InputField {
                    field: "full_name".to_string(),
                },
            ),
        );

        let spec = NativeTransformSpec {
            name: "copy".to_string(),
            transform_type: NativeTransformType::Map(map),
        };

        let mut input = NativeRecord::new();
        input.insert(
            "full_name".to_string(),
            FieldValue::String("Ada Lovelace".to_string()),
        );

        let executor = NativeTransformExecutor::new();
        let result = executor.execute(&spec, &input).unwrap();

        assert_eq!(
            result.get("full_name"),
            Some(&FieldValue::String("Ada Lovelace".to_string()))
        );
    }

    #[test]
    fn map_transform_uses_defaults_for_optional_fields() {
        let optional_field =
            FieldDefinition::new("country", FieldType::String).with_required(false);
        let mut map = NativeMapTransform::new();
        map.insert_field(
            "country",
            NativeMapField::new(
                optional_field,
                NativeFieldComputation::InputField {
                    field: "country".to_string(),
                },
            ),
        );

        let spec = NativeTransformSpec {
            name: "optional".to_string(),
            transform_type: NativeTransformType::Map(map),
        };

        let executor = NativeTransformExecutor::new();
        let input = NativeRecord::new();
        let result = executor.execute(&spec, &input).unwrap();

        assert_eq!(
            result.get("country"),
            Some(&FieldValue::String(String::new()))
        );
    }

    #[test]
    fn map_transform_emits_constants() {
        let mut map = NativeMapTransform::new();
        map.insert_field(
            "status",
            NativeMapField::new(
                string_field("status", true),
                NativeFieldComputation::Constant {
                    value: FieldValue::String("active".to_string()),
                },
            ),
        );

        let spec = NativeTransformSpec {
            name: "constant".to_string(),
            transform_type: NativeTransformType::Map(map),
        };

        let executor = NativeTransformExecutor::new();
        let input = NativeRecord::new();
        let result = executor.execute(&spec, &input).unwrap();

        assert_eq!(
            result.get("status"),
            Some(&FieldValue::String("active".to_string()))
        );
    }

    #[test]
    fn missing_required_inputs_return_error() {
        let mut map = NativeMapTransform::new();
        map.insert_field(
            "email",
            NativeMapField::new(
                string_field("email", true),
                NativeFieldComputation::InputField {
                    field: "email".to_string(),
                },
            ),
        );

        let spec = NativeTransformSpec {
            name: "missing".to_string(),
            transform_type: NativeTransformType::Map(map),
        };

        let executor = NativeTransformExecutor::new();
        let input = NativeRecord::new();
        let err = executor.execute(&spec, &input).unwrap_err();

        assert!(matches!(
            err,
            NativeTransformError::MissingInput {
                input_field,
                output_field,
            } if input_field == "email" && output_field == "email"
        ));
    }

    #[test]
    fn type_mismatch_returns_error() {
        let mut map = NativeMapTransform::new();
        map.insert_field(
            "age",
            NativeMapField::new(
                FieldDefinition::new("age", FieldType::Integer),
                NativeFieldComputation::Constant {
                    value: FieldValue::String("thirty".to_string()),
                },
            ),
        );

        let spec = NativeTransformSpec {
            name: "type-check".to_string(),
            transform_type: NativeTransformType::Map(map),
        };

        let executor = NativeTransformExecutor::new();
        let input = NativeRecord::new();
        let err = executor.execute(&spec, &input).unwrap_err();

        assert!(matches!(err, NativeTransformError::TypeMismatch { field, .. } if field == "age"));
    }

    #[test]
    fn unsupported_expression_mapping_returns_error() {
        let mut map = NativeMapTransform::new();
        map.insert_field(
            "full_name",
            NativeMapField::new(
                string_field("full_name", true),
                NativeFieldComputation::Expression {
                    expression: "${first} ${last}".to_string(),
                },
            ),
        );

        let spec = NativeTransformSpec {
            name: "expression".to_string(),
            transform_type: NativeTransformType::Map(map),
        };

        let executor = NativeTransformExecutor::new();
        let input = NativeRecord::new();
        let err = executor.execute(&spec, &input).unwrap_err();

        assert!(
            matches!(err, NativeTransformError::UnsupportedComputation { field, .. } if field == "full_name")
        );
    }
}
