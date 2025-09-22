use crate::transform::native::{FieldDefinition, FieldDefinitionError, FieldType, FieldValue};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;
use thiserror::Error;

/// Errors emitted by [`JsonBoundaryLayer`] during schema registration or conversion.
#[derive(Debug, Error)]
pub enum JsonBoundaryError {
    /// The requested schema has not been registered.
    #[error("schema '{schema}' is not registered with the JSON boundary layer")]
    SchemaNotRegistered { schema: String },

    /// Input payload did not contain a JSON object at the root.
    #[error("payload for schema '{schema}' must be a JSON object")]
    InvalidPayloadStructure { schema: String },

    /// A field name present in the payload or native data is not recognised.
    #[error("schema '{schema}' does not permit unknown field '{field}'")]
    UnknownField { schema: String, field: String },

    /// A required field is absent from the payload or native data map.
    #[error("missing required field '{field}' for schema '{schema}'")]
    MissingRequiredField { schema: String, field: String },

    /// A field value does not match the declared [`FieldType`].
    #[error(
        "field '{field}' in schema '{schema}' has type mismatch: expected {expected:?}, got {actual:?}"
    )]
    TypeMismatch {
        schema: String,
        field: String,
        expected: Box<FieldType>,
        actual: Box<FieldType>,
    },

    /// A field definition fails validation during registration.
    #[error("field definition error for '{field}' in schema '{schema}': {source}")]
    InvalidFieldDefinition {
        schema: String,
        field: String,
        #[source]
        source: FieldDefinitionError,
    },

    /// The definition name does not match the key under which it was registered.
    #[error(
        "field definition name '{definition_name}' does not match key '{field}' in schema '{schema}'"
    )]
    FieldNameMismatch {
        schema: String,
        field: String,
        definition_name: String,
    },

    /// Optional field lacks a resolvable default when omitted.
    #[error("optional field '{field}' in schema '{schema}' lacks a default value")]
    DefaultResolutionFailed { schema: String, field: String },
}

/// Schema configuration consumed by [`JsonBoundaryLayer`].
#[derive(Debug, Clone)]
pub struct JsonBoundarySchema {
    name: String,
    fields: HashMap<String, FieldDefinition>,
    allow_additional_fields: bool,
}

impl JsonBoundarySchema {
    /// Create a new schema configuration from an explicit field map.
    #[must_use]
    pub fn new(name: impl Into<String>, fields: HashMap<String, FieldDefinition>) -> Self {
        Self {
            name: name.into(),
            fields,
            allow_additional_fields: false,
        }
    }

    /// Create a new schema using field definitions keyed by their internal names.
    #[must_use]
    pub fn from_definitions(
        name: impl Into<String>,
        definitions: impl IntoIterator<Item = FieldDefinition>,
    ) -> Self {
        let mut fields = HashMap::new();
        for definition in definitions {
            fields.insert(definition.name.clone(), definition);
        }
        Self::new(name, fields)
    }

    /// Configure whether unknown fields should be accepted and passed through.
    #[must_use]
    pub fn allow_additional_fields(mut self, allow: bool) -> Self {
        self.allow_additional_fields = allow;
        self
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn fields(&self) -> &HashMap<String, FieldDefinition> {
        &self.fields
    }

    #[must_use]
    pub fn allows_additional_fields(&self) -> bool {
        self.allow_additional_fields
    }
}

/// Conversion layer that maintains JSON compatibility at system boundaries.
#[derive(Debug, Default)]
pub struct JsonBoundaryLayer {
    schemas: HashMap<String, JsonBoundarySchema>,
}

impl JsonBoundaryLayer {
    /// Construct an empty boundary layer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or replace a schema configuration.
    pub fn register_schema(&mut self, schema: JsonBoundarySchema) -> Result<(), JsonBoundaryError> {
        for (field_name, definition) in &schema.fields {
            if definition.name != *field_name {
                return Err(JsonBoundaryError::FieldNameMismatch {
                    schema: schema.name.clone(),
                    field: field_name.clone(),
                    definition_name: definition.name.clone(),
                });
            }

            definition
                .validate()
                .map_err(|source| JsonBoundaryError::InvalidFieldDefinition {
                    schema: schema.name.clone(),
                    field: field_name.clone(),
                    source,
                })?;
        }

        self.schemas.insert(schema.name.clone(), schema);
        Ok(())
    }

