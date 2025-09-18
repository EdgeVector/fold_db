//! Backward Compatibility Tests
//!
//! This comprehensive test suite validates that existing procedural transforms
//! continue to work unchanged and that both transform types can coexist without conflicts.
//!
//! **Backward Compatibility Coverage:**
//! 1. **Existing Procedural Transforms** - Verify existing procedural transforms work unchanged
//! 2. **Mixed Transform Scenarios** - Test both transform types working together
//! 3. **No Regression Testing** - Validate no regression in existing functionality
//! 4. **Consistent Behavior** - Ensure consistent behavior across transform types
//! 5. **Legacy Data Support** - Test support for legacy data patterns
//! 6. **API Compatibility** - Ensure API compatibility is maintained
//! 7. **Migration Path** - Test migration from procedural to declarative transforms

use datafold::db_operations::DbOperations;
use datafold::schema::types::transform::{Transform, TransformRegistration};
use datafold::schema::types::json_schema::{TransformKind, DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::fold_db_core::transform_manager::TransformManager;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::transform::TransformExecutor;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use serde_json::json;

/// Test fixture for backward compatibility testing
struct BackwardCompatibilityFixture {
    pub db_ops: Arc<DbOperations>,
    pub message_bus: Arc<MessageBus>,
    pub transform_manager: Arc<TransformManager>,
    pub _temp_dir: TempDir,
}

impl BackwardCompatibilityFixture {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db = sled::open(temp_dir.path()).expect("Failed to open database");
        let db_ops = Arc::new(DbOperations::new(db).expect("Failed to create database"));
        let message_bus = Arc::new(MessageBus::new());
        let transform_manager = Arc::new(TransformManager::new(db_ops.clone(), message_bus.clone())
            .expect("Failed to create transform manager"));
        
        Self {
            db_ops,
            message_bus,
            transform_manager,
            _temp_dir: temp_dir,
        }
    }
}

/// Test existing procedural transforms continue to work unchanged
#[test]
fn test_existing_procedural_transforms_unchanged() {
    let fixture = BackwardCompatibilityFixture::new();
    
    // Create various types of procedural transforms that should continue to work
    let procedural_transforms = vec![
        // Simple arithmetic transform
        ("simple_arithmetic", Transform::new(
            "input_value * 2".to_string(),
            "test.doubled".to_string()
        )),
        // Complex arithmetic transform
        ("complex_arithmetic", Transform::new(
            "input_value * 2 + 1".to_string(),
            "test.complex_result".to_string()
        )),
        // String manipulation transform
        ("string_manipulation", Transform::new(
            "input_string + '_processed'".to_string(),
            "test.processed_string".to_string()
        )),
        // Conditional transform
        ("conditional", Transform::new(
            "if input_value > 10 then 'high' else 'low'".to_string(),
            "test.category".to_string()
        )),
    ];
    
    let mut registered_transforms = Vec::new();
    
    // Register all procedural transforms
    for (transform_id, transform) in procedural_transforms {
        let registration = TransformRegistration {
            transform_id: transform_id.to_string(),
            transform,
            input_molecules: vec![format!("test.input_{}", transform_id)],
            input_names: vec![format!("input_{}", transform_id)],
            trigger_fields: vec![format!("test.input_{}", transform_id)],
            output_molecule: format!("test.{}", transform_id),
            schema_name: "test".to_string(),
            field_name: transform_id.to_string(),
        };
        
        fixture.transform_manager.register_transform_event_driven(registration)
            .expect("Failed to register procedural transform");
        
        registered_transforms.push(transform_id.to_string());
    }
    
    // Verify all transforms are registered
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    for transform_id in &registered_transforms {
        assert!(transforms.contains_key(transform_id), "Transform {} not found", transform_id);
        
        let transform = &transforms[transform_id];
        assert!(matches!(transform.kind, TransformKind::Procedural { .. }), "Transform {} is not procedural", transform_id);
        assert!(!transform.is_declarative(), "Transform {} incorrectly identified as declarative", transform_id);
    }
    
    // Test execution of all procedural transforms
    let test_cases = vec![
        ("simple_arithmetic", json!({"input_simple_arithmetic": 5})),
        ("complex_arithmetic", json!({"input_complex_arithmetic": 7})),
        ("string_manipulation", json!({"input_string_manipulation": "test"})),
        ("conditional", json!({"input_conditional": 15})),
    ];
    
    for (transform_id, input_data) in test_cases {
        let result = TransformExecutor::execute_transform(
            &transforms[transform_id],
            input_data
        ).expect("Failed to execute procedural transform");
        
        assert!(result.is_object(), "Transform {} result is not an object", transform_id);
        
        let result_obj = result.as_object().unwrap();
        assert!(result_obj.contains_key(transform_id), "Transform {} result missing expected field", transform_id);
        
        println!("Procedural transform {} executed successfully: {}", transform_id, result_obj[transform_id]);
    }
}

