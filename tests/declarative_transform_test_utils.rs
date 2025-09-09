//! Shared test utilities for declarative transform tests.
//!
//! This module provides common test fixtures and utilities to eliminate
//! code duplication across declarative transform test files.

use datafold::schema::types::{
    json_schema::{DeclarativeSchemaDefinition, FieldDefinition, KeyConfig},
    Transform, TransformRegistration,
};
use datafold::schema::SchemaType;
use datafold::schema::core::SchemaCore;
use datafold::fold_db_core::transform_manager::manager::TransformManager;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::db_operations::DbOperations;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use sled;

/// Test fixture for declarative transform unit tests
pub struct DeclarativeTransformTestFixture {
    pub schema_core: SchemaCore,
    pub temp_dir: TempDir,
}

impl DeclarativeTransformTestFixture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let db = sled::open(temp_dir.path())?;
        let db_ops = std::sync::Arc::new(datafold::db_operations::DbOperations::new(db)?);
        let message_bus = std::sync::Arc::new(datafold::fold_db_core::infrastructure::message_bus::MessageBus::new());
        let schema_core = SchemaCore::new(
            temp_dir.path().to_str().unwrap(),
            db_ops.clone(),
            message_bus,
        )?;
        
        Ok(Self {
            schema_core,
            temp_dir,
        })
    }
}

/// Test fixture for declarative transform integration tests
pub struct DeclarativeTransformIntegrationFixture {
    pub transform_manager: TransformManager,
    pub temp_dir: TempDir,
}

impl DeclarativeTransformIntegrationFixture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let db = sled::open(temp_dir.path())?;
        let db_ops = Arc::new(DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());
        let transform_manager = TransformManager::new(db_ops, message_bus)?;
        
        Ok(Self {
            transform_manager,
            temp_dir,
        })
    }
}

/// Helper functions for creating common test schemas
pub struct TestSchemaBuilder;

impl TestSchemaBuilder {
    /// Creates a simple Single schema for testing
    pub fn create_single_schema(name: &str) -> DeclarativeSchemaDefinition {
        let mut fields = HashMap::new();
        fields.insert("field1".to_string(), FieldDefinition {
            field_type: Some("single".to_string()),
            atom_uuid: Some("input.map().value".to_string()),
        });
        
        DeclarativeSchemaDefinition {
            name: name.to_string(),
            schema_type: SchemaType::Single,
            fields,
            key: None,
        }
    }
    
    /// Creates a HashRange schema for testing
    pub fn create_hashrange_schema(name: &str) -> DeclarativeSchemaDefinition {
        let mut fields = HashMap::new();
        fields.insert("word".to_string(), FieldDefinition {
            field_type: Some("single".to_string()),
            atom_uuid: Some("source.map().content.split_by_word().map()".to_string()),
        });
        fields.insert("source_ref".to_string(), FieldDefinition {
            field_type: Some("single".to_string()),
            atom_uuid: Some("source.map().$atom_uuid".to_string()),
        });
        
        DeclarativeSchemaDefinition {
            name: name.to_string(),
            schema_type: SchemaType::HashRange,
            fields,
            key: Some(KeyConfig {
                hash_field: "source.map().content.split_by_word().map()".to_string(),
                range_field: "source.map().timestamp".to_string(),
            }),
        }
    }
    
    /// Creates a Range schema for testing
    pub fn create_range_schema(name: &str, range_key: &str) -> DeclarativeSchemaDefinition {
        let mut fields = HashMap::new();
        fields.insert(range_key.to_string(), FieldDefinition {
            field_type: Some("single".to_string()),
            atom_uuid: Some("input.map().timestamp".to_string()),
        });
        fields.insert("data".to_string(), FieldDefinition {
            field_type: Some("single".to_string()),
            atom_uuid: Some("input.map().value".to_string()),
        });
        
        DeclarativeSchemaDefinition {
            name: name.to_string(),
            schema_type: SchemaType::Range { range_key: range_key.to_string() },
            fields,
            key: None,
        }
    }
    
