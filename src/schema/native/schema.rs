use super::errors::{KeyConfigError, NativeSchemaError, SchemaValidationError};
use crate::transform::native::{FieldDefinition, FieldType, FieldValue};
use crate::validation_utils::ValidationUtils;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Declarative key configuration describing how records are uniquely addressed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyConfig {
    /// Single-key schema addressed by a single field.
    Single { key_field: String },
    /// Range schema that orders data by a range field.
    Range { range_field: String },
    /// Hash-range schema partitioned by hash and range expressions.
    HashRange {
        hash_field: String,
        range_field: String,
    },
}

impl KeyConfig {
    /// Return a copy of the key configuration with trimmed field names.
    #[must_use]
    pub fn normalized(self) -> Self {
        match self {
            Self::Single { key_field } => Self::Single {
                key_field: key_field.trim().to_owned(),
            },
            Self::Range { range_field } => Self::Range {
                range_field: range_field.trim().to_owned(),
            },
            Self::HashRange {
                hash_field,
                range_field,
            } => Self::HashRange {
                hash_field: hash_field.trim().to_owned(),
                range_field: range_field.trim().to_owned(),
            },
        }
    }

    /// Validate structural requirements for the key configuration.
    pub fn validate(&self) -> Result<(), KeyConfigError> {
        match self {
            Self::Single { key_field } => Self::ensure_present(key_field, "key_field"),
            Self::Range { range_field } => Self::ensure_present(range_field, "range_field"),
            Self::HashRange {
                hash_field,
                range_field,
            } => {
                Self::ensure_present(hash_field, "hash_field")?;
                Self::ensure_present(range_field, "range_field")?;
                if hash_field == range_field {
                    return Err(KeyConfigError::DuplicateHashAndRange);
                }
                Ok(())
            }
        }
    }

    fn ensure_present(value: &str, field: &'static str) -> Result<(), KeyConfigError> {
        if value.trim().is_empty() {
            Err(KeyConfigError::EmptyField { field })
        } else {
            Ok(())
        }
    }

    /// List all field names referenced by the key configuration.
    #[must_use]
    pub fn key_fields(&self) -> Vec<&str> {
        match self {
            Self::Single { key_field } => vec![key_field.as_str()],
            Self::Range { range_field } => vec![range_field.as_str()],
            Self::HashRange {
                hash_field,
                range_field,
            } => vec![hash_field.as_str(), range_field.as_str()],
        }
    }
}

/// Strongly typed schema definition backed by native transform primitives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeSchema {
    name: String,
    key_config: KeyConfig,
    fields: HashMap<String, FieldDefinition>,
}

impl NativeSchema {
    /// Create a new schema definition.
    pub fn new(name: impl Into<String>, key_config: KeyConfig) -> Result<Self, NativeSchemaError> {
        let name = name.into();
        ValidationUtils::require_valid_schema_name(&name).map_err(|err| {
            NativeSchemaError::InvalidName {
                name: name.clone(),
                reason: err.to_string(),
            }
        })?;

        let normalized_config = key_config.normalized();
        normalized_config
            .validate()
            .map_err(|source| NativeSchemaError::InvalidKeyConfig {
                schema: name.clone(),
                source,
            })?;

        Ok(Self {
            name,
            key_config: normalized_config,
            fields: HashMap::new(),
        })
    }

    /// Schema identifier.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Access the key configuration.
    #[must_use]
    pub fn key_config(&self) -> &KeyConfig {
        &self.key_config
    }

    /// Borrow the defined fields.
    #[must_use]
    pub fn fields(&self) -> &HashMap<String, FieldDefinition> {
        &self.fields
    }

    /// Returns the number of fields defined in the schema.
    #[must_use]
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Determine whether the schema defines a particular field.
    #[must_use]
    pub fn has_field(&self, field_name: &str) -> bool {
        self.fields.contains_key(field_name)
    }

    /// Retrieve a field definition by name.
    pub fn get_field(&self, field_name: &str) -> Option<&FieldDefinition> {
        self.fields.get(field_name)
    }

    /// Add a new field definition to the schema.
    pub fn add_field(&mut self, field: FieldDefinition) -> Result<(), NativeSchemaError> {
        field
            .validate()
            .map_err(|source| NativeSchemaError::InvalidFieldDefinition {
                schema: self.name.clone(),
                source,
            })?;
        let field_name = field.name.clone();

        if self.fields.contains_key(&field_name) {
            return Err(NativeSchemaError::DuplicateField {
                schema: self.name.clone(),
                field_name,
            });
        }

        self.fields.insert(field_name, field);
        Ok(())
    }

    /// Add multiple field definitions atomically.
    pub fn add_fields<I>(&mut self, fields: I) -> Result<(), NativeSchemaError>
    where
        I: IntoIterator<Item = FieldDefinition>,
    {
        for field in fields {
            self.add_field(field)?;
        }
        Ok(())
    }

    /// Ensure key fields exist and field definitions remain valid.
    pub fn validate_integrity(&self) -> Result<(), NativeSchemaError> {
        for key_field in self.key_config.key_fields() {
            if !self.fields.contains_key(key_field) {
                return Err(NativeSchemaError::MissingKeyField {
                    schema: self.name.clone(),
                    field_name: key_field.to_string(),
                });
            }
        }

        for field in self.fields.values() {
            field
                .validate()
                .map_err(|source| NativeSchemaError::InvalidFieldDefinition {
                    schema: self.name.clone(),
                    source,
                })?;
        }

        Ok(())
    }

