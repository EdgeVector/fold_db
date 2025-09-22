use super::types::{FieldType, FieldValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_FIELD_NAME_LENGTH: usize = 64;

fn default_required() -> bool {
    true
}

/// Native field definition pairing metadata with validation logic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldDefinition {
    /// Field identifier used within transform specifications.
    pub name: String,
    /// Declared field type that all values must satisfy.
    pub field_type: FieldType,
    /// Whether the field must be present in transform output/input payloads.
    #[serde(default = "default_required")]
    pub required: bool,
    /// Optional default value used when the field is omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub default_value: Option<FieldValue>,
}

/// Validation errors emitted when field definitions violate invariants.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FieldDefinitionError {
    /// Field name is empty or whitespace.
    #[error("field name cannot be empty")]
    EmptyName,
    /// Field name exceeds the allowed length constraint.
    #[error("field name '{name}' exceeds maximum length of {max} characters")]
    NameTooLong { name: String, max: usize },
    /// Field name starts with an invalid character.
    #[error("field name '{name}' must start with an ASCII letter or underscore")]
    InvalidNameStart { name: String },
    /// Field name contains unsupported characters.
    #[error("field name '{name}' contains invalid characters; only ASCII letters, digits, and underscores are allowed")]
    InvalidNameCharacters { name: String },
    /// Default value does not match the declared type.
    #[error(
        "default value for field '{name}' does not match declared type (expected {declared:?}, got {actual:?})"
    )]
    DefaultTypeMismatch {
        name: String,
        declared: Box<FieldType>,
        actual: Box<FieldType>,
    },
}

impl FieldDefinition {
    /// Construct a new field definition with the provided name and type.
    #[must_use]
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            required: true,
            default_value: None,
        }
    }

    /// Mark the field as required or optional.
    #[must_use]
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Attach an explicit default value to the field definition.
    #[must_use]
    pub fn with_default(mut self, default_value: FieldValue) -> Self {
        self.default_value = Some(default_value);
        self
    }

    /// Validate the field definition invariants.
    pub fn validate(&self) -> Result<(), FieldDefinitionError> {
        Self::validate_name(&self.name)?;

        if let Some(default_value) = &self.default_value {
            if !self.field_type.matches(default_value) {
                return Err(FieldDefinitionError::DefaultTypeMismatch {
                    name: self.name.clone(),
                    declared: Box::new(self.field_type.clone()),
                    actual: Box::new(default_value.field_type()),
                });
            }
        }

        Ok(())
    }

    /// Resolve the effective default value for the field.
    ///
    /// Explicit defaults win. Optional fields without an explicit default fall back to
    /// the deterministic default generated from the declared [`FieldType`]. Required
    /// fields without explicit defaults return `None` to signal that callers must
    /// provide a value.
    #[must_use]
    pub fn effective_default(&self) -> Option<FieldValue> {
        match (&self.default_value, self.required) {
            (Some(value), _) => Some(value.clone()),
            (None, false) => Some(self.field_type.default_value()),
            (None, true) => None,
        }
    }

    fn validate_name(name: &str) -> Result<(), FieldDefinitionError> {
        let trimmed = name.trim();

        if trimmed.is_empty() {
            return Err(FieldDefinitionError::EmptyName);
        }

        if trimmed.len() != name.len() {
            return Err(FieldDefinitionError::InvalidNameCharacters {
                name: name.to_string(),
            });
        }

        if trimmed.len() > MAX_FIELD_NAME_LENGTH {
            return Err(FieldDefinitionError::NameTooLong {
                name: trimmed.to_string(),
                max: MAX_FIELD_NAME_LENGTH,
            });
        }

        let mut chars = trimmed.chars();
        let Some(first_char) = chars.next() else {
            return Err(FieldDefinitionError::EmptyName);
        };

        if !Self::is_valid_start_char(first_char) {
            return Err(FieldDefinitionError::InvalidNameStart {
                name: trimmed.to_string(),
            });
        }

        if chars.any(|ch| !Self::is_valid_continuation_char(ch)) {
            return Err(FieldDefinitionError::InvalidNameCharacters {
                name: trimmed.to_string(),
            });
        }

        Ok(())
    }

    fn is_valid_start_char(ch: char) -> bool {
        ch.is_ascii_alphabetic() || ch == '_'
    }

    fn is_valid_continuation_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_'
    }
}
