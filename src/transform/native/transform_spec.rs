use super::field_definition::{self, FieldDefinition, FieldDefinitionError};
use super::types::{FieldType, FieldValue};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Fully native specification describing how a transform operates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformSpec {
    /// Logical name of the transform.
    pub name: String,
    /// Declared input fields required by the transform.
    #[serde(default)]
    pub inputs: Vec<FieldDefinition>,
    /// Description of the produced value.
    pub output: FieldDefinition,
    /// Concrete behaviour implemented by the transform.
    pub transform_type: TransformType,
}

/// High-level behaviour available for native transforms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransformType {
    Map(MapTransform),
    Filter(FilterTransform),
    Reduce(ReduceTransform),
    Chain(Vec<TransformSpec>),
}

/// Declarative configuration for map transforms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapTransform {
    pub field_mappings: HashMap<String, FieldMapping>,
}

/// Mapping options available to map transforms when producing output fields.
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

/// Declarative configuration for filter transforms.
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

/// Declarative configuration for reduce transforms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReduceTransform {
    pub reducer: Reducer,
    #[serde(default)]
    pub group_by: Vec<String>,
}

/// Supported reducer behaviours.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Reducer {
    Sum { field: String },
    Count,
    Average { field: String },
    Min { field: String },
    Max { field: String },
    First { field: String },
    Last { field: String },
}

/// Validation failures for transform specifications.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransformSpecError {
    #[error("transform name cannot be empty")]
    EmptyName,
    #[error("transform name '{name}' exceeds maximum length of {max} characters")]
    NameTooLong { name: String, max: usize },
    #[error("transform name '{name}' must start with an ASCII letter or underscore")]
    InvalidNameStart { name: String },
    #[error(
        "transform name '{name}' contains invalid characters; only ASCII letters, digits, and underscores are allowed"
    )]
    InvalidNameCharacters { name: String },
    #[error("duplicate input field '{name}' in transform specification")]
    DuplicateInput { name: String },
    #[error("input field '{name}' failed validation: {source}")]
    InvalidInputDefinition {
        name: String,
        source: FieldDefinitionError,
    },
    #[error("output field definition failed validation: {0}")]
    InvalidOutputDefinition(FieldDefinitionError),
    #[error("map transform must define at least one field mapping")]
    EmptyMapMappings,
    #[error("map transform output must be an object but found {actual:?}")]
    MapOutputNotObject { actual: FieldType },
    #[error("map transform defines mapping for unknown output field '{field}'")]
    UnknownMapOutputField { field: String },
    #[error("map transform mapping for field '{target}' references unknown input '{referenced}'")]
    UnknownInputReference { target: String, referenced: String },
    #[error("map transform mapping for field '{target}' requires expression content")]
    EmptyExpression { target: String },
    #[error(
        "map transform constant for field '{target}' does not match expected type (expected {expected:?}, got {actual:?})"
    )]
    ConstantTypeMismatch {
        target: String,
        expected: Box<FieldType>,
        actual: Box<FieldType>,
    },
    #[error("map transform function for field '{target}' must provide a name")]
    EmptyFunctionName { target: String },
    #[error("map transform function '{function}' references unknown argument '{argument}'")]
    UnknownFunctionArgument { function: String, argument: String },
    #[error("filter transform references unknown field '{field}'")]
    UnknownFilterField { field: String },
    #[error("filter transform '{operator}' group must contain at least one condition")]
    EmptyConditionGroup { operator: &'static str },
    #[error("reduce transform references unknown field '{field}' in reducer")]
    UnknownReducerField { field: String },
    #[error("reduce transform references unknown group-by field '{field}'")]
    UnknownGroupByField { field: String },
    #[error("reduce transform group-by field '{field}' is duplicated")]
    DuplicateGroupByField { field: String },
    #[error("transform chain must contain at least one nested spec")]
    EmptyChain,
    #[error("nested transform '{name}' is invalid: {source}")]
    NestedSpecInvalid {
        name: String,
        source: Box<TransformSpecError>,
    },
}

impl TransformSpec {
    /// Validate the specification to ensure it adheres to naming and structural rules.
    pub fn validate(&self) -> Result<(), TransformSpecError> {
        field_definition::validate_identifier(&self.name).map_err(map_identifier_error)?;

        let mut input_names: HashSet<&str> = HashSet::new();
        for input in &self.inputs {
            input
                .validate()
                .map_err(|source| TransformSpecError::InvalidInputDefinition {
                    name: input.name.clone(),
                    source,
                })?;

            if !input_names.insert(input.name.as_str()) {
                return Err(TransformSpecError::DuplicateInput {
                    name: input.name.clone(),
                });
            }
        }

        self.output
            .validate()
            .map_err(TransformSpecError::InvalidOutputDefinition)?;

        match &self.transform_type {
            TransformType::Map(map_transform) => {
                map_transform.validate(&input_names, &self.output)?
            }
            TransformType::Filter(filter_transform) => filter_transform.validate(&input_names)?,
            TransformType::Reduce(reduce_transform) => reduce_transform.validate(&input_names)?,
            TransformType::Chain(chain) => validate_chain(chain)?,
        }

        Ok(())
    }
}

