//! Database Regression Prevention Tests
//!
//! These tests specifically target the critical database issues that cause repeated failures:
//! 1. Schema field assignment loss after persistence/restart
//! 2. Field mapping persistence failures
//! 3. Database consistency validation

use datafold::db_operations::DbOperations;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::schema::core::SchemaCore;
use datafold::schema::types::{Schema, SingleField, FieldVariant, SchemaType, Field};
use datafold::permissions::types::policy::PermissionsPolicy;
use datafold::fees::types::config::FieldPaymentConfig;
use datafold::fees::SchemaPaymentConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Test fixture for setting up consistent test environments
struct DatabaseTestFixture {
    #[allow(dead_code)]
    temp_dir: TempDir,
    db_ops: Arc<DbOperations>,
    schema_core: SchemaCore,
}

impl DatabaseTestFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test_db");
        let db = sled::open(&db_path)?;
        let db_ops = Arc::new(DbOperations::new(db)?);
        let message_bus = Arc::new(MessageBus::new());
        let schema_core = SchemaCore::new(
            temp_dir.path().to_str().unwrap(),
            Arc::clone(&db_ops),
            Arc::clone(&message_bus),
        )?;
        
        Ok(Self {
            temp_dir,
            db_ops,
            schema_core,
        })
    }
    
    fn create_test_schema(&self, name: &str) -> Schema {
        let mut fields = HashMap::new();
        
        // Create a simple single field with default configurations
        let permission_policy = PermissionsPolicy::default();
        let payment_config = FieldPaymentConfig::default();
        let field_mappers = HashMap::new();
        
        fields.insert(
            "test_field".to_string(),
            FieldVariant::Single(SingleField::new(permission_policy, payment_config, field_mappers)),
        );
        
        Schema {
            name: name.to_string(),
            schema_type: SchemaType::Single,
            key: None,
            fields,
            payment_config: SchemaPaymentConfig::default(),
            hash: None,
        }
    }
    
}

#[test]
fn test_schema_field_assignments_created_on_approval() {
    let fixture = DatabaseTestFixture::new().expect("Failed to create test fixture");
    
    // 1. Create and load a schema
    let schema = fixture.create_test_schema("TestSchema");
    fixture.schema_core.load_schema_internal(schema.clone())
        .expect("Failed to load schema");
    
    // 2. Approve the schema (this should create field assignments)
    fixture.schema_core.approve_schema("TestSchema")
        .expect("Failed to approve schema");
    
    // 3. Verify field has molecule_uuid assignment
    let loaded_schema = fixture.db_ops.get_schema("TestSchema")
        .expect("Failed to get schema")
        .expect("Schema not found");
    
    let field = loaded_schema.fields.get("test_field")
        .expect("Field not found");
    
    assert!(field.molecule_uuid().is_some(), 
            "CRITICAL: Field should have molecule_uuid after approval");
    
    println!("✅ Schema field assignments are created correctly on approval");
}

#[test]
fn test_field_assignment_persistence_bug() {
    let fixture = DatabaseTestFixture::new().expect("Failed to create test fixture");
    
    // This test specifically targets the persist_if_needed backwards logic bug
    
    // 1. Create a schema
    let schema = fixture.create_test_schema("PersistenceTest");
    fixture.schema_core.load_schema_internal(schema)
        .expect("Failed to load schema");
    
    // 2. Approve the schema - this should trigger field assignment creation
    fixture.schema_core.approve_schema("PersistenceTest")
        .expect("Failed to approve schema");
    
    // 3. Check that field assignments were actually created and persisted
    let loaded_schema = fixture.db_ops.get_schema("PersistenceTest")
        .expect("Failed to get schema")
        .expect("Schema not found");
    
    for (field_name, field) in &loaded_schema.fields {
        assert!(field.molecule_uuid().is_some(),
                "CRITICAL BUG: Field '{}' has no molecule_uuid after approval - persist_if_needed logic failed", 
                field_name);
    }
    
    // 4. Verify persistence by retrieving again from database
    let reloaded_schema = fixture.db_ops.get_schema("PersistenceTest")
        .expect("Failed to get schema on reload")
        .expect("Schema not found on reload");
    
    for (field_name, field) in &reloaded_schema.fields {
        assert!(field.molecule_uuid().is_some(),
                "CRITICAL BUG: Field '{}' lost molecule_uuid after re-retrieval", 
                field_name);
    }
    
    println!("✅ Field assignment persistence bug fix verified");
}

