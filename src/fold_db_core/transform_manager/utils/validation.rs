use log::{info, error};
use crate::schema::types::{SchemaError, Transform};
use crate::validation_utils::ValidationUtils;

use super::TransformUtils;

impl TransformUtils {
    /// Comprehensive transform validation
    pub fn validate_transform_registration(
        transform_id: &str,
        transform: &Transform,
    ) -> Result<(), SchemaError> {
        info!("🔍 Validating transform registration for: {}", transform_id);

        if transform_id.trim().is_empty() {
            let error_msg = "Transform ID cannot be empty".to_string();
            error!("❌ {}", error_msg);
            return Err(SchemaError::InvalidData(error_msg));
        }

        let inputs = transform.get_inputs();
        if inputs.is_empty() {
            let error_msg = format!("Transform '{}' must have at least one input", transform_id);
            error!("❌ {}", error_msg);
            return Err(SchemaError::InvalidData(error_msg));
        }

        let output = transform.get_output();
        if output.trim().is_empty() {
            let error_msg = format!("Transform '{}' must have a valid output field", transform_id);
            error!("❌ {}", error_msg);
            return Err(SchemaError::InvalidData(error_msg));
        }

        if let Some(logic) = transform.get_procedural_logic() {
            if logic.trim().is_empty() {
                let error_msg = format!("Transform '{}' must have non-empty logic", transform_id);
                error!("❌ {}", error_msg);
                return Err(SchemaError::InvalidData(error_msg));
            }
        } else if let Some(schema) = transform.get_declarative_schema() {
            ValidationUtils::require_valid_schema_name(&schema.name)?;
        } else {
            let error_msg = format!("Transform '{}' must be either procedural or declarative", transform_id);
            error!("❌ {}", error_msg);
            return Err(SchemaError::InvalidData(error_msg));
        }

        info!("✅ Transform registration validation passed for: {}", transform_id);
        Ok(())
    }

    /// Validate field name format
    pub fn validate_field_name(field_name: &str, context: &str) -> Result<(), SchemaError> {
        info!("🔍 Validating field name '{}' in context: {}", field_name, context);

        if field_name.trim().is_empty() {
            let error_msg = format!("Field name cannot be empty in context: {}", context);
            error!("❌ {}", error_msg);
            return Err(SchemaError::InvalidData(error_msg));
        }

        let parts: Vec<&str> = field_name.split('.').collect();
        if parts.len() != 2 {
            let error_msg = format!(
                "Field name '{}' must be in format 'schema.field' in context: {}",
                field_name, context
            );
            error!("❌ {}", error_msg);
            return Err(SchemaError::InvalidData(error_msg));
        }

        let (schema_name, field_name_part) = (parts[0], parts[1]);
        if schema_name.trim().is_empty() || field_name_part.trim().is_empty() {
            let error_msg = format!("Schema and field names cannot be empty in field '{}' (context: {})", field_name, context);
            error!("❌ {}", error_msg);
            return Err(SchemaError::InvalidData(error_msg));
        }

        info!("✅ Field name validation passed for: {}", field_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_field_name() {
        assert!(TransformUtils::validate_field_name("Schema.field", "test").is_ok());
        assert!(TransformUtils::validate_field_name("invalid_field_name", "test").is_err());
    }
}
