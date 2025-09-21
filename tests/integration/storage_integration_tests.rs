use datafold::db_operations::DbOperations;
use datafold::fold_db_core::infrastructure::message_bus::MessageBus;
use datafold::fold_db_core::transform_manager::TransformManager;
use datafold::schema::types::json_schema::{DeclarativeSchemaDefinition, FieldDefinition};
use datafold::schema::types::schema::SchemaType;
use datafold::schema::types::transform::{Transform, TransformRegistration};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Test storing and retrieving both transform types
#[test]
fn test_storage_integration_both_transform_types() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db = sled::open(temp_dir.path()).expect("Failed to open database");
    let db_ops = DbOperations::new(db).expect("Failed to create database");

    // Create a declarative transform (replacing procedural)
    let procedural_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "result".to_string(),
            FieldDefinition {
                field_type: Some("Number".to_string()),
                atom_uuid: Some("field1.map().add(field2)".to_string()),
            },
        )]),
        key: None,
    };

    let procedural_transform = Transform::from_declarative_schema(
        procedural_schema,
        vec![
            "test_schema.field1".to_string(),
            "test_schema.field2".to_string(),
        ],
        "test_schema.result".to_string(),
    );

    // Create a declarative transform
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "result".to_string(),
            FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            },
        )]),
        key: None,
    };

    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_schema.data".to_string()],
        "test_schema.result".to_string(),
    );

    // Store both transforms
    db_ops
        .store_transform("procedural_test", &procedural_transform)
        .expect("Failed to store procedural transform");
    db_ops
        .store_transform("declarative_test", &declarative_transform)
        .expect("Failed to store declarative transform");

    // Retrieve both transforms
    let retrieved_procedural = db_ops
        .get_transform("procedural_test")
        .expect("Failed to get procedural transform")
        .expect("Procedural transform not found");

    let retrieved_declarative = db_ops
        .get_transform("declarative_test")
        .expect("Failed to get declarative transform")
        .expect("Declarative transform not found");

    // Verify procedural transform
    assert_eq!(retrieved_procedural.get_output(), "test_schema.result");
    assert!(retrieved_procedural.is_declarative());
    assert_eq!(
        retrieved_procedural.get_declarative_schema().unwrap().name,
        "test_schema"
    );

    // Verify declarative transform
    assert_eq!(retrieved_declarative.get_output(), "test_schema.result");
    assert!(retrieved_declarative.is_declarative());
    let declarative_schema_retrieved = retrieved_declarative.get_declarative_schema().unwrap();
    assert_eq!(declarative_schema_retrieved.name, "test_schema");
    assert_eq!(declarative_schema_retrieved.schema_type, SchemaType::Single);
}

