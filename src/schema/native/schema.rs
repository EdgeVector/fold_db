use crate::transform::native::{FieldDefinition, FieldDefinitionError, FieldType, FieldValue};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Native representation of a schema with strongly typed field definitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NativeSchema {
    name: String,
    fields: HashMap<String, FieldDefinition>,
    key_config: KeyConfig,
}

/// Key configuration describing how records are addressed in storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyConfig {
    /// Schemas indexed by a single field.
    Single { key_field: String },
    /// Schemas indexed by a hash/range pair (legacy range behaviour).
    Range {
        hash_field: String,
        range_field: String,
    },
    /// Schemas indexed by a hash/range pair (HashRange semantics).
    HashRange {
        hash_field: String,
        range_field: String,
    },
}

/// Builder that validates schema definitions before instantiating [`NativeSchema`].
#[derive(Debug, Clone)]
pub struct NativeSchemaBuilder {
    name: String,
    key_config: KeyConfig,
    fields: HashMap<String, FieldDefinition>,
}

/// Errors encountered while constructing native schema definitions.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NativeSchemaError {
    /// The schema must contain at least one field definition.
    #[error("schema '{schema}' must contain at least one field")]
    EmptySchema { schema: String },

    /// Duplicate field definitions are not permitted.
    #[error("schema '{schema}' already defines field '{field}'")]
    DuplicateField { schema: String, field: String },

    /// The stored definition name must match the key used in the map.
    #[error(
        "field definition name '{definition_name}' does not match registration key '{field}' in schema '{schema}'"
    )]
    FieldNameMismatch {
        schema: String,
        field: String,
        definition_name: String,
    },

    /// Field definitions must pass validation before registration.
    #[error("field '{field}' in schema '{schema}' is invalid: {source}")]
    InvalidFieldDefinition {
        schema: String,
        field: String,
        #[source]
        source: FieldDefinitionError,
    },

    /// The key configuration references a field that does not exist.
    #[error("schema '{schema}' is missing required key field '{field}'")]
    MissingKeyField { schema: String, field: String },

    /// Key fields must be marked as required.
    #[error("key field '{field}' in schema '{schema}' must be marked as required")]
    KeyFieldNotRequired { schema: String, field: String },

    /// Key configuration cannot reference the same field multiple times.
    #[error("key configuration for schema '{schema}' references '{field}' multiple times")]
    DuplicateKeyField { schema: String, field: String },

    /// Key fields cannot rely on null-only types.
    #[error("key field '{field}' in schema '{schema}' cannot use type {actual:?}")]
    InvalidKeyFieldType {
        schema: String,
        field: String,
        actual: FieldType,
    },
}

/// Errors emitted while validating data against a [`NativeSchema`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchemaValidationError {
    /// Payload supplied a field that is not declared by the schema.
    #[error("schema '{schema}' does not define field '{field}'")]
    UnknownField { schema: String, field: String },

    /// A required field is missing from the payload.
    #[error("schema '{schema}' is missing required field '{field}'")]
    MissingRequiredField { schema: String, field: String },

    /// A field value does not match the declared type.
    #[error(
        "field '{field}' in schema '{schema}' has type mismatch: expected {expected:?}, got {actual:?}"
    )]
    TypeMismatch {
        schema: String,
        field: String,
        expected: Box<FieldType>,
        actual: Box<FieldType>,
    },

    /// Optional field could not resolve a default value while normalising data.
    #[error("schema '{schema}' could not resolve default for optional field '{field}'")]
    DefaultResolutionFailed { schema: String, field: String },
}

impl NativeSchema {
    /// Create a builder for assembling a schema incrementally.
    #[must_use]
    pub fn builder(name: impl Into<String>, key_config: KeyConfig) -> NativeSchemaBuilder {
        NativeSchemaBuilder::new(name, key_config)
    }

    /// Construct a schema directly from an iterator of field definitions.
    pub fn try_from_definitions(
        name: impl Into<String>,
        key_config: KeyConfig,
        definitions: impl IntoIterator<Item = FieldDefinition>,
    ) -> Result<Self, NativeSchemaError> {
        let mut builder = Self::builder(name, key_config);
        builder.add_fields(definitions)?;
        builder.build()
    }