impl MapTransform {
    fn validate(
        &self,
        input_names: &HashSet<&str>,
        output: &FieldDefinition,
    ) -> Result<(), TransformSpecError> {
        if self.field_mappings.is_empty() {
            return Err(TransformSpecError::EmptyMapMappings);
        }

        let FieldType::Object { fields } = &output.field_type else {
            return Err(TransformSpecError::MapOutputNotObject {
                actual: output.field_type.clone(),
            });
        };

        for (target, mapping) in &self.field_mappings {
            let target_type =
                fields
                    .get(target)
                    .ok_or_else(|| TransformSpecError::UnknownMapOutputField {
                        field: target.clone(),
                    })?;

            mapping.validate(target, target_type, input_names)?;
        }

        Ok(())
    }
}

impl FieldMapping {
    fn validate(
        &self,
        target: &str,
        target_type: &FieldType,
        input_names: &HashSet<&str>,
    ) -> Result<(), TransformSpecError> {
        match self {
            FieldMapping::Direct { field } => {
                if !input_names.contains(field.as_str()) {
                    return Err(TransformSpecError::UnknownInputReference {
                        target: target.to_string(),
                        referenced: field.clone(),
                    });
                }
            }
            FieldMapping::Expression { expression } => {
                if expression.trim().is_empty() {
                    return Err(TransformSpecError::EmptyExpression {
                        target: target.to_string(),
                    });
                }
            }
            FieldMapping::Constant { value } => {
                if !target_type.matches(value) {
                    return Err(TransformSpecError::ConstantTypeMismatch {
                        target: target.to_string(),
                        expected: Box::new(target_type.clone()),
                        actual: Box::new(value.field_type()),
                    });
                }
            }
            FieldMapping::Function { name, arguments } => {
                if name.trim().is_empty() {
                    return Err(TransformSpecError::EmptyFunctionName {
                        target: target.to_string(),
                    });
                }

                for argument in arguments {
                    if !input_names.contains(argument.as_str()) {
                        return Err(TransformSpecError::UnknownFunctionArgument {
                            function: name.clone(),
                            argument: argument.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

impl FilterTransform {
    fn validate(&self, input_names: &HashSet<&str>) -> Result<(), TransformSpecError> {
        self.condition.validate(input_names)
    }
}

impl FilterCondition {
    fn validate(&self, input_names: &HashSet<&str>) -> Result<(), TransformSpecError> {
        match self {
            FilterCondition::Equals { field, .. }
            | FilterCondition::NotEquals { field, .. }
            | FilterCondition::GreaterThan { field, .. }
            | FilterCondition::LessThan { field, .. }
            | FilterCondition::Contains { field, .. } => {
                if !input_names.contains(field.as_str()) {
                    return Err(TransformSpecError::UnknownFilterField {
                        field: field.clone(),
                    });
                }
            }
            FilterCondition::And { conditions } => {
                Self::validate_group("and", conditions, input_names)?;
            }
            FilterCondition::Or { conditions } => {
                Self::validate_group("or", conditions, input_names)?;
            }
        }

        Ok(())
    }

    fn validate_group(
        operator: &'static str,
        conditions: &[FilterCondition],
        input_names: &HashSet<&str>,
    ) -> Result<(), TransformSpecError> {
        if conditions.is_empty() {
            return Err(TransformSpecError::EmptyConditionGroup { operator });
        }

        for condition in conditions {
            condition.validate(input_names)?;
        }

        Ok(())
    }
}

impl ReduceTransform {
    fn validate(&self, input_names: &HashSet<&str>) -> Result<(), TransformSpecError> {
        self.reducer.validate(input_names)?;

        let mut seen: HashSet<&str> = HashSet::new();
        for field in &self.group_by {
            if !input_names.contains(field.as_str()) {
                return Err(TransformSpecError::UnknownGroupByField {
                    field: field.clone(),
                });
            }

            if !seen.insert(field.as_str()) {
                return Err(TransformSpecError::DuplicateGroupByField {
                    field: field.clone(),
                });
            }
        }

        Ok(())
    }
}

impl Reducer {
    fn validate(&self, input_names: &HashSet<&str>) -> Result<(), TransformSpecError> {
        match self {
            Reducer::Count => Ok(()),
            Reducer::Sum { field }
            | Reducer::Average { field }
            | Reducer::Min { field }
            | Reducer::Max { field }
            | Reducer::First { field }
            | Reducer::Last { field } => {
                if !input_names.contains(field.as_str()) {
                    return Err(TransformSpecError::UnknownReducerField {
                        field: field.clone(),
                    });
                }

                Ok(())
            }
        }
    }
}

fn validate_chain(chain: &[TransformSpec]) -> Result<(), TransformSpecError> {
    if chain.is_empty() {
        return Err(TransformSpecError::EmptyChain);
    }

    for nested in chain {
        nested
            .validate()
            .map_err(|source| TransformSpecError::NestedSpecInvalid {
                name: nested.name.clone(),
                source: Box::new(source),
            })?;
    }

    Ok(())
}

fn map_identifier_error(error: FieldDefinitionError) -> TransformSpecError {
    match error {
        FieldDefinitionError::EmptyName => TransformSpecError::EmptyName,
        FieldDefinitionError::NameTooLong { name, max } => {
            TransformSpecError::NameTooLong { name, max }
        }
        FieldDefinitionError::InvalidNameStart { name } => {
            TransformSpecError::InvalidNameStart { name }
        }
        FieldDefinitionError::InvalidNameCharacters { name } => {
            TransformSpecError::InvalidNameCharacters { name }
        }
        FieldDefinitionError::DefaultTypeMismatch { .. } => {
            unreachable!("default mismatch cannot occur when validating transform identifiers")
        }
    }
}
