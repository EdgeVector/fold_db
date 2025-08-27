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
#[derive(Debug, Clone, Serialize)]
pub struct JsonTransform {
    /// The transform type and configuration
    #[serde(flatten)]
    pub kind: TransformKind,

    /// Explicit list of input fields in `Schema.field` format
    #[serde(default)]
    pub inputs: Vec<String>,

    /// Output field for this transform in `Schema.field` format
    pub output: String,
}

// Custom deserialization to maintain backward compatibility with existing procedural transforms
impl<'de> serde::Deserialize<'de> for JsonTransform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Helper {
            // New format with explicit kind
            NewFormat {
                #[serde(flatten)]
                kind: TransformKind,
                #[serde(default)]
                inputs: Vec<String>,
                output: String,
            },
            // Legacy format with logic field (backward compatibility)
            LegacyFormat {
                logic: String,
                #[serde(default)]
                inputs: Vec<String>,
                output: String,
            },
        }

        match Helper::deserialize(deserializer)? {
            Helper::NewFormat { kind, inputs, output } => Ok(JsonTransform {
                kind,
                inputs,
                output,
            }),
            Helper::LegacyFormat { logic, inputs, output } => Ok(JsonTransform {
                kind: TransformKind::Procedural { logic },
                inputs,
                output,
            }),
        }
    }
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

impl TransformKind {
    /// Validates the transform kind based on its variant.
    pub fn validate(&self) -> Result<(), SchemaError> {
        match self {
            TransformKind::Procedural { logic } => {
                self.validate_procedural_logic(logic)
            }
            TransformKind::Declarative { schema } => {
                schema.validate()
            }
        }
    }

    /// Validates procedural transform logic
    fn validate_procedural_logic(&self, logic: &str) -> Result<(), SchemaError> {
        use crate::validation_utils::ValidationUtils;

        ValidationUtils::require_non_empty_string(logic, "Procedural transform logic")?;

        // Basic syntax validation for procedural logic
        let trimmed_logic = logic.trim();
        
        // Check for reasonable length
        if trimmed_logic.len() > 10000 {
            return Err(SchemaError::InvalidField(
                "Procedural transform logic is too long (max 10000 characters)".to_string()
            ));
        }

        // Check for obviously malformed logic
        if trimmed_logic.chars().filter(|&c| c == '{').count() != trimmed_logic.chars().filter(|&c| c == '}').count() {
            return Err(SchemaError::InvalidField(
                "Procedural transform logic has mismatched braces".to_string()
            ));
        }

        if trimmed_logic.chars().filter(|&c| c == '(').count() != trimmed_logic.chars().filter(|&c| c == ')').count() {
            return Err(SchemaError::InvalidField(
                "Procedural transform logic has mismatched parentheses".to_string()
            ));
        }

        Ok(())
    }
}

impl JsonTransform {
    /// Validates the complete JSON transform structure.
    pub fn validate(&self) -> Result<(), SchemaError> {
        use crate::validation_utils::ValidationUtils;

        // Validate output field
        ValidationUtils::require_non_empty_string(&self.output, "Transform output field")?;

        // Validate output field format (should be schema.field)
        if !self.output.contains('.') {
            return Err(SchemaError::InvalidField(
                "Transform output field must be in format 'schema.field'".to_string()
            ));
        }

        // Validate input fields
        for (i, input) in self.inputs.iter().enumerate() {
            ValidationUtils::require_non_empty_string(input, &format!("Transform input field {}", i))?;
            
            if !input.contains('.') {
                return Err(SchemaError::InvalidField(format!(
                    "Transform input field '{}' must be in format 'schema.field'", input
                )));
            }
        }

        // Validate the transform kind
        self.kind.validate()?;

        Ok(())
    }
}

/// Configuration for hash and range key expressions in HashRange schemas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyConfig {
    /// Hash field expression for the key
    pub hash_field: String,
    /// Range field expression for the key
    pub range_field: String,
}

impl KeyConfig {
    /// Validates the key configuration for HashRange schemas.
    pub fn validate(&self) -> Result<(), SchemaError> {
        use crate::validation_utils::ValidationUtils;

        // Validate hash field is not empty or whitespace-only
        if self.hash_field.trim().is_empty() {
            return Err(SchemaError::InvalidField(
                "HashRange hash_field cannot be empty".to_string()
            ));
        }

        // Validate range field is not empty or whitespace-only
        if self.range_field.trim().is_empty() {
            return Err(SchemaError::InvalidField(
                "HashRange range_field cannot be empty".to_string()
            ));
        }

        // Validate field expressions have valid syntax (basic check)
        self.validate_field_expression(&self.hash_field, "hash_field")?;
        self.validate_field_expression(&self.range_field, "range_field")?;

        // Ensure hash and range fields are different
        if self.hash_field.trim() == self.range_field.trim() {
            return Err(SchemaError::InvalidField(
                "HashRange hash_field and range_field must be different".to_string()
            ));
        }

        Ok(())
    }

