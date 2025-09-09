//! Single schema executor for declarative transforms.
//!
//! This module handles execution of Single schema types, which are the simplest
//! form of declarative transforms without range semantics or complex indexing.

use crate::schema::types::{SchemaError, json_schema::DeclarativeSchemaDefinition};
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Executes a Single schema declarative transform.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `input_values` - The input values for execution
///
/// # Returns
///
/// The execution result
pub fn execute_single_schema(
    schema: &DeclarativeSchemaDefinition,
    input_values: HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    info!("🚀 Executing Single schema: {}", schema.name);
    
    // For Single schemas, we need to use the ExecutionEngine for proper field resolution
    // since field expressions can be complex (e.g., "user_data.map().user")
    // However, for simple expressions like "input.field", we can handle them directly
    execute_single_expression_with_fallback(schema, &input_values)
}

/// Executes single expression with fallback for simple field access.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `input_values` - The input values for execution
///
/// # Returns
///
/// The execution result
fn execute_single_expression_with_fallback(
    schema: &DeclarativeSchemaDefinition,
    input_values: &HashMap<String, JsonValue>,
) -> Result<JsonValue, SchemaError> {
    info!("🚀 Executing single expression with fallback for schema: {}", schema.name);
    
    let mut result_object = serde_json::Map::new();
    
    // For Single schemas, process each field with fallback logic
    for (field_name, field_def) in &schema.fields {
        let field_value = if let Some(atom_uuid_expr) = &field_def.atom_uuid {
            info!("🔗 Processing field '{}' with expression: {}", field_name, atom_uuid_expr);
            
            // Try simple field resolution first
            let field_value = if let Some(field_path) = atom_uuid_expr.strip_prefix("input.") {
                input_values.get(field_path).cloned().unwrap_or(JsonValue::Null)
            } else if !atom_uuid_expr.contains('.') && !atom_uuid_expr.contains('(') && !atom_uuid_expr.contains(')') {
                // Direct field name (no dots, no function calls) - treat as direct input field
                input_values.get(atom_uuid_expr).cloned().unwrap_or(JsonValue::Null)
            } else if atom_uuid_expr.contains('.') && !atom_uuid_expr.contains('(') && !atom_uuid_expr.contains(')') {
                // Simple field access like "user.profile.name" or "items.0" - handle directly for any depth
                let parts: Vec<&str> = atom_uuid_expr.split('.').collect();
                let mut current_value = input_values.get(parts[0]).cloned().unwrap_or(JsonValue::Null);
                
                for part in parts.iter().skip(1) {
                    // Check if this part is an array index (numeric)
                    if let Ok(index) = part.parse::<usize>() {
                        if let Some(arr) = current_value.as_array() {
                            current_value = arr.get(index).cloned().unwrap_or(JsonValue::Null);
                        } else {
                            current_value = JsonValue::Null;
                            break;
                        }
                    } else {
                        // Object field access
                        if let Some(obj) = current_value.as_object() {
                            current_value = obj.get(*part).cloned().unwrap_or(JsonValue::Null);
                        } else {
                            current_value = JsonValue::Null;
                            break;
                        }
                    }
                }
                
                current_value
            } else {
                // For more complex expressions, try ExecutionEngine
                info!("⚠️ Complex expression '{}' requires ExecutionEngine, attempting execution", atom_uuid_expr);
                match execute_with_engine_fallback(schema, input_values, field_name, atom_uuid_expr) {
                    Ok(value) => value,
                    Err(err) => {
                        info!("⚠️ ExecutionEngine failed for '{}': {}, returning null", atom_uuid_expr, err);
                        JsonValue::Null
                    }
                }
            };
            
            field_value
        } else {
            // Field has no atom_uuid - provide default value based on field type
            info!("🔗 Processing field '{}' with no expression, providing default value", field_name);
            match field_def.field_type.as_deref() {
                Some("String") => JsonValue::String("".to_string()),
                Some("Number") => JsonValue::Number(serde_json::Number::from(0)),
                Some("Boolean") => JsonValue::Bool(false),
                Some("Array") => JsonValue::Array(vec![]),
                Some("Object") => JsonValue::Object(serde_json::Map::new()),
                _ => JsonValue::Null,
            }
        };
        
        result_object.insert(field_name.clone(), field_value);
    }
    
    let result = JsonValue::Object(result_object);
    info!("✨ Single expression execution completed: {}", result);
    Ok(result)
}