    /// Convert inbound JSON into native field values, applying defaults and validation.
    pub fn json_to_native(
        &self,
        schema_name: &str,
        json_data: &JsonValue,
    ) -> Result<HashMap<String, FieldValue>, JsonBoundaryError> {
        let schema = self.fetch_schema(schema_name)?;
        let object =
            json_data
                .as_object()
                .ok_or_else(|| JsonBoundaryError::InvalidPayloadStructure {
                    schema: schema_name.to_string(),
                })?;

        if !schema.allows_additional_fields() {
            if let Some(extra) = object
                .keys()
                .find(|key| !schema.fields().contains_key(*key))
            {
                return Err(JsonBoundaryError::UnknownField {
                    schema: schema_name.to_string(),
                    field: extra.clone(),
                });
            }
        }

        let mut native_data = HashMap::with_capacity(schema.fields().len());

        for (field_name, definition) in schema.fields() {
            match object.get(field_name) {
                Some(value) => {
                    let native_value = FieldValue::from_json_value(value.clone());
                    if !definition.field_type.matches(&native_value) {
                        return Err(JsonBoundaryError::TypeMismatch {
                            schema: schema_name.to_string(),
                            field: field_name.clone(),
                            expected: Box::new(definition.field_type.clone()),
                            actual: Box::new(native_value.field_type()),
                        });
                    }
                    native_data.insert(field_name.clone(), native_value);
                }
                None => {
                    if definition.required {
                        return Err(JsonBoundaryError::MissingRequiredField {
                            schema: schema_name.to_string(),
                            field: field_name.clone(),
                        });
                    }

                    let Some(default_value) = definition.effective_default() else {
                        return Err(JsonBoundaryError::DefaultResolutionFailed {
                            schema: schema_name.to_string(),
                            field: field_name.clone(),
                        });
                    };

                    native_data.insert(field_name.clone(), default_value);
                }
            }
        }

        if schema.allows_additional_fields() {
            for (field_name, value) in object {
                if schema.fields().contains_key(field_name) {
                    continue;
                }
                native_data.insert(
                    field_name.clone(),
                    FieldValue::from_json_value(value.clone()),
                );
            }
        }

        Ok(native_data)
    }

    /// Convert native field values to JSON suitable for API responses.
    pub fn native_to_json(
        &self,
        schema_name: &str,
        native_data: &HashMap<String, FieldValue>,
    ) -> Result<JsonValue, JsonBoundaryError> {
        let schema = self.fetch_schema(schema_name)?;

        if !schema.allows_additional_fields() {
            if let Some(extra) = native_data
                .keys()
                .find(|key| !schema.fields().contains_key(*key))
            {
                return Err(JsonBoundaryError::UnknownField {
                    schema: schema_name.to_string(),
                    field: extra.clone(),
                });
            }
        }

        let mut json_map = JsonMap::new();

        for (field_name, definition) in schema.fields() {
            match native_data.get(field_name) {
                Some(value) => {
                    if !definition.field_type.matches(value) {
                        return Err(JsonBoundaryError::TypeMismatch {
                            schema: schema_name.to_string(),
                            field: field_name.clone(),
                            expected: Box::new(definition.field_type.clone()),
                            actual: Box::new(value.field_type()),
                        });
                    }
                    json_map.insert(field_name.clone(), value.to_json_value());
                }
                None => {
                    if definition.required {
                        return Err(JsonBoundaryError::MissingRequiredField {
                            schema: schema_name.to_string(),
                            field: field_name.clone(),
                        });
                    }
                    let Some(default_value) = definition.effective_default() else {
                        return Err(JsonBoundaryError::DefaultResolutionFailed {
                            schema: schema_name.to_string(),
                            field: field_name.clone(),
                        });
                    };
                    json_map.insert(field_name.clone(), default_value.to_json_value());
                }
            }
        }

        if schema.allows_additional_fields() {
            for (field_name, value) in native_data {
                if schema.fields().contains_key(field_name) {
                    continue;
                }
                json_map.insert(field_name.clone(), value.to_json_value());
            }
        }

        Ok(JsonValue::Object(json_map))
    }

    fn fetch_schema(&self, schema_name: &str) -> Result<&JsonBoundarySchema, JsonBoundaryError> {
        self.schemas
            .get(schema_name)
            .ok_or_else(|| JsonBoundaryError::SchemaNotRegistered {
                schema: schema_name.to_string(),
            })
    }
}