#[test]
fn test_database_consistency_validation() {
    let fixture = DatabaseTestFixture::new().expect("Failed to create test fixture");
    
    // Create and approve a schema
    let schema = fixture.create_test_schema("ConsistencyTest");
    fixture.schema_core.load_schema_internal(schema)
        .expect("Failed to load schema");
    fixture.schema_core.approve_schema("ConsistencyTest")
        .expect("Failed to approve schema");
    
    // Verify consistency: all approved schemas should have field assignments
    let loaded_schema = fixture.db_ops.get_schema("ConsistencyTest")
        .expect("Failed to get schema")
        .expect("Schema not found");
    
    for (field_name, field) in &loaded_schema.fields {
        assert!(field.molecule_uuid().is_some(),
                "Consistency violation: Approved schema field '{}' has no molecule_uuid", 
                field_name);
        
        // Verify the molecule actually exists
        let molecule_uuid = field.molecule_uuid().unwrap();
        let molecule_exists = fixture.db_ops.get_item::<datafold::atom::Molecule>(
            &format!("ref:{}", molecule_uuid)
        ).expect("Failed to check molecule existence").is_some();
        
        assert!(molecule_exists,
                "Consistency violation: Field references non-existent molecule {}", 
                molecule_uuid);
    }
    
    println!("✅ Database consistency validation passed");
}

#[test]
fn test_schema_approval_workflow_integrity() {
    let fixture = DatabaseTestFixture::new().expect("Failed to create test fixture");
    
    // Test the complete schema approval workflow that was failing
    
    // 1. Create multiple schemas to test batch operations
    let schema1 = fixture.create_test_schema("WorkflowTest1");
    let schema2 = fixture.create_test_schema("WorkflowTest2");
    
    // 2. Load schemas
    fixture.schema_core.load_schema_internal(schema1)
        .expect("Failed to load schema1");
    fixture.schema_core.load_schema_internal(schema2)
        .expect("Failed to load schema2");
    
    // 3. Approve schemas
    fixture.schema_core.approve_schema("WorkflowTest1")
        .expect("Failed to approve schema1");
    fixture.schema_core.approve_schema("WorkflowTest2")
        .expect("Failed to approve schema2");
    
    // 4. Verify both schemas have proper field assignments
    let loaded_schema1 = fixture.db_ops.get_schema("WorkflowTest1")
        .expect("Failed to get schema1")
        .expect("Schema1 not found");
    
    let loaded_schema2 = fixture.db_ops.get_schema("WorkflowTest2")
        .expect("Failed to get schema2")
        .expect("Schema2 not found");
    
    // Check that approval workflow worked for both schemas
    for (field_name, field) in &loaded_schema1.fields {
        assert!(field.molecule_uuid().is_some(),
                "Schema1 field '{}' missing molecule_uuid after approval workflow", field_name);
    }
    
    for (field_name, field) in &loaded_schema2.fields {
        assert!(field.molecule_uuid().is_some(),
                "Schema2 field '{}' missing molecule_uuid after approval workflow", field_name);
    }
    
    println!("✅ Schema approval workflow integrity verified");
}