    /// Populate missing optional fields with their effective defaults.
    pub fn apply_defaults(&self, record: &mut HashMap<String, FieldValue>) {
        for (field_name, field_def) in &self.fields {
            if record.contains_key(field_name) {
                continue;
            }

            if let Some(default_value) = field_def.effective_default() {
                record.insert(field_name.clone(), default_value);
            }
        }
    }

    /// Validate a record's fields against the schema definition.
    pub fn validate_record(
        &self,
        record: &HashMap<String, FieldValue>,
    ) -> Result<(), SchemaValidationError> {
        for (field_name, field_def) in &self.fields {
            match record.get(field_name) {
                Some(value) => {
                    if !field_def.field_type.matches(value) {
                        return Err(SchemaValidationError::TypeMismatch {
                            field_name: field_name.clone(),
                            expected: Box::new(field_def.field_type.clone()),
                            actual: Box::new(value.field_type()),
                        });
                    }
                }
                None if field_def.required => {
                    return Err(SchemaValidationError::RequiredFieldMissing {
                        field_name: field_name.clone(),
                    });
                }
                None => {}
            }
        }

        for field_name in record.keys() {
            if !self.fields.contains_key(field_name) {
                return Err(SchemaValidationError::UnexpectedField {
                    field_name: field_name.clone(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::native::FieldValue;
    use std::collections::HashMap;

    fn base_schema() -> NativeSchema {
        let mut schema = NativeSchema::new(
            "BlogPost",
            KeyConfig::Single {
                key_field: "id".into(),
            },
        )
        .expect("schema");
        schema
            .add_field(FieldDefinition::new("id", FieldType::String))
            .expect("id");
        schema
            .add_field(FieldDefinition::new("title", FieldType::String))
            .expect("title");
        schema
            .add_field(FieldDefinition::new("views", FieldType::Integer).with_required(false))
            .expect("views");
        schema
    }

    #[test]
    fn key_config_normalization_trims_whitespace() {
        let schema = NativeSchema::new(
            "Example",
            KeyConfig::Single {
                key_field: "   identifier   ".into(),
            },
        )
        .expect("schema creation");

        assert_eq!(schema.key_config.key_fields(), vec!["identifier"]);
    }

    #[test]
    fn validation_rejects_missing_key_field() {
        let schema = NativeSchema::new(
            "Example",
            KeyConfig::Single {
                key_field: "id".into(),
            },
        )
        .expect("schema creation");

        let err = schema.validate_integrity().expect_err("missing key field");
        assert!(
            matches!(err, NativeSchemaError::MissingKeyField { field_name, .. } if field_name == "id")
        );
    }

    #[test]
    fn apply_defaults_fills_optional_fields() {
        let mut schema = base_schema();
        schema
            .add_field(
                FieldDefinition::new("status", FieldType::String)
                    .with_required(false)
                    .with_default(FieldValue::String("draft".into())),
            )
            .expect("status");

        let mut record = HashMap::new();
        record.insert("id".into(), FieldValue::String("abc".into()));
        record.insert("title".into(), FieldValue::String("Post".into()));
        schema.apply_defaults(&mut record);

        assert_eq!(
            record.get("status"),
            Some(&FieldValue::String("draft".into()))
        );
        assert_eq!(record.get("views"), Some(&FieldValue::Integer(0)));
    }

    #[test]
    fn validate_record_detects_missing_required_field() {
        let schema = base_schema();
        let mut record = HashMap::new();
        record.insert("id".into(), FieldValue::String("abc".into()));

        let err = schema
            .validate_record(&record)
            .expect_err("missing required field");
        assert!(matches!(
            err,
            SchemaValidationError::RequiredFieldMissing { field_name }
            if field_name == "title"
        ));
    }

    #[test]
    fn validate_record_detects_type_mismatch() {
        let mut schema = base_schema();
        schema
            .add_field(FieldDefinition::new("published", FieldType::Boolean))
            .expect("published");

        let mut record = HashMap::new();
        record.insert("id".into(), FieldValue::String("abc".into()));
        record.insert("title".into(), FieldValue::String("Post".into()));
        record.insert("published".into(), FieldValue::Integer(1));

        let err = schema.validate_record(&record).expect_err("type mismatch");
        assert!(matches!(
            err,
            SchemaValidationError::TypeMismatch { field_name, .. }
            if field_name == "published"
        ));
    }

    #[test]
    fn validate_record_detects_unknown_fields() {
        let schema = base_schema();
        let mut record = HashMap::new();
        record.insert("id".into(), FieldValue::String("abc".into()));
        record.insert("title".into(), FieldValue::String("Post".into()));
        record.insert("extra".into(), FieldValue::String("value".into()));

        let err = schema
            .validate_record(&record)
            .expect_err("unexpected field");
        assert!(matches!(
            err,
            SchemaValidationError::UnexpectedField { field_name }
            if field_name == "extra"
        ));
    }
}