    /// Creates a complex schema with multiple fields for testing
    pub fn create_complex_schema(name: &str) -> DeclarativeSchemaDefinition {
        let mut fields = HashMap::new();
        fields.insert("main_content".to_string(), FieldDefinition {
            field_type: Some("single".to_string()),
            atom_uuid: Some("post.map().content".to_string()),
        });
        fields.insert("author_info".to_string(), FieldDefinition {
            field_type: Some("single".to_string()),
            atom_uuid: Some("post.map().author.name".to_string()),
        });
        fields.insert("metadata".to_string(), FieldDefinition {
            field_type: Some("single".to_string()),
            atom_uuid: Some("post.map().metadata.tags".to_string()),
        });
        
        DeclarativeSchemaDefinition {
            name: name.to_string(),
            schema_type: SchemaType::HashRange,
            fields,
            key: Some(KeyConfig {
                hash_field: "post.map().content.split_by_word().map()".to_string(),
                range_field: "post.map().created_at".to_string(),
            }),
        }
    }
}

/// Helper functions for creating test transforms
pub struct TestTransformBuilder;

impl TestTransformBuilder {
    /// Creates a transform registration for testing
    pub fn create_transform_registration(
        transform_id: &str,
        schema: DeclarativeSchemaDefinition,
        inputs: Vec<String>,
        output: &str,
    ) -> TransformRegistration {
        let transform = Transform::from_declarative_schema(schema, inputs.clone(), output.to_string());
        
        TransformRegistration {
            transform_id: transform_id.to_string(),
            transform,
            input_molecules: inputs.clone(),
            input_names: inputs.iter().map(|s| s.split('.').last().unwrap_or(s).to_string()).collect(),
            trigger_fields: inputs,
            output_molecule: output.to_string(),
            schema_name: output.split('.').next().unwrap_or("test").to_string(),
            field_name: output.split('.').last().unwrap_or("field").to_string(),
        }
    }
}

/// Common test assertions
pub struct TestAssertions;

impl TestAssertions {
    /// Asserts that a schema validation result is successful
    pub fn assert_schema_validation_success(result: Result<(), datafold::schema::types::SchemaError>, context: &str) {
        assert!(result.is_ok(), "{}: Schema validation failed: {:?}", context, result);
    }
    
    /// Asserts that a schema validation result is a failure
    pub fn assert_schema_validation_failure(result: Result<(), datafold::schema::types::SchemaError>, context: &str) {
        assert!(result.is_err(), "{}: Schema validation should have failed", context);
    }
    
    /// Asserts that a transform is declarative
    pub fn assert_transform_is_declarative(transform: &Transform, context: &str) {
        assert!(transform.is_declarative(), "{}: Transform should be declarative", context);
        assert!(!transform.is_procedural(), "{}: Transform should not be procedural", context);
    }
    
    /// Asserts that a transform is procedural
    pub fn assert_transform_is_procedural(transform: &Transform, context: &str) {
        assert!(transform.is_procedural(), "{}: Transform should be procedural", context);
        assert!(!transform.is_declarative(), "{}: Transform should not be declarative", context);
    }
    
    /// Asserts that a transform has the expected inputs
    pub fn assert_transform_inputs(transform: &Transform, expected_inputs: &[&str], context: &str) {
        let actual_inputs: Vec<&str> = transform.get_inputs().iter().map(|s| s.as_str()).collect();
        assert_eq!(actual_inputs, expected_inputs, "{}: Transform inputs mismatch", context);
    }
    
    /// Asserts that a transform has the expected output
    pub fn assert_transform_output(transform: &Transform, expected_output: &str, context: &str) {
        assert_eq!(transform.get_output(), expected_output, "{}: Transform output mismatch", context);
    }
}