    /// Name of the schema.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Key configuration associated with the schema.
    #[must_use]
    pub fn key_config(&self) -> &KeyConfig {
        &self.key_config
    }

    /// Access the registered field definitions.
    #[must_use]
    pub fn fields(&self) -> &HashMap<String, FieldDefinition> {
        &self.fields
    }

    /// Retrieve a field definition by name.
    #[must_use]
    pub fn get_field(&self, field: &str) -> Option<&FieldDefinition> {
        self.fields.get(field)
    }

    /// Return the number of registered fields.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Check whether the schema contains at least one field.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Register an additional field after the schema has been created.
    pub fn add_field(&mut self, definition: FieldDefinition) -> Result<(), NativeSchemaError> {
        let field_name = definition.name.clone();
        self.ensure_definition_valid(&field_name, &definition)?;

        if self.fields.contains_key(&field_name) {
            return Err(NativeSchemaError::DuplicateField {
                schema: self.name.clone(),
                field: field_name,
            });
        }

        self.fields.insert(field_name.clone(), definition);
        self.validate_key_config()?;
        Ok(())
    }

    /// Validate a payload against the schema without mutating it.
    pub fn validate_payload(
        &self,
        payload: &HashMap<String, FieldValue>,
    ) -> Result<(), SchemaValidationError> {
        self.ensure_known_fields(payload.keys())?;

        for (field_name, definition) in &self.fields {
            match payload.get(field_name) {
                Some(value) => {
                    if !definition.field_type.matches(value) {
                        return Err(SchemaValidationError::TypeMismatch {
                            schema: self.name.clone(),
                            field: field_name.clone(),
                            expected: Box::new(definition.field_type.clone()),
                            actual: Box::new(value.field_type()),
                        });
                    }
                }
                None if definition.required => {
                    return Err(SchemaValidationError::MissingRequiredField {
                        schema: self.name.clone(),
                        field: field_name.clone(),
                    });
                }
                None => {}
            }
        }

        Ok(())
    }

    /// Validate payload types and populate omitted optional fields with defaults.
    pub fn normalise_payload(
        &self,
        payload: &mut HashMap<String, FieldValue>,
    ) -> Result<(), SchemaValidationError> {
        self.ensure_known_fields(payload.keys())?;

        for (field_name, definition) in &self.fields {
            match payload.get(field_name) {
                Some(value) => {
                    if !definition.field_type.matches(value) {
                        return Err(SchemaValidationError::TypeMismatch {
                            schema: self.name.clone(),
                            field: field_name.clone(),
                            expected: Box::new(definition.field_type.clone()),
                            actual: Box::new(value.field_type()),
                        });
                    }
                }
                None if definition.required => {
                    return Err(SchemaValidationError::MissingRequiredField {
                        schema: self.name.clone(),
                        field: field_name.clone(),
                    });
                }
                None => {
                    let Some(default_value) = definition.effective_default() else {
                        return Err(SchemaValidationError::DefaultResolutionFailed {
                            schema: self.name.clone(),
                            field: field_name.clone(),
                        });
                    };
                    payload.insert(field_name.clone(), default_value);
                }
            }
        }

        Ok(())
    }

    /// Produce a payload that includes defaults for optional fields.
    pub fn project_payload(
        &self,
        payload: &HashMap<String, FieldValue>,
    ) -> Result<HashMap<String, FieldValue>, SchemaValidationError> {
        let mut normalised = payload.clone();
        self.normalise_payload(&mut normalised)?;
        Ok(normalised)
    }

    fn validate_structure(&self) -> Result<(), NativeSchemaError> {
        if self.fields.is_empty() {
            return Err(NativeSchemaError::EmptySchema {
                schema: self.name.clone(),
            });
        }

        for (field_name, definition) in &self.fields {
            if definition.name != *field_name {
                return Err(NativeSchemaError::FieldNameMismatch {
                    schema: self.name.clone(),
                    field: field_name.clone(),
                    definition_name: definition.name.clone(),
                });
            }

            definition
                .validate()
                .map_err(|source| NativeSchemaError::InvalidFieldDefinition {
                    schema: self.name.clone(),
                    field: field_name.clone(),
                    source,
                })?;
        }

        self.validate_key_config()
    }

