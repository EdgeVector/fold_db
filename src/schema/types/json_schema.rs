use crate::fees::payment_config::SchemaPaymentConfig;
use crate::fees::types::config::FieldPaymentConfig;
use crate::fees::types::config::TrustDistanceScaling;
use crate::permissions::types::policy::{ExplicitCounts, PermissionsPolicy, TrustDistance};
use crate::schema::types::field::FieldType;
use crate::schema::types::SchemaError;
use crate::schema::types::Transform;
use crate::transform::parser::TransformParser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a complete JSON schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaDefinition {
    pub name: String,
    #[serde(default = "crate::schema::types::schema::default_schema_type")]
    pub schema_type: crate::schema::types::schema::SchemaType,
    pub fields: HashMap<String, JsonSchemaField>,
    pub payment_config: SchemaPaymentConfig,
    /// SHA256 hash of the schema content for integrity verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// Represents a field in the JSON schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaField {
    pub permission_policy: JsonPermissionPolicy,
    #[serde(default)]
    pub molecule_uuid: Option<String>,
    pub payment_config: JsonFieldPaymentConfig,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonTransform {
    /// The transform logic expressed in the DSL
    pub logic: String,

    /// Explicit list of input fields in `Schema.field` format
    #[serde(default)]
    pub inputs: Vec<String>,

    /// Output field for this transform in `Schema.field` format
    pub output: String,
}

/// Represents the type of transform being applied.
///
/// Supports both procedural transforms using DSL logic and
/// placeholder declarative transforms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransformKind {
    /// Transform defined by DSL logic.
    Procedural { logic: String },
    /// Transform defined by declarative schema.
    Declarative { schema: DeclarativeSchemaDefinition },
}

/// Placeholder for declarative transform schema definition.
///
/// Will be fully implemented in DTS-1-2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DeclarativeSchemaDefinition {}

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
        let mut transform = Transform::new(json.logic, json.output);
        transform.set_inputs(json.inputs);
        transform
    }
}

fn default_field_type() -> FieldType {
    FieldType::Single
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

        // Validate each field
        for (field_name, field) in &self.fields {
            Self::validate_field(field_name, field)?;
        }

        Ok(())
    }

    fn validate_field(field_name: &str, field: &JsonSchemaField) -> Result<(), SchemaError> {
        // Validate payment config
        if field.payment_config.base_multiplier <= 0.0 {
            return Err(SchemaError::InvalidField(format!(
                "Field {field_name} base_multiplier must be positive"
            )));
        }

        // Validate trust distance scaling
        match &field.payment_config.trust_distance_scaling {
            TrustDistanceScaling::Linear { min_factor, .. }
            | TrustDistanceScaling::Exponential { min_factor, .. } => {
                if *min_factor < 1.0 {
                    return Err(SchemaError::InvalidField(format!(
                        "Field {field_name} min_factor must be >= 1.0"
                    )));
                }
            }
            TrustDistanceScaling::None => {}
        }

        if let Some(min_payment) = field.payment_config.min_payment {
            if min_payment == 0 {
                return Err(SchemaError::InvalidField(format!(
                    "Field {field_name} min_payment cannot be zero"
                )));
            }
        }

        // Validate transform if present
        if let Some(transform) = &field.transform {
            // Logic cannot be empty
            if transform.logic.is_empty() {
                return Err(SchemaError::InvalidField(format!(
                    "Field {field_name} transform logic cannot be empty"
                )));
            }

            // Parse transform logic using the DSL parser
            let parser = TransformParser::new();
            parser.parse_expression(&transform.logic).map_err(|e| {
                SchemaError::InvalidField(format!(
                    "Error parsing transform for field {field_name}: {}",
                    e
                ))
            })?;
        }

        Ok(())
    }
}

/// Key configuration for HashRange schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyConfig {
    /// Expression defining the hash key field
    pub hash_field: String,
    /// Expression defining the range key field
    pub range_field: String,
}

/// Definition of an individual field in a declarative schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Optional atom UUID mapping for the field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atom_uuid: Option<String>,
    /// Optional field type specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_type: Option<String>,
}

/// Declarative schema definition for transform generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeSchemaDefinition {
    /// Name identifying this declarative schema
    pub name: String,
    /// Type of schema this definition represents
    #[serde(default = "crate::schema::types::schema::default_schema_type")]
    pub schema_type: crate::schema::types::schema::SchemaType,
    /// Key configuration required for HashRange schemas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<KeyConfig>,
    /// Collection of field definitions
    pub fields: HashMap<String, FieldDefinition>,
}

impl DeclarativeSchemaDefinition {
    /// Validate the declarative schema definition
    pub fn validate(&self) -> Result<(), SchemaError> {
        use crate::schema::types::schema::SchemaType;

        if let SchemaType::HashRange { .. } = self.schema_type {
            let key = self.key.as_ref().ok_or_else(|| {
                SchemaError::InvalidField("HashRange schemas require key configuration".to_string())
            })?;

            if key.hash_field.is_empty() || key.range_field.is_empty() {
                return Err(SchemaError::InvalidField(
                    "KeyConfig fields cannot be empty".to_string(),
                ));
            }
        }

        for (name, field) in &self.fields {
            if field.atom_uuid.is_none() && field.field_type.is_none() {
                return Err(SchemaError::InvalidField(format!(
                    "Field '{}' must have atom_uuid or field_type",
                    name
                )));
            }
        }

        Ok(())
    }
}