    /// Validates basic field expression syntax
    fn validate_field_expression(&self, expression: &str, field_name: &str) -> Result<(), SchemaError> {
        let expr = expression.trim();
        
        // Basic validation - must not start or end with dots
        if expr.starts_with('.') || expr.ends_with('.') {
            return Err(SchemaError::InvalidField(format!(
                "Field expression '{}' in {} cannot start or end with a dot", 
                expr, field_name
            )));
        }

        // Must not contain double dots
        if expr.contains("..") {
            return Err(SchemaError::InvalidField(format!(
                "Field expression '{}' in {} cannot contain consecutive dots", 
                expr, field_name
            )));
        }

        // Must contain at least one character that isn't a dot
        if expr.chars().all(|c| c == '.' || c.is_whitespace()) {
            return Err(SchemaError::InvalidField(format!(
                "Field expression '{}' in {} must contain valid field references", 
                expr, field_name
            )));
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
            return Err(SchemaError::InvalidField(format!(
                "Field '{}' must have at least one property defined (atom_uuid or field_type)",
                field_name
            )));
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
    fn validate_atom_uuid_expression(&self, atom_uuid: &str, field_name: &str) -> Result<(), SchemaError> {
        let expr = atom_uuid.trim();
        
        if expr.is_empty() {
            return Err(SchemaError::InvalidField(format!(
                "Field '{}' atom_uuid cannot be empty", field_name
            )));
        }

        // Basic expression validation
        if expr.starts_with('.') || expr.ends_with('.') {
            return Err(SchemaError::InvalidField(format!(
                "Field '{}' atom_uuid expression '{}' cannot start or end with a dot", 
                field_name, expr
            )));
        }

        if expr.contains("..") {
            return Err(SchemaError::InvalidField(format!(
                "Field '{}' atom_uuid expression '{}' cannot contain consecutive dots", 
                field_name, expr
            )));
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
            return Err(SchemaError::InvalidField(format!(
                "Field '{}' field_type cannot be empty", field_name
            )));
        }

        // Basic type validation - ensure it's a reasonable type name
        if field_type.len() > 100 {
            return Err(SchemaError::InvalidField(format!(
                "Field '{}' field_type '{}' is too long (max 100 characters)", 
                field_name, field_type
            )));
        }

        // Ensure type doesn't contain invalid characters (check the original string, not trimmed)
        if field_type.chars().any(|c| c.is_control() || c == '\n' || c == '\r') {
            return Err(SchemaError::InvalidField(format!(
                "Field '{}' field_type '{}' contains invalid characters", 
                field_name, field_type
            )));
        }

        Ok(())
    }
}

/// Declarative schema definition used by declarative transforms.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeclarativeSchemaDefinition {
    /// Schema name (same as transform name)
    pub name: String,
    /// Schema type ("Single" | "HashRange")
    pub schema_type: crate::schema::types::schema::SchemaType,
    /// Key configuration (required when schema_type == "HashRange")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<KeyConfig>,
    /// Field definitions with their mapping expressions
    pub fields: HashMap<String, FieldDefinition>,
}

impl DeclarativeSchemaDefinition {
    /// Validates the declarative schema definition with comprehensive checks.
    pub fn validate(&self) -> Result<(), SchemaError> {
        use crate::validation_utils::ValidationUtils;

        // Validate required fields
        ValidationUtils::require_non_empty_string(&self.name, "Schema name")?;
        
        // Validate fields map is not empty
        if self.fields.is_empty() {
            return Err(SchemaError::InvalidField(
                "Schema must have at least one field defined".to_string()
            ));
        }

        // Schema type specific validation
        match &self.schema_type {
            crate::schema::types::schema::SchemaType::HashRange => {
                self.validate_hashrange_requirements()?;
            }
            crate::schema::types::schema::SchemaType::Single => {
                self.validate_single_requirements()?;
            }
            crate::schema::types::schema::SchemaType::Range { range_key } => {
                self.validate_range_requirements(range_key)?;
            }
        }

        // Validate all field definitions
        for (name, field) in &self.fields {
            field.validate(name)?;
        }

        Ok(())
    }

    /// Validates HashRange schema specific requirements
    fn validate_hashrange_requirements(&self) -> Result<(), SchemaError> {
        let key = self.key.as_ref().ok_or_else(|| {
            SchemaError::InvalidField("HashRange schema requires key configuration".to_string())
        })?;

        key.validate()?;

        Ok(())
    }

    /// Validates Single schema specific requirements
    fn validate_single_requirements(&self) -> Result<(), SchemaError> {
        if self.key.is_some() {
            return Err(SchemaError::InvalidField(
                "Single schema should not have key configuration".to_string()
            ));
        }

        Ok(())
    }

    /// Validates Range schema specific requirements
    fn validate_range_requirements(&self, range_key: &str) -> Result<(), SchemaError> {
        use crate::validation_utils::ValidationUtils;

        ValidationUtils::require_non_empty_string(range_key, "Range schema range_key")?;

        if self.key.is_some() {
            return Err(SchemaError::InvalidField(
                "Range schema should not have key configuration (range_key is specified in schema_type)".to_string()
            ));
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
        match json.kind {
            TransformKind::Procedural { logic } => {
                let mut transform = Transform::new(logic, json.output);
                transform.set_inputs(json.inputs);
                transform
            }
            TransformKind::Declarative { schema } => {
                Transform::from_declarative_schema(schema, json.inputs, json.output)
            }
        }
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
            // Validate transform based on its kind
            match &transform.kind {
                TransformKind::Procedural { logic } => {
                    // Logic cannot be empty for procedural transforms
                    if logic.is_empty() {
                        return Err(SchemaError::InvalidField(format!(
                            "Field {field_name} transform logic cannot be empty"
                        )));
                    }

                    // Parse transform logic using the DSL parser
                    let parser = TransformParser::new();
                    parser.parse_expression(logic).map_err(|e| {
                        SchemaError::InvalidField(format!(
                            "Error parsing transform for field {field_name}: {}",
                            e
                        ))
                    })?;
                }
                TransformKind::Declarative { schema } => {
                    // Validate declarative schema
                    schema.validate().map_err(|e| {
                        SchemaError::InvalidField(format!(
                            "Error validating declarative transform for field {field_name}: {}",
                            e
                        ))
                    })?;
                }
            }
        }

        Ok(())
    }
}
