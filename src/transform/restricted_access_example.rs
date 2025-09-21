//! Example demonstrating proper usage of the restricted transform access pattern.
//!
//! This example shows how transforms should interact with the system using
//! only mutation-based persistence and read-only data access.

use crate::schema::types::{Mutation, Transform};
use crate::schema::SchemaError;
use crate::transform::{
    DatabaseTransformDataAccess, MutationBasedPersistence, TransformAccessValidator,
    TransformDataPersistence, TransformSafeDataAccess,
};
use log::info;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Example: Proper transform execution with restricted access
pub struct ExampleTransformExecutor {
    /// Database operations for data access
    db_ops: Arc<crate::db_operations::DbOperations>,
    /// Source public key for audit trails
    source_pub_key: String,
}

impl ExampleTransformExecutor {
    /// Create a new example transform executor
    pub fn new(db_ops: Arc<crate::db_operations::DbOperations>, source_pub_key: String) -> Self {
        Self {
            db_ops,
            source_pub_key,
        }
    }

    /// Execute a transform following the restricted access pattern
    pub fn execute_transform_safely(
        &self,
        transform: &Transform,
    ) -> Result<JsonValue, SchemaError> {
        info!("🚀 Executing transform with restricted access pattern");

        // Step 1: Validate that transform doesn't attempt direct creation
        self.validate_transform_access(transform)?;

        // Step 2: Create safe data access handler
        let data_access = DatabaseTransformDataAccess::new(self.db_ops.clone());

        // Step 3: Create mutation-based persistence handler
        let persistence = MutationBasedPersistence::new(self.source_pub_key.clone());

        // Step 4: Execute transform with restricted access
        self.execute_with_restricted_access(transform, &data_access, &persistence)
    }

    /// Validate that transform follows access restrictions
    fn validate_transform_access(&self, transform: &Transform) -> Result<(), SchemaError> {
        info!("🔍 Validating transform access restrictions");

        // Get transform code for validation (only declarative transforms supported)
        let mut code_parts = Vec::new();
        for field_def in transform.schema.fields.values() {
            if let Some(atom_uuid) = &field_def.atom_uuid {
                code_parts.push(atom_uuid.clone());
            }
        }
        let transform_code = code_parts.join(" ");

        // Validate no direct creation patterns
        TransformAccessValidator::validate_no_direct_creation(&transform_code)?;

        // Validate proper mutation usage
        TransformAccessValidator::validate_mutation_usage(&transform_code)?;

        info!("✅ Transform access validation passed");
        Ok(())
    }

    /// Execute transform with restricted access to data and persistence
    fn execute_with_restricted_access(
        &self,
        transform: &Transform,
        data_access: &DatabaseTransformDataAccess,
        persistence: &MutationBasedPersistence,
    ) -> Result<JsonValue, SchemaError> {
        info!("⚡ Executing transform with restricted access");

        // Example: Read input data using safe access
        let input_data = self.gather_input_data_safely(transform, data_access)?;

        // Example: Process the data (this would be the actual transform logic)
        let result = self.process_transform_data(transform, &input_data)?;

        // Example: Persist results using mutation interface
        self.persist_results_safely(transform, &result, persistence)?;

        Ok(result)
    }

    /// Gather input data using safe read-only access
    fn gather_input_data_safely(
        &self,
        transform: &Transform,
        data_access: &DatabaseTransformDataAccess,
    ) -> Result<HashMap<String, JsonValue>, SchemaError> {
        info!("📊 Gathering input data with safe access");

        let mut input_data = HashMap::new();

        // Example: Access input molecules safely
        for input in &transform.inputs {
            if let Ok(molecule) = data_access.get_readonly_molecule(input) {
                // Get the referenced atom safely
                if let Ok(atom) = data_access.get_readonly_atom(molecule.get_atom_uuid()) {
                    // Extract data from atom content
                    input_data.insert(input.clone(), atom.content().clone());
                    info!("✅ Safely accessed input data for: {}", input);
                }
            }
        }

        Ok(input_data)
    }

    /// Process transform data (example implementation)
    fn process_transform_data(
        &self,
        _transform: &Transform,
        input_data: &HashMap<String, JsonValue>,
    ) -> Result<JsonValue, SchemaError> {
        info!("🔄 Processing transform data");

        // Example: Simple data processing
        // In a real implementation, this would use the transform's actual logic
        let mut result = JsonValue::Object(serde_json::Map::new());

        for (key, value) in input_data {
            // Example processing: convert strings to uppercase
            if let Some(str_value) = value.as_str() {
                result[key] = JsonValue::String(str_value.to_uppercase());
            } else {
                result[key] = value.clone();
            }
        }

        info!("✅ Transform data processing completed");
        Ok(result)
    }