/// Test transform manager integration with both transform types
#[test]
fn test_transform_manager_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db = sled::open(temp_dir.path()).expect("Failed to open database");
    let db_ops = Arc::new(DbOperations::new(db).expect("Failed to create database"));
    let message_bus = Arc::new(MessageBus::new());

    let transform_manager = TransformManager::new(db_ops.clone(), message_bus)
        .expect("Failed to create transform manager");

    // Create and register a declarative transform (replacing procedural)
    let procedural_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "doubled".to_string(),
            FieldDefinition {
                field_type: Some("Number".to_string()),
                atom_uuid: Some("input_field.map().multiply(2)".to_string()),
            },
        )]),
        key: None,
    };

    let procedural_transform = Transform::from_declarative_schema(
        procedural_schema,
        vec!["test_schema.input_field".to_string()],
        "test_schema.doubled".to_string(),
    );

    let procedural_registration = TransformRegistration {
        transform_id: "procedural_double".to_string(),
        transform: procedural_transform,
        input_molecules: vec!["test_schema.input_field".to_string()],
        input_names: vec!["input_field".to_string()],
        trigger_fields: vec!["test_schema.input_field".to_string()],
        output_molecule: "test_schema.doubled".to_string(),
        schema_name: "test_schema".to_string(),
        field_name: "doubled".to_string(),
    };

    transform_manager
        .register_transform_event_driven(procedural_registration)
        .expect("Failed to register procedural transform");

    // Create and register a declarative transform
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "processed".to_string(),
            FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.map().value".to_string()),
            },
        )]),
        key: None,
    };

    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_schema.data".to_string()],
        "test_schema.processed".to_string(),
    );

    let declarative_registration = TransformRegistration {
        transform_id: "declarative_process".to_string(),
        transform: declarative_transform,
        input_molecules: vec!["test_schema.data".to_string()],
        input_names: vec!["data".to_string()],
        trigger_fields: vec!["test_schema.data".to_string()],
        output_molecule: "test_schema.processed".to_string(),
        schema_name: "test_schema".to_string(),
        field_name: "processed".to_string(),
    };

    transform_manager
        .register_transform_event_driven(declarative_registration)
        .expect("Failed to register declarative transform");

    // Verify both transforms are registered
    let transforms = transform_manager
        .list_transforms()
        .expect("Failed to list transforms");
    assert!(transforms.contains_key(&"procedural_double".to_string()));
    assert!(transforms.contains_key(&"declarative_process".to_string()));

    // Verify field mappings are created
    let procedural_transforms = transform_manager
        .get_transforms_for_field("test_schema", "input_field")
        .expect("Failed to get transforms for field");
    assert!(procedural_transforms.contains(&"procedural_double".to_string()));

    let declarative_transforms = transform_manager
        .get_transforms_for_field("test_schema", "data")
        .expect("Failed to get transforms for field");
    assert!(declarative_transforms.contains(&"declarative_process".to_string()));
}

/// Test backward compatibility with existing procedural transforms
#[test]
fn test_backward_compatibility_procedural_transforms() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db = sled::open(temp_dir.path()).expect("Failed to open database");
    let db_ops = DbOperations::new(db).expect("Failed to create database");

    // Create a legacy procedural transform (without explicit kind)
    let legacy_transform = Transform {
        inputs: vec!["test_schema.input".to_string()],
        output: "test_schema.output".to_string(),
        schema: DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields: HashMap::from([(
                "output".to_string(),
                FieldDefinition {
                    field_type: Some("String".to_string()),
                    atom_uuid: Some("input.map().add(1)".to_string()),
                },
            )]),
            key: None,
        },
        parsed_expression: None,
    };

    // Store the legacy transform
    db_ops
        .store_transform("legacy_procedural", &legacy_transform)
        .expect("Failed to store legacy transform");

    // Retrieve and verify it works
    let retrieved = db_ops
        .get_transform("legacy_procedural")
        .expect("Failed to get legacy transform")
        .expect("Legacy transform not found");

    assert_eq!(retrieved.get_output(), "test_schema.output");
    assert!(retrieved.is_declarative());
    assert_eq!(
        retrieved.get_declarative_schema().unwrap().name,
        "test_schema"
    );
    assert_eq!(retrieved.get_inputs(), vec!["test_schema.input"]);
}

/// Test debug information for both transform types
#[test]
fn test_debug_information_both_types() {
    // Test declarative transform debug info (replacing procedural)
    let procedural_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "doubled".to_string(),
            FieldDefinition {
                field_type: Some("Number".to_string()),
                atom_uuid: Some("x.map().multiply(2)".to_string()),
            },
        )]),
        key: None,
    };

    let procedural_transform = Transform::from_declarative_schema(
        procedural_schema,
        vec!["test.x".to_string()],
        "test.doubled".to_string(),
    );

    let procedural_debug = procedural_transform.get_debug_info();
    assert!(procedural_debug.contains("test_schema"));
    assert!(procedural_debug.contains("test.doubled"));

    // Test declarative transform debug info
    let declarative_schema = DeclarativeSchemaDefinition {
        name: "test_schema".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "result".to_string(),
            FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("data.value".to_string()),
            },
        )]),
        key: None,
    };

    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["test_schema.data".to_string()],
        "test_schema.result".to_string(),
    );

    let declarative_debug = declarative_transform.get_debug_info();
    assert!(declarative_debug.contains("test_schema"));
    assert!(declarative_debug.contains("test_schema.result"));
}