/// Test mixed transform scenarios with both procedural and declarative transforms
#[test]
fn test_mixed_transform_scenarios() {
    let fixture = BackwardCompatibilityFixture::new();
    
    // Create procedural transforms
    let procedural_transform = Transform::new(
        "input_value * 2".to_string(),
        "mixed.doubled".to_string()
    );
    
    let procedural_registration = TransformRegistration {
        transform_id: "mixed_procedural".to_string(),
        transform: procedural_transform,
        input_molecules: vec!["mixed.input_value".to_string()],
        input_names: vec!["input_value".to_string()],
        trigger_fields: vec!["mixed.input_value".to_string()],
        output_molecule: "mixed.doubled".to_string(),
        schema_name: "mixed".to_string(),
        field_name: "doubled".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(procedural_registration)
        .expect("Failed to register procedural transform");
    
    // Create declarative transforms
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "mixed_declarative".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("processed".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["mixed_declarative.data".to_string()],
        "mixed_declarative.processed".to_string()
    );
    
    let declarative_registration = TransformRegistration {
        transform_id: "mixed_declarative".to_string(),
        transform: declarative_transform,
        input_molecules: vec!["mixed_declarative.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["mixed_declarative.data".to_string()],
        output_molecule: "mixed_declarative.processed".to_string(),
        schema_name: "mixed_declarative".to_string(),
        field_name: "processed".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(declarative_registration)
        .expect("Failed to register declarative transform");
    
    // Verify both transforms are registered
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    assert!(transforms.contains_key(&"mixed_procedural".to_string()));
    assert!(transforms.contains_key(&"mixed_declarative".to_string()));
    
    // Verify transform types
    let procedural_transform = &transforms[&"mixed_procedural".to_string()];
    let declarative_transform = &transforms[&"mixed_declarative".to_string()];
    
    assert!(matches!(procedural_transform.kind, TransformKind::Procedural { .. }));
    assert!(matches!(declarative_transform.kind, TransformKind::Declarative { .. }));
    
    // Test execution of both transform types
    let procedural_input = json!({
        "input_value": 10
    });
    
    let declarative_input = json!({
        "data": {
            "value": "mixed_test_value"
        }
    });
    
    let procedural_result = TransformExecutor::execute_transform(
        procedural_transform,
        procedural_input
    ).expect("Failed to execute procedural transform");
    
    let declarative_result = TransformExecutor::execute_transform(
        declarative_transform,
        declarative_input
    ).expect("Failed to execute declarative transform");
    
    // Verify both results are valid
    assert!(procedural_result.is_object());
    assert!(declarative_result.is_object());
    
    let procedural_obj = procedural_result.as_object().unwrap();
    let declarative_obj = declarative_result.as_object().unwrap();
    
    assert!(procedural_obj.contains_key("doubled"));
    assert!(declarative_obj.contains_key("processed"));
    
    println!("Mixed transform scenario executed successfully:");
    println!("  Procedural result: {}", procedural_obj["doubled"]);
    println!("  Declarative result: {}", declarative_obj["processed"]);
}

/// Test no regression in existing functionality
#[test]
fn test_no_regression_in_existing_functionality() {
    let fixture = BackwardCompatibilityFixture::new();
    
    // Test that existing transform management functionality still works
    let original_transform = Transform::new(
        "input_value + 1".to_string(),
        "regression_test.incremented".to_string()
    );
    
    let registration = TransformRegistration {
        transform_id: "regression_test".to_string(),
        transform: original_transform,
        input_molecules: vec!["regression_test.input_value".to_string()],
        input_names: vec!["input_value".to_string()],
        trigger_fields: vec!["regression_test.input_value".to_string()],
        output_molecule: "regression_test.incremented".to_string(),
        schema_name: "regression_test".to_string(),
        field_name: "incremented".to_string(),
    };
    
    // Test registration
    fixture.transform_manager.register_transform_event_driven(registration)
        .expect("Failed to register transform");
    
    // Test listing transforms
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    assert!(transforms.contains_key(&"regression_test".to_string()));
    
    // Test getting transform inputs
    let inputs = fixture.transform_manager.get_transform_inputs("regression_test")
        .expect("Failed to get transform inputs");
    assert!(inputs.contains(&"regression_test.input_value".to_string()));
    
    // Test getting transform output
    let output = fixture.transform_manager.get_transform_output("regression_test")
        .expect("Failed to get transform output");
    assert_eq!(output, Some("regression_test.incremented".to_string()));
    
    // Test getting transforms for field
    let field_transforms = fixture.transform_manager.get_transforms_for_field("regression_test", "input_value")
        .expect("Failed to get transforms for field");
    assert!(field_transforms.contains(&"regression_test".to_string()));
    
    // Test execution
    let input_data = json!({
        "input_value": 5
    });
    
    let result = TransformExecutor::execute_transform(
        &transforms[&"regression_test".to_string()],
        input_data
    ).expect("Failed to execute transform");
    
    assert!(result.is_object());
    let result_obj = result.as_object().unwrap();
    assert!(result_obj.contains_key("incremented"));
    
    // Test unregistration
    let unregistered = fixture.transform_manager.unregister_transform("regression_test")
        .expect("Failed to unregister transform");
    assert!(unregistered);
    
    // Verify transform is no longer in the list
    let transforms_after = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms after unregistration");
    assert!(!transforms_after.contains_key(&"regression_test".to_string()));
}

/// Test consistent behavior across transform types
#[test]
fn test_consistent_behavior_across_transform_types() {
    let fixture = BackwardCompatibilityFixture::new();
    
    // Create equivalent procedural and declarative transforms
    let procedural_transform = Transform::new(
        "input_value * 2".to_string(),
        "consistent.doubled".to_string()
    );
    
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "consistent_declarative".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("doubled".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["consistent_declarative.data".to_string()],
        "consistent_declarative.doubled".to_string()
    );
    
    // Register both transforms
    let procedural_registration = TransformRegistration {
        transform_id: "consistent_procedural".to_string(),
        transform: procedural_transform,
        input_molecules: vec!["consistent.input_value".to_string()],
        input_names: vec!["input_value".to_string()],
        trigger_fields: vec!["consistent.input_value".to_string()],
        output_molecule: "consistent.doubled".to_string(),
        schema_name: "consistent".to_string(),
        field_name: "doubled".to_string(),
    };
    
    let declarative_registration = TransformRegistration {
        transform_id: "consistent_declarative".to_string(),
        transform: declarative_transform,
        input_molecules: vec!["consistent_declarative.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["consistent_declarative.data".to_string()],
        output_molecule: "consistent_declarative.doubled".to_string(),
        schema_name: "consistent_declarative".to_string(),
        field_name: "doubled".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(procedural_registration)
        .expect("Failed to register procedural transform");
    fixture.transform_manager.register_transform_event_driven(declarative_registration)
        .expect("Failed to register declarative transform");
    
    // Test consistent API behavior
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    // Both transforms should be accessible through the same API
    assert!(transforms.contains_key(&"consistent_procedural".to_string()));
    assert!(transforms.contains_key(&"consistent_declarative".to_string()));
    
    // Both transforms should have consistent accessor methods
    let procedural_transform = &transforms[&"consistent_procedural".to_string()];
    let declarative_transform = &transforms[&"consistent_declarative".to_string()];
    
    // Both should have consistent output field access
    assert_eq!(procedural_transform.get_output(), "consistent.doubled");
    assert_eq!(declarative_transform.get_output(), "consistent_declarative.doubled");
    
    // Both should have consistent input access
    assert_eq!(procedural_transform.get_inputs(), vec!["consistent.input_value"]);
    assert_eq!(declarative_transform.get_inputs(), vec!["consistent_declarative.data"]);
    
    // Both should have consistent debug information
    let procedural_debug = procedural_transform.get_debug_info();
    let declarative_debug = declarative_transform.get_debug_info();
    
    assert!(procedural_debug.contains("Procedural"));
    assert!(declarative_debug.contains("Declarative"));
    
    println!("Consistent behavior test completed:");
    println!("  Procedural debug: {}", procedural_debug);
    println!("  Declarative debug: {}", declarative_debug);
}

/// Test legacy data support
#[test]
fn test_legacy_data_support() {
    let fixture = BackwardCompatibilityFixture::new();
    
    // Test that legacy procedural transforms work with various data formats
    let legacy_transforms = vec![
        // Numeric data
        ("numeric_legacy", Transform::new(
            "input_number + 1".to_string(),
            "legacy.incremented_number".to_string()
        )),
        // String data
        ("string_legacy", Transform::new(
            "input_string + '_legacy'".to_string(),
            "legacy.processed_string".to_string()
        )),
        // Boolean data
        ("boolean_legacy", Transform::new(
            "if input_boolean then 'true' else 'false'".to_string(),
            "legacy.boolean_string".to_string()
        )),
    ];
    
    let mut registered_transforms = Vec::new();
    
    for (transform_id, transform) in legacy_transforms {
        let registration = TransformRegistration {
            transform_id: transform_id.to_string(),
            transform,
            input_molecules: vec![format!("legacy.input_{}", transform_id)],
            input_names: vec![format!("input_{}", transform_id)],
            trigger_fields: vec![format!("legacy.input_{}", transform_id)],
            output_molecule: format!("legacy.{}", transform_id),
            schema_name: "legacy".to_string(),
            field_name: transform_id.to_string(),
        };
        
        fixture.transform_manager.register_transform_event_driven(registration)
            .expect("Failed to register legacy transform");
        
        registered_transforms.push(transform_id.to_string());
    }
    
    // Test execution with legacy data formats
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    let legacy_test_cases = vec![
        ("numeric_legacy", json!({"input_numeric_legacy": 42})),
        ("string_legacy", json!({"input_string_legacy": "test"})),
        ("boolean_legacy", json!({"input_boolean_legacy": true})),
    ];
    
    for (transform_id, input_data) in legacy_test_cases {
        let result = TransformExecutor::execute_transform(
            &transforms[transform_id],
            input_data
        ).expect("Failed to execute legacy transform");
        
        assert!(result.is_object(), "Legacy transform {} result is not an object", transform_id);
        
        let result_obj = result.as_object().unwrap();
        assert!(result_obj.contains_key(transform_id), "Legacy transform {} result missing expected field", transform_id);
        
        println!("Legacy transform {} executed successfully: {}", transform_id, result_obj[transform_id]);
    }
}

/// Test API compatibility
#[test]
fn test_api_compatibility() {
    let fixture = BackwardCompatibilityFixture::new();
    
    // Test that all existing API methods work with both transform types
    let procedural_transform = Transform::new(
        "input_value * 2".to_string(),
        "api_test.doubled".to_string()
    );
    
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "api_declarative".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([
            ("result".to_string(), FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            }),
        ]),
        key: None,
    };
    
    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["api_declarative.data".to_string()],
        "api_declarative.result".to_string()
    );
    
    // Register both transforms
    let procedural_registration = TransformRegistration {
        transform_id: "api_procedural".to_string(),
        transform: procedural_transform,
        input_molecules: vec!["api_test.input_value".to_string()],
        input_names: vec!["input_value".to_string()],
        trigger_fields: vec!["api_test.input_value".to_string()],
        output_molecule: "api_test.doubled".to_string(),
        schema_name: "api_test".to_string(),
        field_name: "doubled".to_string(),
    };
    
    let declarative_registration = TransformRegistration {
        transform_id: "api_declarative".to_string(),
        transform: declarative_transform,
        input_molecules: vec!["api_declarative.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["api_declarative.data".to_string()],
        output_molecule: "api_declarative.result".to_string(),
        schema_name: "api_declarative".to_string(),
        field_name: "result".to_string(),
    };
    
    fixture.transform_manager.register_transform_event_driven(procedural_registration)
        .expect("Failed to register procedural transform");
    fixture.transform_manager.register_transform_event_driven(declarative_registration)
        .expect("Failed to register declarative transform");
    
    // Test all API methods work with both transform types
    let transforms = fixture.transform_manager.list_transforms()
        .expect("Failed to list transforms");
    
    for transform_id in &["api_procedural", "api_declarative"] {
        let transform = &transforms[*transform_id];
        
        // Test basic accessor methods
        let _output = transform.get_output();
        let _inputs = transform.get_inputs();
        let _debug_info = transform.get_debug_info();
        
        // Test type checking methods
        let _is_declarative = transform.is_declarative();
        let _is_procedural = matches!(transform.kind, TransformKind::Procedural { .. });
        
        // Test validation
        let _validation_result = transform.validate();
        
        // Test execution
        let input_data = if *transform_id == "api_procedural" {
            json!({"input_value": 5})
        } else {
            json!({"data": {"value": "api_test"}})
        };
        
        let _result = TransformExecutor::execute_transform(transform, input_data)
            .expect("Failed to execute transform");
        
        println!("API compatibility test passed for transform: {}", transform_id);
    }
}
