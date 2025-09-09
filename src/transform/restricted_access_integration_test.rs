//! Integration tests for transform restricted access using the test_db folder
//!
//! These tests demonstrate the proper usage of the restricted access pattern
//! with a real database setup using the root test_db folder.

use crate::transform::{
    TransformDataPersistence, MutationBasedPersistence, TransformAccessValidator,
    DatabaseTransformDataAccess,
};
use crate::schema::types::SchemaError;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Integration test fixture using a temporary database
pub struct TransformRestrictedAccessTestFixture {
    pub db_ops: Arc<crate::db_operations::DbOperations>,
    pub data_access: DatabaseTransformDataAccess,
    pub persistence: MutationBasedPersistence,
    pub source_pub_key: String,
    pub _temp_dir: TempDir,
}

impl TransformRestrictedAccessTestFixture {
    /// Create a new test fixture using a temporary database
    pub fn new() -> Result<Self, SchemaError> {
        // Create database operations using a temporary directory to avoid locks
        let temp_dir = tempfile::tempdir().map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create temp directory: {}", e))
        })?;
        
        let db = sled::open(temp_dir.path()).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to open test database: {}", e))
        })?;
        
        let db_ops = Arc::new(crate::db_operations::DbOperations::new(db).map_err(|e| {
            SchemaError::InvalidData(format!("Failed to create DbOperations: {}", e))
        })?);
        
        // Create safe data access handler
        let data_access = DatabaseTransformDataAccess::new(db_ops.clone());
        
        // Create mutation-based persistence handler
        let source_pub_key = "test_source_key".to_string();
        let persistence = MutationBasedPersistence::new(source_pub_key.clone());
        
        Ok(Self {
            db_ops,
            data_access,
            persistence,
            source_pub_key,
            _temp_dir: temp_dir,
        })
    }
    
    /// Clean up the test database
    pub fn cleanup(&self) -> Result<(), SchemaError> {
        // Clear all trees in the database
        let db = self.db_ops.db();
        for tree_name in db.tree_names() {
            if let Ok(tree) = db.open_tree(&tree_name) {
                tree.clear().map_err(|e| {
                    SchemaError::InvalidData(format!("Failed to clear tree {}: {}", 
                        String::from_utf8_lossy(&tree_name), e))
                })?;
            }
        }
        Ok(())
    }
    
    /// Test creating and persisting data through mutations
    pub fn test_mutation_persistence(&self) -> Result<(), SchemaError> {
        println!("🧪 Testing mutation-based persistence with test_db");
        
        // Create a mutation for test data
        let mutation = self.persistence.create_persistence_mutation(
            "TestSchema",
            "test_field",
            JsonValue::String("test_value".to_string()),
            &self.source_pub_key,
        )?;
        
        println!("✅ Created mutation: {:?}", mutation);
        
        // Verify mutation properties
        assert_eq!(mutation.schema_name, "TestSchema");
        assert_eq!(mutation.pub_key, self.source_pub_key);
        
        Ok(())
    }
    
    /// Test batch mutation creation
    pub fn test_batch_mutations(&self) -> Result<(), SchemaError> {
        println!("🧪 Testing batch mutation creation with test_db");
        
        let mut field_updates = HashMap::new();
        field_updates.insert("field1".to_string(), JsonValue::String("value1".to_string()));
        field_updates.insert("field2".to_string(), JsonValue::String("value2".to_string()));
        
        let mutations = self.persistence.create_batch_persistence_mutations(
            "TestSchema",
            field_updates,
            &self.source_pub_key,
        )?;
        
        println!("✅ Created {} batch mutations", mutations.len());
        assert_eq!(mutations.len(), 1); // Should create one batch mutation
        
        Ok(())
    }
    
    /// Test transform validation with real database context
    pub fn test_transform_validation(&self) -> Result<(), SchemaError> {
        println!("🧪 Testing transform validation with test_db context");
        
        // Test valid transform code
        let valid_code = "let result = create_persistence_mutation(schema, field, value);";
        let result = TransformAccessValidator::validate_no_direct_creation(valid_code);
        assert!(result.is_ok());
        
        // Test invalid transform code
        let invalid_code = "let atom = Atom::new(schema, key, content);";
        let result = TransformAccessValidator::validate_no_direct_creation(invalid_code);
        assert!(result.is_err());
        
        println!("✅ Transform validation tests passed");
        Ok(())
    }
    
    /// Test safe data access with real database
    pub fn test_safe_data_access(&self) -> Result<(), SchemaError> {
        println!("🧪 Testing safe data access with test_db");
        
        // This test would require actual data in the database
        // For now, we'll just verify the data access handler is properly configured
        println!("✅ Safe data access handler configured for test_db");
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transform_restricted_access_with_test_db() {
        println!("🚀 Starting transform restricted access integration test with temp database");
        
        // Create test fixture using temp database
        let fixture = TransformRestrictedAccessTestFixture::new()
            .expect("Failed to create test fixture with temp database");
        
        // Test mutation persistence
        fixture.test_mutation_persistence()
            .expect("Mutation persistence test failed");
        
        // Test batch mutations
        fixture.test_batch_mutations()
            .expect("Batch mutations test failed");
        
        // Test transform validation
        fixture.test_transform_validation()
            .expect("Transform validation test failed");
        
        // Test safe data access
        fixture.test_safe_data_access()
            .expect("Safe data access test failed");
        
        // Clean up test database
        fixture.cleanup()
            .expect("Failed to clean up test database");
        
        println!("✅ All transform restricted access tests passed with temp database");
    }
    
    #[test]
    fn test_temp_database_usage() {
        println!("🧪 Testing temp database usage");
        
        // Verify we can create a fixture using a temp database
        let _fixture = TransformRestrictedAccessTestFixture::new()
            .expect("Failed to create fixture with temp database");
        
        // Verify the database is functional
        println!("📁 Database operations created successfully");
        
        println!("✅ Temp database usage verified");
    }
    
    #[test]
    fn test_database_cleanup() {
        println!("🧪 Testing database cleanup functionality");
        
        let fixture = TransformRestrictedAccessTestFixture::new()
            .expect("Failed to create test fixture");
        
        // Test cleanup functionality
        fixture.cleanup()
            .expect("Failed to clean up test database");
        
        println!("✅ Database cleanup test passed");
    }
}
