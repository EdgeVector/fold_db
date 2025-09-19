use datafold::schema::types::{
    json_schema::{DeclarativeSchemaDefinition, FieldDefinition, KeyConfig},
    Transform, TransformRegistration,
};
use datafold::schema::SchemaType;
use std::collections::HashMap;

// Import shared test utilities
use crate::declarative_transform_test_utils::{
    DeclarativeTransformIntegrationFixture, TestAssertions, TestSchemaBuilder, TestTransformBuilder,
};

/// Test end-to-end declarative transform workflow
#[test]
fn test_end_to_end_declarative_transform_workflow() {
    let fixture = DeclarativeTransformIntegrationFixture::new()
        .expect("Failed to create integration test fixture");

    // Step 1: Create a declarative transform using shared utility
    let declarative_schema = TestSchemaBuilder::create_single_schema("blog_processing");

    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["blog_processing.blogpost".to_string()],
        "blog_processing.processed_content".to_string(),
    );

    // Step 2: Validate the transform
    let validation_result = declarative_transform.validate();
    TestAssertions::assert_schema_validation_success(validation_result, "End-to-end workflow test");

    // Step 3: Register the transform using shared utility
    let registration = TestTransformBuilder::create_transform_registration(
        "blog_processor",
        TestSchemaBuilder::create_single_schema("blog_processing"),
        vec!["blog_processing.blogpost".to_string()],
        "blog_processing.processed_content",
    );

    fixture
        .transform_manager
        .register_transform_event_driven(registration)
        .expect("Failed to register declarative transform");

    // Step 4: Verify transform is registered
    let transforms = fixture
        .transform_manager
        .list_transforms()
        .expect("Failed to list transforms");

    assert!(
        transforms.contains_key(&"blog_processor".to_string()),
        "Transform should be registered"
    );

    let registered_transform = &transforms["blog_processor"];
    TestAssertions::assert_transform_is_declarative(
        registered_transform,
        "End-to-end workflow test",
    );

    // Step 5: Verify transform can be retrieved
    let transform_exists = fixture
        .transform_manager
        .transform_exists("blog_processor")
        .expect("Failed to check transform existence");
    assert!(transform_exists, "Transform should exist");

    // Step 6: Verify field mappings are created
    let field_transforms = fixture
        .transform_manager
        .get_transforms_for_field("blog_processing", "blogpost")
        .expect("Failed to get transforms for field");

    assert!(
        field_transforms.contains(&"blog_processor".to_string()),
        "Field should be mapped to transform"
    );
}

/// Test declarative transform with HashRange schema type
#[test]
fn test_hashrange_declarative_transform() {
    let fixture = DeclarativeTransformIntegrationFixture::new()
        .expect("Failed to create integration test fixture");

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "word_index".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            (
                "word".to_string(),
                FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some("text.map().content.split_by_word().map()".to_string()),
                },
            ),
            (
                "source".to_string(),
                FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some("text.map().$atom_uuid".to_string()),
                },
            ),
        ]),
        key: Some(KeyConfig {
            hash_field: "text.map().content.split_by_word().map()".to_string(),
            range_field: "text.map().timestamp".to_string(),
        }),
    };

    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["text".to_string()],
        "word_index.key".to_string(),
    );

    let registration = TransformRegistration {
        transform_id: "word_indexer".to_string(),
        transform: declarative_transform,
        input_molecules: vec!["text".to_string()],
        input_names: vec!["text".to_string()],
        trigger_fields: vec!["text".to_string()],
        output_molecule: "word_index.key".to_string(),
        schema_name: "word_index".to_string(),
        field_name: "key".to_string(),
    };

    fixture
        .transform_manager
        .register_transform_event_driven(registration)
        .expect("Failed to register HashRange declarative transform");

    // Verify transform is registered
    let transforms = fixture
        .transform_manager
        .list_transforms()
        .expect("Failed to list transforms");

    assert!(
        transforms.contains_key(&"word_indexer".to_string()),
        "HashRange transform should be registered"
    );

    let registered_transform = &transforms["word_indexer"];
    assert!(
        registered_transform.is_declarative(),
        "Registered transform should be declarative"
    );

    // Verify the declarative schema is preserved
    let retrieved_schema = registered_transform
        .get_declarative_schema()
        .expect("Should have declarative schema");
    assert_eq!(retrieved_schema.name, "word_index");
    assert!(matches!(
        retrieved_schema.schema_type,
        SchemaType::HashRange
    ));
}