/// Test error handling for invalid transform storage
#[test]
fn test_error_handling_invalid_transforms() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db = sled::open(temp_dir.path()).expect("Failed to open database");
    let db_ops = DbOperations::new(db).expect("Failed to create database");

    // Test storing transform with empty output
    let invalid_transform = Transform {
        inputs: vec![],
        output: String::new(), // Empty output should cause validation error
        schema: DeclarativeSchemaDefinition {
            name: "test_schema".to_string(),
            schema_type: SchemaType::Single,
            fields: HashMap::from([(
                "output".to_string(),
                FieldDefinition {
                    field_type: Some("String".to_string()),
                    atom_uuid: Some("1".to_string()),
                },
            )]),
            key: None,
        },
        parsed_expression: None,
    };

    // This should fail validation
    let result = invalid_transform.validate();
    assert!(result.is_err());

    // Test retrieving non-existent transform
    let result = db_ops.get_transform("non_existent");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

/// Test performance of storage operations for both transform types
#[test]
fn test_storage_performance_both_types() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db = sled::open(temp_dir.path()).expect("Failed to open database");
    let db_ops = DbOperations::new(db).expect("Failed to create database");

    let start_time = std::time::Instant::now();

    // Store multiple transforms of both types
    for i in 0..10 {
        let procedural_schema = DeclarativeSchemaDefinition {
            name: format!("test_schema_{}", i),
            schema_type: SchemaType::Single,
            fields: HashMap::from([(
                "result".to_string(),
                FieldDefinition {
                    field_type: Some("Number".to_string()),
                    atom_uuid: Some(format!("field{}.map().multiply(2)", i)),
                },
            )]),
            key: None,
        };

        let procedural_transform = Transform::from_declarative_schema(
            procedural_schema,
            vec![format!("test_schema_{}.field{}", i, i)],
            format!("test_schema.result{}", i),
        );

        db_ops
            .store_transform(&format!("procedural_{}", i), &procedural_transform)
            .expect("Failed to store procedural transform");

        let declarative_schema = DeclarativeSchemaDefinition {
            name: format!("test_schema_{}", i),
            schema_type: SchemaType::Single,
            fields: HashMap::from([(
                format!("result{}", i),
                FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some(format!("data.field{}", i)),
                },
            )]),
            key: None,
        };

        let declarative_transform = Transform::from_declarative_schema(
            declarative_schema,
            vec![format!("test_schema_{}.data", i)],
            format!("test_schema_{}.result{}", i, i),
        );

        db_ops
            .store_transform(&format!("declarative_{}", i), &declarative_transform)
            .expect("Failed to store declarative transform");
    }

    let store_duration = start_time.elapsed();

    // Retrieve all transforms
    let retrieve_start = std::time::Instant::now();

    for i in 0..10 {
        let procedural = db_ops
            .get_transform(&format!("procedural_{}", i))
            .expect("Failed to get procedural transform")
            .expect("Procedural transform not found");
        assert!(procedural.is_declarative());

        let declarative = db_ops
            .get_transform(&format!("declarative_{}", i))
            .expect("Failed to get declarative transform")
            .expect("Declarative transform not found");
        assert!(declarative.is_declarative());
    }

    let retrieve_duration = retrieve_start.elapsed();

    // Performance should be reasonable (less than 1 second for 20 operations)
    assert!(store_duration.as_millis() < 1000);
    assert!(retrieve_duration.as_millis() < 1000);

    println!(
        "Storage performance: {}ms store, {}ms retrieve",
        store_duration.as_millis(),
        retrieve_duration.as_millis()
    );
}