#[test]
fn test_cross_database_instance_persistence() {
    // Test that field assignments persist across different database instances
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("persistence_test_db");
    
    // First instance: create and approve schema
    {
        let db = sled::open(&db_path).expect("Failed to open database");
        let db_ops = Arc::new(DbOperations::new(db).expect("Failed to create DbOperations"));
        let message_bus = Arc::new(MessageBus::new());
        let schema_core = SchemaCore::new(
            temp_dir.path().to_str().unwrap(),
            Arc::clone(&db_ops),
            Arc::clone(&message_bus),
        ).expect("Failed to create SchemaCore");
        
        let mut fields = HashMap::new();
        let permission_policy = PermissionsPolicy::default();
        let payment_config = FieldPaymentConfig::default();
        let field_mappers = HashMap::new();
        
        fields.insert(
            "test_field".to_string(),
            FieldVariant::Single(SingleField::new(permission_policy, payment_config, field_mappers)),
        );
        
        let schema = Schema {
            name: "PersistenceTestSchema".to_string(),
            schema_type: SchemaType::Single,
            key: None,
            fields,
            payment_config: SchemaPaymentConfig::default(),
            hash: None,
        };
        
        schema_core.load_schema_internal(schema)
            .expect("Failed to load schema");
        schema_core.approve_schema("PersistenceTestSchema")
            .expect("Failed to approve schema");
        
        // Verify field assignment was created
        let loaded_schema = db_ops.get_schema("PersistenceTestSchema")
            .expect("Failed to get schema")
            .expect("Schema not found");
        
        let field = loaded_schema.fields.get("test_field").expect("Field not found");
        assert!(field.molecule_uuid().is_some(),
                "Field assignment not created in first instance");
    } // First instance is dropped here
    
    // Second instance: verify persistence
    {
        let db = sled::open(&db_path).expect("Failed to reopen database");
        let db_ops = Arc::new(DbOperations::new(db).expect("Failed to create DbOperations"));
        
        let reloaded_schema = db_ops.get_schema("PersistenceTestSchema")
            .expect("Failed to get schema from second instance")
            .expect("Schema not found in second instance");
        
        let reloaded_field = reloaded_schema.fields.get("test_field")
            .expect("Field not found in second instance");
        
        assert!(reloaded_field.molecule_uuid().is_some(),
                "CRITICAL: Field assignment lost across database instances");
    }
    
    println!("✅ Cross-database instance persistence verified");
}

/// Integration test to verify the complete fix for the critical bugs identified
#[test]
fn test_critical_bug_fixes_integration() {
    let fixture = DatabaseTestFixture::new().expect("Failed to create test fixture");
    
    // Test the specific bugs that were identified:
    
    // 1. Schema field assignment persistence bug (persist_if_needed backwards logic)
    let schema = fixture.create_test_schema("CriticalBugTest");
    fixture.schema_core.load_schema_internal(schema)
        .expect("Failed to load schema");
    fixture.schema_core.approve_schema("CriticalBugTest")
        .expect("Failed to approve schema");
    
    // Verify field assignments are created properly
    let loaded_schema = fixture.db_ops.get_schema("CriticalBugTest")
        .expect("Failed to get schema")
        .expect("Schema not found");
    
    let field = loaded_schema.fields.get("test_field").expect("Field not found");
    assert!(field.molecule_uuid().is_some(), 
            "BUG REGRESSION: Field assignment not created during approval");
    
    // 2. Test multiple field assignments in a single schema
    let mut multi_field_schema = fixture.create_test_schema("MultiFieldTest");
    
    // Add more fields to test batch assignment
    let permission_policy = PermissionsPolicy::default();
    let payment_config = FieldPaymentConfig::default();
    let field_mappers = HashMap::new();
    
    multi_field_schema.fields.insert(
        "field2".to_string(),
        FieldVariant::Single(SingleField::new(permission_policy.clone(), payment_config.clone(), field_mappers.clone())),
    );
    multi_field_schema.fields.insert(
        "field3".to_string(),
        FieldVariant::Single(SingleField::new(permission_policy, payment_config, field_mappers)),
    );
    
    fixture.schema_core.load_schema_internal(multi_field_schema)
        .expect("Failed to load multi-field schema");
    fixture.schema_core.approve_schema("MultiFieldTest")
        .expect("Failed to approve multi-field schema");
    
    let multi_loaded_schema = fixture.db_ops.get_schema("MultiFieldTest")
        .expect("Failed to get multi-field schema")
        .expect("Multi-field schema not found");
    
    // Verify ALL fields have assignments
    for (field_name, field) in &multi_loaded_schema.fields {
        assert!(field.molecule_uuid().is_some(),
                "BUG REGRESSION: Multi-field schema field '{}' missing assignment", field_name);
    }
    
    println!("✅ All critical bug fixes verified - no regressions detected");
}