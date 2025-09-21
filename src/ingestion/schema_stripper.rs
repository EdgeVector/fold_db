//! Schema stripping service for removing payment and permission data

use crate::ingestion::IngestionResult;
use crate::schema::Schema;
use serde_json::{Map, Value};

/// Service for stripping payment and permission data from schemas
pub struct SchemaStripper;

impl SchemaStripper {
    /// Create a new schema stripper
    pub fn new() -> Self {
        Self
    }

    /// Strip payment and permission data from a single schema
    pub fn strip_schema(&self, schema: &Schema) -> IngestionResult<Value> {
        // Convert schema to JSON
        let mut schema_json = serde_json::to_value(schema)?;

        // Remove payment_config at the root level
        if let Value::Object(ref mut obj) = schema_json {
            obj.remove("payment_config");

            // Remove payment and permission data from fields
            if let Some(Value::Object(fields)) = obj.get_mut("fields") {
                for (_, field_value) in fields.iter_mut() {
                    self.strip_field_data(field_value)?;
                }
            }
        }

        Ok(schema_json)
    }

    /// Strip payment and permission data from multiple schemas
    pub fn strip_schemas(&self, schemas: &[Schema]) -> IngestionResult<Vec<Value>> {
        schemas
            .iter()
            .map(|schema| self.strip_schema(schema))
            .collect()
    }

    /// Strip payment and permission data from a field
    #[allow(clippy::only_used_in_recursion)]
    fn strip_field_data(&self, field_value: &mut Value) -> IngestionResult<()> {
        if let Value::Object(ref mut field_obj) = field_value {
            // Remove payment_config
            field_obj.remove("payment_config");

            // Remove permission_policy
            field_obj.remove("permission_policy");

            // For range fields, strip nested field data
            if let Some(Value::Object(range_field)) = field_obj.get_mut("field") {
                self.strip_field_data(&mut Value::Object(range_field.clone()))?;
            }
        }

        Ok(())
    }

    /// Create a clean schema representation for AI analysis
    pub fn create_ai_schema_representation(&self, schemas: &[Schema]) -> IngestionResult<Value> {
        let stripped_schemas = self.strip_schemas(schemas)?;

        // Create a structured representation for the AI
        let mut ai_schemas = Map::new();

        for schema_value in stripped_schemas {
            if let Value::Object(schema_obj) = schema_value {
                if let Some(Value::String(name)) = schema_obj.get("name") {
                    // Create a simplified schema representation
                    let mut simplified = Map::new();

                    // Add basic schema info
                    simplified.insert("name".to_string(), Value::String(name.clone()));

                    // Add field structure
                    if let Some(fields) = schema_obj.get("fields") {
                        simplified.insert("fields".to_string(), self.simplify_fields(fields)?);
                    }

                    ai_schemas.insert(name.clone(), Value::Object(simplified));
                }
            }
        }

        Ok(Value::Object(ai_schemas))
    }

    /// Simplify field structure for AI analysis
    fn simplify_fields(&self, fields: &Value) -> IngestionResult<Value> {
        if let Value::Object(fields_obj) = fields {
            let mut simplified_fields = Map::new();

            for (field_name, field_value) in fields_obj {
                simplified_fields.insert(field_name.clone(), self.simplify_field(field_value)?);
            }

            Ok(Value::Object(simplified_fields))
        } else {
            Ok(fields.clone())
        }
    }

    /// Simplify a single field for AI analysis
    #[allow(clippy::only_used_in_recursion)]
    fn simplify_field(&self, field: &Value) -> IngestionResult<Value> {
        if let Value::Object(field_obj) = field {
            let mut simplified = Map::new();

            // Keep essential field information
            if let Some(field_type) = field_obj.get("field_type") {
                simplified.insert("type".to_string(), field_type.clone());
            }

            // For range fields, include range structure
            if let Some(range_field) = field_obj.get("field") {
                simplified.insert("field".to_string(), self.simplify_field(range_field)?);
            }

            // Include transform information if present
            if let Some(transform) = field_obj.get("transform") {
                simplified.insert("transform".to_string(), transform.clone());
            }

            Ok(Value::Object(simplified))
        } else {
            Ok(field.clone())
        }
    }
}

impl Default for SchemaStripper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fees::types::FieldPaymentConfig;
    use crate::permissions::types::policy::PermissionsPolicy;
    use crate::schema::types::field::{FieldVariant, SingleField};
    use std::collections::HashMap;

    fn create_test_schema() -> Schema {
        let mut schema = Schema::new("test_schema".to_string());

        // Create a simple field with payment and permission data
        let field = FieldVariant::Single(SingleField::new(
            PermissionsPolicy::default(),
            FieldPaymentConfig::default(),
            HashMap::new(),
        ));

        schema.add_field("test_field".to_string(), field);
        schema
    }

    #[test]
    fn test_strip_schema() {
        let stripper = SchemaStripper::new();
        let schema = create_test_schema();

        let result = stripper.strip_schema(&schema);
        assert!(result.is_ok());

        let stripped = result.unwrap();
        if let Value::Object(obj) = stripped {
            // Payment config should be removed
            assert!(!obj.contains_key("payment_config"));

            // Fields should still exist
            assert!(obj.contains_key("fields"));
        }
    }

    #[test]
    fn test_create_ai_schema_representation() {
        let stripper = SchemaStripper::new();
        let schemas = vec![create_test_schema()];

        let result = stripper.create_ai_schema_representation(&schemas);
        assert!(result.is_ok());

        let ai_repr = result.unwrap();
        if let Value::Object(obj) = ai_repr {
            assert!(obj.contains_key("test_schema"));
        }
    }
}