/// Test multiple declarative transforms
#[test]
fn test_multiple_declarative_transforms() {
    let fixture = DeclarativeTransformIntegrationFixture::new()
        .expect("Failed to create integration test fixture");

    // Create first transform
    let schema1 = DeclarativeSchemaDefinition {
        name: "transform1".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "output1".to_string(),
            FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input1.map().value".to_string()),
            },
        )]),
        key: None,
    };

    let transform1 = Transform::from_declarative_schema(
        schema1,
        vec!["input1".to_string()],
        "transform1.output1".to_string(),
    );

    let registration1 = TransformRegistration {
        transform_id: "declarative_1".to_string(),
        transform: transform1,
        input_molecules: vec!["input1".to_string()],
        input_names: vec!["input1".to_string()],
        trigger_fields: vec!["input1".to_string()],
        output_molecule: "transform1.output1".to_string(),
        schema_name: "transform1".to_string(),
        field_name: "output1".to_string(),
    };

    fixture
        .transform_manager
        .register_transform_event_driven(registration1)
        .expect("Failed to register first transform");

    // Create second transform
    let schema2 = DeclarativeSchemaDefinition {
        name: "transform2".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "output2".to_string(),
            FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("input2.map().value".to_string()),
            },
        )]),
        key: None,
    };

    let transform2 = Transform::from_declarative_schema(
        schema2,
        vec!["input2".to_string()],
        "transform2.output2".to_string(),
    );

    let registration2 = TransformRegistration {
        transform_id: "declarative_2".to_string(),
        transform: transform2,
        input_molecules: vec!["input2".to_string()],
        input_names: vec!["input2".to_string()],
        trigger_fields: vec!["input2".to_string()],
        output_molecule: "transform2.output2".to_string(),
        schema_name: "transform2".to_string(),
        field_name: "output2".to_string(),
    };

    fixture
        .transform_manager
        .register_transform_event_driven(registration2)
        .expect("Failed to register second transform");

    // Verify both transforms are registered
    let transforms = fixture
        .transform_manager
        .list_transforms()
        .expect("Failed to list transforms");

    assert!(
        transforms.contains_key(&"declarative_1".to_string()),
        "First transform should be registered"
    );
    assert!(
        transforms.contains_key(&"declarative_2".to_string()),
        "Second transform should be registered"
    );

    assert_eq!(transforms.len(), 2, "Should have exactly 2 transforms");

    // Verify both are declarative
    assert!(transforms["declarative_1"].is_declarative());
    assert!(transforms["declarative_2"].is_declarative());
}

/// Test declarative transform with complex field dependencies
#[test]
fn test_complex_field_dependencies() {
    let fixture = DeclarativeTransformIntegrationFixture::new()
        .expect("Failed to create integration test fixture");

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "complex_processor".to_string(),
        schema_type: SchemaType::HashRange,
        fields: HashMap::from([
            (
                "main_data".to_string(),
                FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some("source.map().data".to_string()),
                },
            ),
            (
                "nested_info".to_string(),
                FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some("source.map().nested.field".to_string()),
                },
            ),
            (
                "computed_value".to_string(),
                FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some("source.map().computed.result".to_string()),
                },
            ),
        ]),
        key: Some(KeyConfig {
            hash_field: "source.map().data".to_string(),
            range_field: "source.map().timestamp".to_string(),
        }),
    };

    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["source".to_string()],
        "complex_processor.key".to_string(),
    );

    let registration = TransformRegistration {
        transform_id: "complex_processor".to_string(),
        transform: declarative_transform,
        input_molecules: vec!["source".to_string()],
        input_names: vec!["source".to_string()],
        trigger_fields: vec!["source".to_string()],
        output_molecule: "complex_processor.key".to_string(),
        schema_name: "complex_processor".to_string(),
        field_name: "key".to_string(),
    };

    fixture
        .transform_manager
        .register_transform_event_driven(registration)
        .expect("Failed to register complex transform");

    // Verify transform is registered
    let transforms = fixture
        .transform_manager
        .list_transforms()
        .expect("Failed to list transforms");

    assert!(
        transforms.contains_key(&"complex_processor".to_string()),
        "Complex transform should be registered"
    );

    let registered_transform = &transforms["complex_processor"];
    let retrieved_schema = registered_transform
        .get_declarative_schema()
        .expect("Should have declarative schema");

    // Verify all fields are preserved
    assert_eq!(retrieved_schema.fields.len(), 3);
    assert!(retrieved_schema.fields.contains_key("main_data"));
    assert!(retrieved_schema.fields.contains_key("nested_info"));
    assert!(retrieved_schema.fields.contains_key("computed_value"));
}