/// Attempts to execute a complex expression using ExecutionEngine as fallback.
///
/// # Arguments
///
/// * `schema` - The declarative schema definition
/// * `input_values` - The input values for execution
/// * `field_name` - The field name being processed
/// * `atom_uuid_expr` - The expression to execute
///
/// # Returns
///
/// The execution result or error
fn execute_with_engine_fallback(
    _schema: &DeclarativeSchemaDefinition,
    input_values: &HashMap<String, JsonValue>,
    field_name: &str,
    atom_uuid_expr: &str,
) -> Result<JsonValue, SchemaError> {
    // Parse the expression using ChainParser
    let parsed_chain = parse_atom_uuid_expression(atom_uuid_expr)
        .map_err(|err| SchemaError::InvalidField(format!("Failed to parse expression '{}' for field '{}': {}", atom_uuid_expr, field_name, err)))?;
    
    // Validate field alignment
    let validator = crate::transform::iterator_stack::field_alignment::FieldAlignmentValidator::new();
    let alignment_result = validator.validate_alignment(&[parsed_chain.clone()])
        .map_err(|err| SchemaError::InvalidField(format!("Alignment validation failed: {}", err)))?;
    
    if !alignment_result.valid {
        let error_messages: Vec<String> = alignment_result.errors.iter()
            .map(|err| format!("{:?}: {}", err.error_type, err.message))
            .collect();
        return Err(SchemaError::InvalidField(format!(
            "Field alignment validation failed: {}", 
            error_messages.join("; ")
        )));
    }
    
    // Execute with ExecutionEngine
    let input_data = JsonValue::Object(input_values.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let mut execution_engine = crate::transform::iterator_stack::execution_engine::ExecutionEngine::new();
    
    let execution_result = execution_engine.execute_fields(
        &[parsed_chain],
        &alignment_result,
        input_data,
    ).map_err(|err| SchemaError::InvalidField(format!("ExecutionEngine failed: {}", err)))?;
    
    // Extract the field value from the first entry
    if let Some(entry) = execution_result.index_entries.first() {
        Ok(entry.hash_value.clone())
    } else {
        Ok(JsonValue::Null)
    }
}

/// Parses an atom UUID expression using ChainParser.
///
/// # Arguments
///
/// * `expression` - The expression to parse
///
/// # Returns
///
/// The parsed chain or error
fn parse_atom_uuid_expression(expression: &str) -> Result<crate::transform::iterator_stack::chain_parser::ParsedChain, SchemaError> {
    let parser = crate::transform::iterator_stack::chain_parser::ChainParser::new();
    parser.parse(expression).map_err(|err| {
        SchemaError::InvalidField(format!("Failed to parse expression '{}': {}", expression, err))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::types::json_schema::DeclarativeSchemaDefinition;
    use crate::schema::types::schema::SchemaType;
    use serde_json::json;

    #[test]
    fn test_execute_single_schema_simple() {
        // Create a simple Single schema for testing
        let mut fields = std::collections::HashMap::new();
        fields.insert("title".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.title".to_string()),
        });
        fields.insert("count".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("number".to_string()),
            atom_uuid: Some("input.count".to_string()),
        });
        
        let schema = DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields,
            key: None,
        };
        
        let input_values = HashMap::from([
            ("title".to_string(), json!("Hello World")),
            ("count".to_string(), json!(42)),
        ]);
        
        let result = execute_single_schema(&schema, input_values);
        
        match result {
            Ok(json_result) => {
                assert!(json_result.is_object());
                let obj = json_result.as_object().unwrap();
                assert_eq!(obj.get("title").unwrap(), "Hello World");
                assert_eq!(obj.get("count").unwrap(), 42);
            }
            Err(err) => {
                panic!("Single schema execution failed: {}", err);
            }
        }
    }

    #[test]
    fn test_execute_single_schema_missing_field() {
        // Test handling of missing input fields
        let mut fields = std::collections::HashMap::new();
        fields.insert("title".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.title".to_string()),
        });
        fields.insert("missing".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.missing".to_string()),
        });
        
        let schema = DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields,
            key: None,
        };
        
        let input_values = HashMap::from([
            ("title".to_string(), json!("Hello World")),
        ]);
        
        let result = execute_single_schema(&schema, input_values);
        
        match result {
            Ok(json_result) => {
                assert!(json_result.is_object());
                let obj = json_result.as_object().unwrap();
                assert_eq!(obj.get("title").unwrap(), "Hello World");
                assert_eq!(*obj.get("missing").unwrap(), json!(null));
            }
            Err(err) => {
                panic!("Single schema execution failed: {}", err);
            }
        }
    }

    #[test]
    fn test_execute_single_schema_complex_expression() {
        // Test handling of complex expressions (should return null)
        let mut fields = std::collections::HashMap::new();
        fields.insert("simple".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.simple".to_string()),
        });
        fields.insert("complex".to_string(), crate::schema::types::json_schema::FieldDefinition {
            field_type: Some("string".to_string()),
            atom_uuid: Some("input.data.map().filter()".to_string()),
        });
        
        let schema = DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields,
            key: None,
        };
        
        let input_values = HashMap::from([
            ("simple".to_string(), json!("Hello World")),
        ]);
        
        let result = execute_single_schema(&schema, input_values);
        
        match result {
            Ok(json_result) => {
                assert!(json_result.is_object());
                let obj = json_result.as_object().unwrap();
                assert_eq!(obj.get("simple").unwrap(), "Hello World");
                assert_eq!(*obj.get("complex").unwrap(), json!(null));
            }
            Err(err) => {
                panic!("Single schema execution failed: {}", err);
            }
        }
    }
}
