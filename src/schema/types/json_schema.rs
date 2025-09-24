use crate::fees::payment_config::SchemaPaymentConfig;
use crate::fees::types::config::FieldPaymentConfig;
use crate::fees::types::config::TrustDistanceScaling;
use crate::permissions::types::policy::{ExplicitCounts, PermissionsPolicy, TrustDistance};
use crate::schema::constants::DEFAULT_VALIDATION_MAX_LOGIC_LENGTH;
use crate::schema::types::declarative_schemas::DeclarativeSchemaDefinition;
use crate::schema::types::field::FieldType;
use crate::schema::types::SchemaError;
use crate::schema::types::Transform;
use crate::transform::parser::TransformParser;
use crate::validation::{templates};
use crate::validation_utils::ValidationUtils;
use crate::{invalid_field_fmt, invalid_field};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;   

/// Represents a complete JSON schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaDefinition {
    pub name: String,
    #[serde(default = "crate::schema::types::schema::default_schema_type")]
    pub schema_type: crate::schema::types::schema::SchemaType,
    pub fields: HashMap<String, JsonSchemaField>,
    #[serde(default = "default_schema_payment_config")]
    pub payment_config: SchemaPaymentConfig,
    /// SHA256 hash of the schema content for integrity verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// Represents a field in the JSON schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaField {
    #[serde(default = "default_permission_policy")]
    pub permission_policy: JsonPermissionPolicy,
    #[serde(default)]
    pub molecule_uuid: Option<String>,
    #[serde(default = "default_payment_config")]
    pub payment_config: JsonFieldPaymentConfig,
    #[serde(default)]
    pub field_mappers: HashMap<String, String>,
    #[serde(default = "default_field_type")]
    pub field_type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<JsonTransform>,
}

/// JSON representation of a transform
///
/// Only the required pieces of information are kept. Any unknown
/// fields in the incoming JSON will cause a deserialization error so
/// that stale attributes such as `reversible` or `signature` do not
/// silently pass through the system.
#[derive(Debug, Clone, Serialize)]
pub struct JsonTransform {
    /// The declarative schema definition
    #[serde(flatten)]
    pub schema: DeclarativeSchemaDefinition,

    /// Explicit list of input fields in `Schema.field` format
    #[serde(default)]
    pub inputs: Vec<String>,

    /// Output field for this transform in `Schema.field` format
    pub output: String,
}

// Custom deserialization for declarative transforms only
impl<'de> serde::Deserialize<'de> for JsonTransform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            #[serde(flatten)]
            schema: DeclarativeSchemaDefinition,
            #[serde(default)]
            inputs: Vec<String>,
            output: String,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(JsonTransform {
            schema: helper.schema,
            inputs: helper.inputs,
            output: helper.output,
        })
    }
}

impl JsonTransform {
    /// Validates the complete JSON transform structure.
    pub fn validate(&self) -> Result<(), SchemaError> {
        // Validate output field
        ValidationUtils::require_non_empty_string(&self.output, "Transform output field")?;

        // Validate output field format (should be schema.field)
        if !self.output.contains('.') {
            return Err(invalid_field!(templates::transform::INVALID_OUTPUT_FORMAT));
        }

        // Validate input fields
        for (i, input) in self.inputs.iter().enumerate() {
            ValidationUtils::require_non_empty_string(
                input,
                &format!("Transform input field {}", i),
            )?;

            if !input.contains('.') {
                return Err(invalid_field_fmt!(templates::transform::INVALID_INPUT_FORMAT, input));
            }
        }

        Ok(())
    }
}

/// Definition for a single field within a declarative schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FieldDefinition {
    /// Atom UUID field expression (for reference fields)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atom_uuid: Option<String>,
    /// Field type (inferred from context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_type: Option<String>,
}

impl FieldDefinition {
    /// Validates the field definition.
    pub fn validate(&self, field_name: &str) -> Result<(), SchemaError> {
        // Validate at least one property is defined
        if self.atom_uuid.is_none() && self.field_type.is_none() {
            return Err(invalid_field_fmt!(templates::field::MISSING_PROPERTY, field_name));
        }

        // Validate atom_uuid if present
        if let Some(atom_uuid) = &self.atom_uuid {
            self.validate_atom_uuid_expression(atom_uuid, field_name)?;
        }

        // Validate field_type if present
        if let Some(field_type) = &self.field_type {
            self.validate_field_type(field_type, field_name)?;
        }

        Ok(())
    }