/// Test declarative transform persistence and reload
#[test]
fn test_declarative_transform_persistence() {
    let fixture = DeclarativeTransformIntegrationFixture::new()
        .expect("Failed to create integration test fixture");

    let declarative_schema = DeclarativeSchemaDefinition {
        name: "persistent_transform".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "persistent_output".to_string(),
            FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("persistent_input.map().value".to_string()),
            },
        )]),
        key: None,
    };

    let declarative_transform = Transform::from_declarative_schema(
        declarative_schema,
        vec!["persistent_input".to_string()],
        "persistent_transform.persistent_output".to_string(),
    );

    let registration = TransformRegistration {
        transform_id: "persistent_test".to_string(),
        transform: declarative_transform,
        input_molecules: vec!["persistent_input".to_string()],
        input_names: vec!["persistent_input".to_string()],
        trigger_fields: vec!["persistent_input".to_string()],
        output_molecule: "persistent_transform.persistent_output".to_string(),
        schema_name: "persistent_transform".to_string(),
        field_name: "persistent_output".to_string(),
    };

    fixture
        .transform_manager
        .register_transform_event_driven(registration)
        .expect("Failed to register persistent transform");

    // Verify transform is in memory
    let transforms = fixture
        .transform_manager
        .list_transforms()
        .expect("Failed to list transforms");
    assert!(transforms.contains_key(&"persistent_test".to_string()));

    // Test reload functionality
    fixture
        .transform_manager
        .reload_transforms()
        .expect("Failed to reload transforms");

    // Verify transform is still available after reload
    let transforms_after_reload = fixture
        .transform_manager
        .list_transforms()
        .expect("Failed to list transforms after reload");
    assert!(transforms_after_reload.contains_key(&"persistent_test".to_string()));

    let reloaded_transform = &transforms_after_reload["persistent_test"];
    assert!(reloaded_transform.is_declarative());

    // Verify schema is preserved through reload
    let reloaded_schema = reloaded_transform
        .get_declarative_schema()
        .expect("Should have declarative schema after reload");
    assert_eq!(reloaded_schema.name, "persistent_transform");
}

/// Test declarative transform with different schema types
#[test]
fn test_declarative_transform_schema_types() {
    let fixture = DeclarativeTransformIntegrationFixture::new()
        .expect("Failed to create integration test fixture");

    // Test Single schema type
    let single_schema = DeclarativeSchemaDefinition {
        name: "single_type".to_string(),
        schema_type: SchemaType::Single,
        fields: HashMap::from([(
            "single_output".to_string(),
            FieldDefinition {
                field_type: Some("single".to_string()),
                atom_uuid: Some("single_input.map().value".to_string()),
            },
        )]),
        key: None,
    };

    let single_transform = Transform::from_declarative_schema(
        single_schema,
        vec!["single_input".to_string()],
        "single_type.single_output".to_string(),
    );

    let single_registration = TransformRegistration {
        transform_id: "single_test".to_string(),
        transform: single_transform,
        input_molecules: vec!["single_input".to_string()],
        input_names: vec!["single_input".to_string()],
        trigger_fields: vec!["single_input".to_string()],
        output_molecule: "single_type.single_output".to_string(),
        schema_name: "single_type".to_string(),
        field_name: "single_output".to_string(),
    };

    fixture
        .transform_manager
        .register_transform_event_driven(single_registration)
        .expect("Failed to register single schema transform");

    // Test Range schema type
    let range_schema = DeclarativeSchemaDefinition {
        name: "range_type".to_string(),
        schema_type: SchemaType::Range {
            range_key: "timestamp".to_string(),
        },
        fields: HashMap::from([
            (
                "timestamp".to_string(),
                FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some("range_input.map().timestamp".to_string()),
                },
            ),
            (
                "range_output".to_string(),
                FieldDefinition {
                    field_type: Some("single".to_string()),
                    atom_uuid: Some("range_input.map().value".to_string()),
                },
            ),
        ]),
        key: None,
    };

    let range_transform = Transform::from_declarative_schema(
        range_schema,
        vec!["range_input".to_string()],
        "range_type.range_output".to_string(),
    );

    let range_registration = TransformRegistration {
        transform_id: "range_test".to_string(),
        transform: range_transform,
        input_molecules: vec!["range_input".to_string()],
        input_names: vec!["range_input".to_string()],
        trigger_fields: vec!["range_input".to_string()],
        output_molecule: "range_type.range_output".to_string(),
        schema_name: "range_type".to_string(),
        field_name: "range_output".to_string(),
    };

    fixture
        .transform_manager
        .register_transform_event_driven(range_registration)
        .expect("Failed to register range schema transform");

    // Verify both transforms are registered
    let transforms = fixture
        .transform_manager
        .list_transforms()
        .expect("Failed to list transforms");

    assert!(transforms.contains_key(&"single_test".to_string()));
    assert!(transforms.contains_key(&"range_test".to_string()));

    // Verify schema types are preserved
    let single_retrieved = transforms["single_test"]
        .get_declarative_schema()
        .expect("Should have declarative schema");
    assert!(matches!(single_retrieved.schema_type, SchemaType::Single));

    let range_retrieved = transforms["range_test"]
        .get_declarative_schema()
        .expect("Should have declarative schema");
    assert!(matches!(
        range_retrieved.schema_type,
        SchemaType::Range { .. }
    ));
}
