use super::field_definition::{FieldDefinition, FieldDefinitionError};
use super::types::FieldValue;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Native transform specification describing inputs, output, and execution kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformSpec {
    /// Logical identifier for the transform specification.
    pub name: String,
    /// Input field definitions required by the transform.
    #[serde(default)]
    pub inputs: Vec<FieldDefinition>,
    /// Output field definition produced by the transform.
    pub output: FieldDefinition,
    /// Transform behaviour description.
    #[serde(rename = "type")]
    pub transform_type: TransformType,
}

impl TransformSpec {
    /// Construct a new transform specification.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        inputs: Vec<FieldDefinition>,
        output: FieldDefinition,
        transform_type: TransformType,
    ) -> Self {
        Self {
            name: name.into(),
            inputs,
            output,
            transform_type,
        }
    }

    /// Validate structural invariants and field references for the specification.
    pub fn validate(&self) -> Result<(), TransformSpecError> {
        if self.name.trim().is_empty() {
            return Err(TransformSpecError::EmptyName);
        }

        let mut input_names = HashSet::new();
        for input in &self.inputs {
            input
                .validate()
                .map_err(|source| TransformSpecError::InputValidation {
                    field: input.name.clone(),
                    source,
                })?;

            if !input_names.insert(input.name.clone()) {
                return Err(TransformSpecError::DuplicateInputField {
                    field: input.name.clone(),
                });
            }
        }

        self.output
            .validate()
            .map_err(|source| TransformSpecError::OutputValidation {
                field: self.output.name.clone(),
                source,
            })?;

        match &self.transform_type {
            TransformType::Map(map_transform) => {
                self.validate_map_transform(map_transform, &input_names)
            }
            TransformType::Filter(filter_transform) => {
                self.validate_filter_transform(filter_transform, &input_names)
            }
            TransformType::Reduce(reduce_transform) => {
                self.validate_reduce_transform(reduce_transform, &input_names)
            }
            TransformType::Chain(chain) => self.validate_chain(chain),
        }
    }

    fn validate_map_transform(
        &self,
        map_transform: &MapTransform,
        input_names: &HashSet<String>,
    ) -> Result<(), TransformSpecError> {
        if map_transform.field_mappings.is_empty() {
            return Err(TransformSpecError::EmptyFieldMappings);
        }

        for (output_field, mapping) in &map_transform.field_mappings {
            if output_field.trim().is_empty() {
                return Err(TransformSpecError::InvalidOutputFieldName {
                    field: output_field.clone(),
                });
            }

            match mapping {
                FieldMapping::Direct { field } => {
                    ensure_known_field(field, input_names)?;
                }
                FieldMapping::Expression { expression } => {
                    if expression.trim().is_empty() {
                        return Err(TransformSpecError::EmptyExpressionMapping {
                            field: output_field.clone(),
                        });
                    }
                }
                FieldMapping::Constant { .. } => {}
                FieldMapping::Function { name, arguments } => {
                    if name.trim().is_empty() {
                        return Err(TransformSpecError::EmptyFunctionName {
                            field: output_field.clone(),
                        });
                    }

                    for argument in arguments {
                        if !input_names.contains(argument) {
                            return Err(TransformSpecError::UnknownFunctionArgument {
                                function: name.clone(),
                                argument: argument.clone(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_filter_transform(
        &self,
        filter_transform: &FilterTransform,
        input_names: &HashSet<String>,
    ) -> Result<(), TransformSpecError> {
        Self::validate_filter_condition(&filter_transform.condition, input_names)
    }

    fn validate_filter_condition(
        condition: &FilterCondition,
        input_names: &HashSet<String>,
    ) -> Result<(), TransformSpecError> {
        match condition {
            FilterCondition::Equals { field, .. }
            | FilterCondition::NotEquals { field, .. }
            | FilterCondition::GreaterThan { field, .. }
            | FilterCondition::LessThan { field, .. }
            | FilterCondition::Contains { field, .. } => ensure_known_field(field, input_names),
            FilterCondition::And { conditions } | FilterCondition::Or { conditions } => {
                if conditions.is_empty() {
                    return Err(TransformSpecError::EmptyConditionGroup);
                }

                for nested in conditions {
                    Self::validate_filter_condition(nested, input_names)?;
                }

                Ok(())
            }
        }
    }

    fn validate_reduce_transform(
        &self,
        reduce_transform: &ReduceTransform,
        input_names: &HashSet<String>,
    ) -> Result<(), TransformSpecError> {
        for field in &reduce_transform.group_by {
            if field.trim().is_empty() {
                return Err(TransformSpecError::UnknownGroupByField {
                    field: field.clone(),
                });
            }

            ensure_known_field(field, input_names).map_err(|_| {
                TransformSpecError::UnknownGroupByField {
                    field: field.clone(),
                }
            })?;
        }

        if let Some((label, field_name)) = reduce_transform.reducer.field_requirement() {
            if field_name.trim().is_empty() {
                return Err(TransformSpecError::ReducerMissingField);
            }

            if !input_names.contains(field_name) {
                return Err(TransformSpecError::UnknownReducerField {
                    reducer: label,
                    field: field_name.clone(),
                });
            }
        }

        Ok(())
    }

    fn validate_chain(&self, chain: &[TransformSpec]) -> Result<(), TransformSpecError> {
        if chain.is_empty() {
            return Err(TransformSpecError::EmptyTransformChain);
        }

        for (index, spec) in chain.iter().enumerate() {
            if let Err(source) = spec.validate() {
                return Err(TransformSpecError::InvalidNestedSpec {
                    index,
                    source: Box::new(source),
                });
            }
        }

        Ok(())
    }
}

fn ensure_known_field(
    field: &str,
    known_fields: &HashSet<String>,
) -> Result<(), TransformSpecError> {
    if field.trim().is_empty() || !known_fields.contains(field) {
        return Err(TransformSpecError::UnknownFieldReference {
            field: field.to_string(),
        });
    }

    Ok(())
}

/// Supported transform behaviours.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransformType {
    Map(MapTransform),
    Filter(FilterTransform),
    Reduce(ReduceTransform),
    Chain(Vec<TransformSpec>),
}

/// Mapping transform metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapTransform {
    /// Map of output field names to mapping behaviour.
    #[serde(default)]
    pub field_mappings: HashMap<String, FieldMapping>,
}

impl MapTransform {
    /// Construct a new mapping transform.
    #[must_use]
    pub fn new(field_mappings: HashMap<String, FieldMapping>) -> Self {
        Self { field_mappings }
    }
}

/// Field mapping definition for map transforms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldMapping {
    Direct {
        field: String,
    },
    Expression {
        expression: String,
    },
    Constant {
        value: FieldValue,
    },
    Function {
        name: String,
        #[serde(default)]
        arguments: Vec<String>,
    },
}

/// Filter transform metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterTransform {
    pub condition: FilterCondition,
}

/// Supported filter conditions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FilterCondition {
    Equals { field: String, value: FieldValue },
    NotEquals { field: String, value: FieldValue },
    GreaterThan { field: String, value: FieldValue },
    LessThan { field: String, value: FieldValue },
    Contains { field: String, value: FieldValue },
    And { conditions: Vec<FilterCondition> },
    Or { conditions: Vec<FilterCondition> },
}

/// Reduce transform metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReduceTransform {
    pub reducer: ReducerType,
    #[serde(default)]
    pub group_by: Vec<String>,
}

impl ReduceTransform {
    /// Construct a new reducer transform.
    #[must_use]
    pub fn new(reducer: ReducerType, group_by: Vec<String>) -> Self {
        Self { reducer, group_by }
    }
}

/// Supported reducer types for aggregate transforms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReducerType {
    Sum { field: String },
    Count,
    Average { field: String },
    Min { field: String },
    Max { field: String },
    First { field: String },
    Last { field: String },
}

impl ReducerType {
    fn field_requirement(&self) -> Option<(&'static str, &String)> {
        match self {
            ReducerType::Sum { field } => Some(("sum", field)),
            ReducerType::Average { field } => Some(("average", field)),
            ReducerType::Min { field } => Some(("min", field)),
            ReducerType::Max { field } => Some(("max", field)),
            ReducerType::First { field } => Some(("first", field)),
            ReducerType::Last { field } => Some(("last", field)),
            ReducerType::Count => None,
        }
    }
}

/// Validation errors emitted by [`TransformSpec::validate`].
#[derive(Debug, Error)]
pub enum TransformSpecError {
    /// Transform name is empty or whitespace.
    #[error("transform spec name cannot be empty")]
    EmptyName,
    /// Duplicate input field definitions were provided.
    #[error("duplicate input field '{field}' in transform spec")]
    DuplicateInputField { field: String },
    /// Input field definition failed validation.
    #[error("input field '{field}' failed validation: {source}")]
    InputValidation {
        field: String,
        #[source]
        source: FieldDefinitionError,
    },
    /// Output field definition failed validation.
    #[error("output field '{field}' failed validation: {source}")]
    OutputValidation {
        field: String,
        #[source]
        source: FieldDefinitionError,
    },
    /// Map transform contained no mappings.
    #[error("map transform must declare at least one field mapping")]
    EmptyFieldMappings,
    /// Map transform output field name is invalid.
    #[error("map transform output field name '{field}' cannot be empty")]
    InvalidOutputFieldName { field: String },
    /// Mapping references an unknown input field.
    #[error("mapping references unknown input field '{field}'")]
    UnknownFieldReference { field: String },
    /// Expression mapping provided an empty expression string.
    #[error("expression mapping for field '{field}' cannot be empty")]
    EmptyExpressionMapping { field: String },
    /// Function mapping omitted the function name.
    #[error("function mapping for '{field}' must provide a function name")]
    EmptyFunctionName { field: String },
    /// Function mapping references an unknown argument.
    #[error("function '{function}' references unknown input field '{argument}'")]
    UnknownFunctionArgument { function: String, argument: String },
    /// Logical condition group contains no conditions.
    #[error("logical condition groups must contain at least one condition")]
    EmptyConditionGroup,
    /// Reducer variant requires a field name but none was provided.
    #[error("reduce transform reducer variant requires a source field name")]
    ReducerMissingField,
    /// Reducer references an unknown input field.
    #[error("reduce transform reducer '{reducer}' references unknown input field '{field}'")]
    UnknownReducerField {
        reducer: &'static str,
        field: String,
    },
    /// Group-by clause references an unknown field.
    #[error("reduce transform group-by field '{field}' is unknown")]
    UnknownGroupByField { field: String },
    /// Transform chain was empty.
    #[error("chain transform must contain at least one transform specification")]
    EmptyTransformChain,
    /// Nested transform specification failed validation.
    #[error("nested transform spec at index {index} failed validation: {source}")]
    InvalidNestedSpec {
        index: usize,
        #[source]
        source: Box<TransformSpecError>,
    },
}