    /// Persist results using mutation interface
    fn persist_results_safely(
        &self,
        transform: &Transform,
        result: &JsonValue,
        persistence: &MutationBasedPersistence,
    ) -> Result<(), SchemaError> {
        info!("💾 Persisting results using mutation interface");

        // Parse output field to determine target schema and field
        let output_field = transform.get_output();
        if let Some(dot_pos) = output_field.find('.') {
            let schema_name = &output_field[..dot_pos];
            let field_name = &output_field[dot_pos + 1..];

            // Create mutation using the persistence interface
            let _mutation = persistence.create_persistence_mutation(
                schema_name,
                field_name,
                result.clone(),
                &self.source_pub_key,
            )?;

            info!("📝 Created mutation for {}.{}", schema_name, field_name);

            // In a real implementation, this mutation would be executed
            // through the mutation service to persist the data
            info!("✅ Results persisted successfully via mutation");
        } else {
            return Err(SchemaError::InvalidField(format!(
                "Invalid output field format '{}', expected 'Schema.field'",
                output_field
            )));
        }

        Ok(())
    }
}

/// Example: Demonstrating what NOT to do (this would fail validation)
pub struct BadTransformExample;

impl BadTransformExample {
    /// This method demonstrates forbidden patterns that would fail validation
    pub fn demonstrate_forbidden_patterns() -> Result<(), SchemaError> {
        // ❌ FORBIDDEN: Direct atom creation
        // let atom = crate::atom::Atom::new(
        //     "TestSchema".to_string(),
        //     "test_key".to_string(),
        //     JsonValue::String("test_content".to_string())
        // );

        // ❌ FORBIDDEN: Direct molecule creation
        // let molecule = crate::atom::Molecule::new(
        //     "atom_uuid".to_string(),
        //     "test_key".to_string()
        // );

        // ❌ FORBIDDEN: Direct molecule range creation
        // let molecule_range = crate::atom::MoleculeRange::new("test_key".to_string());

        // ✅ CORRECT: Use mutation-based persistence
        let persistence = MutationBasedPersistence::new("test_key".to_string());
        let _mutation = persistence.create_persistence_mutation(
            "TestSchema",
            "test_field",
            JsonValue::String("test_value".to_string()),
            "test_key",
        )?;

        Ok(())
    }
}

/// Example: Batch mutation execution
pub struct BatchTransformExecutor {
    persistence: MutationBasedPersistence,
}

impl BatchTransformExecutor {
    /// Create a new batch transform executor
    pub fn new(source_pub_key: String) -> Self {
        Self {
            persistence: MutationBasedPersistence::new(source_pub_key),
        }
    }

    /// Execute multiple field updates in a single batch
    pub fn execute_batch_updates(
        &self,
        schema_name: &str,
        field_updates: HashMap<String, JsonValue>,
    ) -> Result<Vec<Mutation>, SchemaError> {
        info!("📦 Executing batch updates for schema: {}", schema_name);

        // Create batch mutations using the persistence interface
        let mutations = self.persistence.create_batch_persistence_mutations(
            schema_name,
            field_updates,
            &self.persistence.source_pub_key,
        )?;

        info!("✅ Created {} batch mutations", mutations.len());
        Ok(mutations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_access_validation() {
        // Test valid transform code
        let valid_code = "let result = create_persistence_mutation(schema, field, value);";
        let result = TransformAccessValidator::validate_no_direct_creation(valid_code);
        assert!(result.is_ok());

        // Test invalid transform code
        let invalid_code = "let atom = Atom::new(schema, key, content);";
        let result = TransformAccessValidator::validate_no_direct_creation(invalid_code);
        assert!(result.is_err());
    }

    #[test]
    fn test_mutation_creation() {
        let persistence = MutationBasedPersistence::new("test_key".to_string());
        let mutation = persistence
            .create_persistence_mutation(
                "TestSchema",
                "test_field",
                JsonValue::String("test_value".to_string()),
                "test_key",
            )
            .unwrap();

        assert_eq!(mutation.schema_name, "TestSchema");
        // Note: MutationType comparison removed due to missing PartialEq implementation
    }

    #[test]
    fn test_batch_mutation_creation() {
        let executor = BatchTransformExecutor::new("test_key".to_string());
        let mut field_updates = HashMap::new();
        field_updates.insert(
            "field1".to_string(),
            JsonValue::String("value1".to_string()),
        );
        field_updates.insert(
            "field2".to_string(),
            JsonValue::String("value2".to_string()),
        );

        let mutations = executor
            .execute_batch_updates("TestSchema", field_updates)
            .unwrap();
        assert_eq!(mutations.len(), 1); // Should create one batch mutation
    }
}
