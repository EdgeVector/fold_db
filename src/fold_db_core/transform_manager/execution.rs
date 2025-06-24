use super::manager::TransformManager;
use crate::fold_db_core::transform_manager::utils::*;
use crate::transform::executor::TransformExecutor;
use crate::schema::types::{Schema, SchemaError};
use crate::schema::types::field::common::Field;
use log::info;
use std::sync::Arc;
use std::collections::HashMap;
use serde_json::Value as JsonValue;

impl TransformManager {

    /// Execute a single transform with input fetching and computation
    pub fn execute_single_transform(_transform_id: &str, transform: &crate::schema::types::Transform, db_ops: &Arc<crate::db_operations::DbOperations>) -> Result<JsonValue, SchemaError> {
        let mut input_values = HashMap::new();
        let inputs_to_process = if transform.get_inputs().is_empty() { transform.analyze_dependencies().into_iter().collect::<Vec<_>>() } else { transform.get_inputs().to_vec() };
        for input_field in inputs_to_process {
            if let Some(dot_pos) = input_field.find('.') {
                let input_schema = &input_field[..dot_pos];
                let input_field_name = &input_field[dot_pos + 1..];
                let value = Self::fetch_field_value(db_ops, input_schema, input_field_name).unwrap_or_else(|_| DefaultValueHelper::get_default_value_for_field(input_field_name));
                input_values.insert(input_field.clone(), value);
            } else {
                input_values.insert(input_field.clone(), DefaultValueHelper::get_default_value_for_field(&input_field));
            }
        }
        TransformExecutor::execute_transform(transform, input_values)
    }
    
    /// Fetch field value from a specific schema
    fn fetch_field_value(db_ops: &Arc<crate::db_operations::DbOperations>, schema_name: &str, field_name: &str) -> Result<JsonValue, SchemaError> {
        let schema = db_ops.get_schema(schema_name)?.ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", schema_name)))?;
        Self::get_field_value_from_schema(db_ops, &schema, field_name)
    }
    
    
    
    /// Generic result storage for any transform
    pub fn store_transform_result_generic(db_ops: &Arc<crate::db_operations::DbOperations>, transform: &crate::schema::types::Transform, result: &JsonValue) -> Result<(), SchemaError> {
        if let Some(dot_pos) = transform.get_output().find('.') {
            let schema_name = &transform.get_output()[..dot_pos];
            let field_name = &transform.get_output()[dot_pos + 1..];
            let atom = db_ops.create_atom(schema_name, "transform_system".to_string(), None, result.clone(), None)?;
            Self::update_field_reference(db_ops, schema_name, field_name, atom.uuid())
        } else {
            Err(SchemaError::InvalidField(format!("Invalid output field format '{}', expected 'Schema.field'", transform.get_output())))
        }
    }
    
    /// Update a field's ref_atom_uuid to point to a new atom and create proper linking
    /// SCHEMA-003: Only updates field values, NOT schema structure (schemas are immutable)
    fn update_field_reference(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema_name: &str,
        field_name: &str,
        atom_uuid: &str,
    ) -> Result<(), SchemaError> {
        info!("🔗 Updating field reference: {}.{} -> atom {}", schema_name, field_name, atom_uuid);
        
        // 1. Load the schema (read-only - we will NOT modify it)
        let schema = db_ops.get_schema(schema_name)?
            .ok_or_else(|| SchemaError::InvalidData(format!("Schema '{}' not found", schema_name)))?;
        
        // 2. Get the field (read-only)
        let field = schema.fields.get(field_name)
            .ok_or_else(|| SchemaError::InvalidField(format!("Field '{}' not found in schema '{}'", field_name, schema_name)))?;
        
        // 3. Get the field's ref_atom_uuid (should already exist in schema)
        let ref_uuid = field.ref_atom_uuid()
            .ok_or_else(|| SchemaError::InvalidField(format!("Field '{}.{}' has no ref_atom_uuid - schema may be malformed", schema_name, field_name)))?;
        
        // 4. Create/update AtomRef to point to the new atom (this is a field VALUE update, not schema structure)
        let atom_ref = crate::atom::AtomRef::new(atom_uuid.to_string(), "transform_system".to_string());
        db_ops.store_item(&format!("ref:{}", ref_uuid), &atom_ref)?;
        
        info!("✅ Updated field value reference for '{}.{}' to point to atom {}", schema_name, field_name, atom_uuid);
        LoggingHelper::log_atom_ref_operation(ref_uuid, atom_uuid, "creation");
        
        // SCHEMA-003: Do NOT modify schema structure - only update field value through AtomRef
        // The schema remains immutable, we only updated what the field's reference points to
        
        Ok(())
    }

    /// Get field value from a schema using database operations (consolidated implementation)
    fn get_field_value_from_schema(
        db_ops: &Arc<crate::db_operations::DbOperations>,
        schema: &Schema,
        field_name: &str,
    ) -> Result<JsonValue, SchemaError> {
        // Use the unified FieldValueResolver instead of duplicate implementation
        crate::fold_db_core::transform_manager::utils::TransformUtils::resolve_field_value(db_ops, schema, field_name, None)
    }
}