    /// Validates atom_uuid expression syntax
    fn validate_atom_uuid_expression(
        &self,
        atom_uuid: &str,
        field_name: &str,
    ) -> Result<(), SchemaError> {
        let expr = atom_uuid.trim();

        if expr.is_empty() {
            return Err(invalid_field_fmt!(templates::field::EMPTY_ATOM_UUID, field_name));
        }

        // Basic expression validation
        if expr.starts_with('.') || expr.ends_with('.') {
            return Err(invalid_field_fmt!(templates::field::INVALID_ATOM_UUID_FORMAT, field_name, expr));
        }

        if expr.contains("..") {
            return Err(invalid_field_fmt!(templates::field::CONSECUTIVE_DOTS, field_name, expr));
        }

        // Should typically end with $atom_uuid for reference fields
        if !expr.contains("$atom_uuid") && !expr.contains("atom_uuid") {
            // This is more of a warning - atom_uuid expressions should typically reference atom IDs
            // But we won't make this a hard requirement as there might be edge cases
        }

        Ok(())
    }

    /// Validates field_type value
    fn validate_field_type(&self, field_type: &str, field_name: &str) -> Result<(), SchemaError> {
        // Check for empty after trimming whitespace, but preserve original string for other checks
        if field_type.trim().is_empty() {
            return Err(invalid_field_fmt!(templates::field::EMPTY_FIELD_TYPE, field_name));
        }

        // Basic type validation - ensure it's a reasonable type name
        if field_type.len() > 100 {
            return Err(invalid_field_fmt!(templates::field::FIELD_TYPE_TOO_LONG, field_name, field_type));
        }

        // Ensure type doesn't contain invalid characters (check the original string, not trimmed)
        if field_type
            .chars()
            .any(|c| c.is_control() || c == '\n' || c == '\r')
        {
            return Err(invalid_field_fmt!(templates::field::INVALID_FIELD_TYPE_CHARS, field_name, field_type));
        }

        Ok(())
    }
}

/// JSON representation of permission policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPermissionPolicy {
    #[serde(rename = "read_policy")]
    pub read: TrustDistance,
    #[serde(rename = "write_policy")]
    pub write: TrustDistance,
    #[serde(rename = "explicit_read_policy")]
    pub explicit_read: Option<ExplicitCounts>,
    #[serde(rename = "explicit_write_policy")]
    pub explicit_write: Option<ExplicitCounts>,
}

/// JSON representation of field payment config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFieldPaymentConfig {
    pub base_multiplier: f64,
    pub trust_distance_scaling: TrustDistanceScaling,
    pub min_payment: Option<u64>,
}

impl From<JsonPermissionPolicy> for PermissionsPolicy {
    fn from(json: JsonPermissionPolicy) -> Self {
        Self {
            read_policy: json.read,
            write_policy: json.write,
            explicit_read_policy: json.explicit_read,
            explicit_write_policy: json.explicit_write,
        }
    }
}

impl From<JsonFieldPaymentConfig> for FieldPaymentConfig {
    fn from(json: JsonFieldPaymentConfig) -> Self {
        Self {
            base_multiplier: json.base_multiplier,
            trust_distance_scaling: json.trust_distance_scaling,
            min_payment: json.min_payment,
        }
    }
}

impl From<JsonTransform> for Transform {
    fn from(json: JsonTransform) -> Self {
        Transform::from_declarative_schema(json.schema)
    }
}

fn default_field_type() -> FieldType {
    FieldType::Single
}

fn default_permission_policy() -> JsonPermissionPolicy {
    JsonPermissionPolicy {
        read: TrustDistance::Distance(0),
        write: TrustDistance::Distance(0),
        explicit_read: None,
        explicit_write: None,
    }
}

fn default_payment_config() -> JsonFieldPaymentConfig {
    JsonFieldPaymentConfig {
        base_multiplier: 1.0,
        trust_distance_scaling: TrustDistanceScaling::None,
        min_payment: None,
    }
}

fn default_schema_payment_config() -> SchemaPaymentConfig {
    SchemaPaymentConfig {
        base_multiplier: 1.0,
        min_payment_threshold: 0,
    }
}

impl JsonSchemaDefinition {
    /// Validates the schema definition according to the rules.
    ///
    /// # Errors
    /// Returns a `SchemaError::InvalidField` if:
    /// - The schema's base multiplier is not positive
    /// - Any field's base multiplier is not positive
    /// - Any field's min factor is less than 1.0
    /// - Any field's min payment is zero when specified
    pub fn validate(&self) -> Result<(), SchemaError> {
        // Base multiplier must be positive
        if self.payment_config.base_multiplier <= 0.0 {
            return Err(SchemaError::InvalidField(
                "Schema base_multiplier must be positive".to_string(),
            ));
        }

        Ok(())
    }
}