    fn validate_key_config(&self) -> Result<(), NativeSchemaError> {
        let mut seen = HashSet::new();
        for field_name in self.key_config.field_names() {
            if !seen.insert(field_name) {
                return Err(NativeSchemaError::DuplicateKeyField {
                    schema: self.name.clone(),
                    field: field_name.to_string(),
                });
            }

            let Some(definition) = self.fields.get(field_name) else {
                return Err(NativeSchemaError::MissingKeyField {
                    schema: self.name.clone(),
                    field: field_name.to_string(),
                });
            };

            if !definition.required {
                return Err(NativeSchemaError::KeyFieldNotRequired {
                    schema: self.name.clone(),
                    field: field_name.to_string(),
                });
            }

            let field_type = definition.field_type.clone();
            if field_type == FieldType::Null {
                return Err(NativeSchemaError::InvalidKeyFieldType {
                    schema: self.name.clone(),
                    field: field_name.to_string(),
                    actual: field_type,
                });
            }
        }

        Ok(())
    }

    fn ensure_definition_valid(
        &self,
        field_name: &str,
        definition: &FieldDefinition,
    ) -> Result<(), NativeSchemaError> {
        if definition.name != field_name {
            return Err(NativeSchemaError::FieldNameMismatch {
                schema: self.name.clone(),
                field: field_name.to_string(),
                definition_name: definition.name.clone(),
            });
        }

        definition
            .validate()
            .map_err(|source| NativeSchemaError::InvalidFieldDefinition {
                schema: self.name.clone(),
                field: field_name.to_string(),
                source,
            })
    }

    fn ensure_known_fields<'a>(
        &self,
        payload_fields: impl IntoIterator<Item = &'a String>,
    ) -> Result<(), SchemaValidationError> {
        for field in payload_fields {
            if !self.fields.contains_key(field.as_str()) {
                return Err(SchemaValidationError::UnknownField {
                    schema: self.name.clone(),
                    field: field.clone(),
                });
            }
        }

        Ok(())
    }
}

impl NativeSchemaBuilder {
    /// Create a new builder for the provided schema name and key configuration.
    #[must_use]
    pub fn new(name: impl Into<String>, key_config: KeyConfig) -> Self {
        Self {
            name: name.into(),
            key_config,
            fields: HashMap::new(),
        }
    }

    /// Add a field definition to the schema under construction.
    pub fn add_field(
        &mut self,
        definition: FieldDefinition,
    ) -> Result<&mut Self, NativeSchemaError> {
        let field_name = definition.name.clone();

        if self.fields.contains_key(&field_name) {
            return Err(NativeSchemaError::DuplicateField {
                schema: self.name.clone(),
                field: field_name,
            });
        }

        definition
            .validate()
            .map_err(|source| NativeSchemaError::InvalidFieldDefinition {
                schema: self.name.clone(),
                field: field_name.clone(),
                source,
            })?;

        self.fields.insert(field_name, definition);
        Ok(self)
    }

    /// Extend the builder with multiple field definitions.
    pub fn add_fields(
        &mut self,
        definitions: impl IntoIterator<Item = FieldDefinition>,
    ) -> Result<&mut Self, NativeSchemaError> {
        for definition in definitions {
            self.add_field(definition)?;
        }
        Ok(self)
    }

    /// Finalise the builder into a [`NativeSchema`], running validation checks.
    pub fn build(self) -> Result<NativeSchema, NativeSchemaError> {
        let schema = NativeSchema {
            name: self.name,
            fields: self.fields,
            key_config: self.key_config,
        };

        schema.validate_structure()?;
        Ok(schema)
    }
}

impl KeyConfig {
    fn field_names(&self) -> Vec<&str> {
        match self {
            KeyConfig::Single { key_field } => vec![key_field.as_str()],
            KeyConfig::Range {
                hash_field,
                range_field,
            }
            | KeyConfig::HashRange {
                hash_field,
                range_field,
            } => vec![hash_field.as_str(), range_field.as_str()],
        }
    }
}
